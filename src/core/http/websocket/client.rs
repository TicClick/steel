use futures::{SinkExt, StreamExt};
use rosu_v2::{
    model::chat::{ChannelMessageType, ChannelType},
    prelude::Token,
};

use std::sync::Arc;
use steel_core::{
    chat::{ChatType, ConnectionStatus, MessageType},
    ipc::server::AppMessageIn,
};
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{client::IntoClientRequest, protocol::WebSocketConfig, Message, Utf8Bytes},
    WebSocketStream,
};
use ureq::http::HeaderValue;

use crate::core::{
    error::{SteelApplicationError, SteelApplicationResult},
    http::{
        api::Client,
        token_storage::{self, PersistedTokenState},
        websocket::{ChatMessageNewData, EventType, GeneralWebsocketEvent},
        APISettings,
    },
};

pub struct WebsocketResult {
    pub api: Option<Arc<rosu_v2::Osu>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct JwtClaims {
    #[serde(default)]
    #[allow(dead_code)]
    aud: String,
    #[serde(default)]
    #[allow(dead_code)]
    jti: String,
    iat: f64,
    nbf: f64,
    exp: f64,
    #[serde(default)]
    #[allow(dead_code)]
    sub: String,
    #[serde(default)]
    #[allow(dead_code)]
    scopes: Vec<String>,
}

fn parse_jwt_token(token: &str) -> Option<JwtClaims> {
    let payload_base64 = token.split('.').nth(1)?;

    let payload_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        payload_base64,
    )
    .map_err(|e| log::warn!("Failed to decode JWT payload: {e}"))
    .ok()?;

    let payload_str = std::str::from_utf8(&payload_bytes)
        .map_err(|e| log::warn!("Failed to parse JWT payload as UTF-8: {e}"))
        .ok()?;

    serde_json::from_str(payload_str)
        .map_err(|e| log::warn!("Failed to parse JWT claims: {e}"))
        .ok()
}

async fn try_build_with_stored_token(
    settings: &APISettings,
) -> Option<(Arc<rosu_v2::Osu>, PersistedTokenState)> {
    match token_storage::load_token_state() {
        Ok(token_state) => {
            if token_state.has_valid_token() {
                log::info!("Found valid stored token, attempting to use it");

                let refresh_token = token_state
                    .refresh_token
                    .clone()
                    .map(|s| s.into_boxed_str());

                let token = Token::new(&token_state.access_token, refresh_token);
                let expires_in = if token_state.is_access_token_valid() {
                    let now = chrono::Utc::now();
                    Some(
                        token_state
                            .access_expires_at
                            .signed_duration_since(now)
                            .num_seconds(),
                    )
                } else {
                    None
                };

                match rosu_v2::OsuBuilder::new()
                    .client_id(settings.client_id)
                    .client_secret(settings.client_secret.clone())
                    .with_token(token, expires_in)
                    .build()
                    .await
                {
                    Ok(api) => {
                        log::info!("Successfully built API client with stored token");
                        return Some((Arc::new(api), token_state.clone()));
                    }
                    Err(e) => {
                        log::error!("Failed to build API client with stored token: {e}");
                    }
                }
            } else {
                log::warn!("Stored token is expired");
            }
        }
        Err(e) => {
            log::debug!("No stored token found or failed to load: {e}");
        }
    }

    None
}

async fn connect_with_config(
    request: tokio_tungstenite::tungstenite::http::Request<()>,
    ws_config: WebSocketConfig,
) -> Result<
    (
        WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        tokio_tungstenite::tungstenite::http::Response<Option<Vec<u8>>>,
    ),
    tokio_tungstenite::tungstenite::Error,
> {
    connect_async_with_config(request, Some(ws_config), true).await
}

async fn try_connect_websocket(
    settings: &APISettings,
    token: &str,
    jwt_claims: &JwtClaims,
) -> Result<
    (
        WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        tokio_tungstenite::tungstenite::http::Response<Option<Vec<u8>>>,
    ),
    SteelApplicationError,
> {
    let now = chrono::Utc::now().timestamp() as f64;

    let time_since_issue = now - jwt_claims.iat;
    let time_until_valid = jwt_claims.nbf - jwt_claims.iat;

    if time_since_issue < 0.0 {
        log::warn!(
            "Clock skew detected: token appears to be issued {:.2}s in the future (local clock may be behind server time)",
            -time_since_issue
        );
    }

    if time_since_issue < time_until_valid {
        let wait_duration = time_until_valid - time_since_issue;
        log::info!(
            "Token not yet valid (issued {:.2}s ago, valid in {:.2}s), waiting {:.2}s",
            time_since_issue,
            time_until_valid - time_since_issue,
            wait_duration
        );
        tokio::time::sleep(std::time::Duration::from_secs_f64(wait_duration)).await;
    }

    let base_request = settings.ws_base_uri.clone().into_client_request().unwrap();
    let token_header = HeaderValue::from_str(token).unwrap();

    log::debug!("WS handshake request base: {base_request:?}");

    let ws_config = WebSocketConfig::default();

    const TOKEN_FRESH_DURATION: f64 = 2.0;
    const MAX_FRESH_TOKEN_RETRIES: u32 = 4;
    const RETRY_DELAY_MS: u64 = 150;

    let is_token_fresh = time_since_issue < TOKEN_FRESH_DURATION;

    let num_attempts = if is_token_fresh {
        log::info!(
            "Token is fresh (issued {:.2}s ago), will retry on auth errors up to {} times",
            time_since_issue,
            MAX_FRESH_TOKEN_RETRIES
        );
        MAX_FRESH_TOKEN_RETRIES
    } else {
        log::info!(
            "Token is not fresh (issued {:.2}s ago), single connection attempt",
            time_since_issue
        );
        1
    };

    for attempt in 1..=num_attempts {
        if is_token_fresh {
            let now_attempt = chrono::Utc::now().timestamp() as f64;
            let time_since_issue_now = now_attempt - jwt_claims.iat;
            if time_since_issue_now >= TOKEN_FRESH_DURATION {
                log::warn!(
                    "Token is no longer fresh (issued {:.2}s ago), stopping retries",
                    time_since_issue_now
                );
                break;
            }
        }

        let mut request = base_request.clone();
        request
            .headers_mut()
            .insert("Authorization", token_header.clone());

        log::debug!("WebSocket connection attempt {}/{}", attempt, num_attempts);

        match connect_with_config(request, ws_config).await {
            Ok((ws, resp)) => {
                log::info!("WebSocket connected successfully on attempt {}", attempt);
                return Ok((ws, resp));
            }
            Err(e) => {
                let error_msg = e.to_string();
                log::warn!(
                    "WebSocket connection attempt {} failed: {}",
                    attempt,
                    error_msg
                );

                let is_auth_error = error_msg.contains("authentication failed")
                    || error_msg.contains("Connection reset");

                if is_auth_error {
                    if attempt < num_attempts {
                        log::info!("Auth error detected, retrying in {}ms...", RETRY_DELAY_MS);
                        tokio::time::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS)).await;
                        continue;
                    } else {
                        log::error!("All retry attempts exhausted for token");
                        if !is_token_fresh {
                            log::error!("Authentication failed with old token - need to re-login");
                        }
                        return Err(SteelApplicationError::InvalidOAuth);
                    }
                } else {
                    log::error!("Non-auth error during connection: {}", error_msg);
                    return Err(SteelApplicationError::InvalidOAuth);
                }
            }
        }
    }

    log::error!("Failed to connect after {} attempts", num_attempts);
    Err(SteelApplicationError::InvalidOAuth)
}

async fn fetch_initial_data(
    api: &rosu_v2::Osu,
    tx: &UnboundedSender<AppMessageIn>,
    client: &Client,
) -> (Option<String>, Option<u32>) {
    let mut own_username = None;
    let mut own_user_id = None;

    match api.own_data().await {
        Ok(user) => {
            log::info!("Logged in as: {} (ID: {})", user.username, user.user_id);
            own_username = Some(user.username.clone().into_string());
            own_user_id = Some(user.user_id);

            tx.send(AppMessageIn::own_username_detected(
                user.username.to_string(),
            ))
            .unwrap_or_else(|e| log::error!("Failed to send own username: {e}"));
        }
        Err(e) => {
            log::error!("Failed to fetch own user data: {e}");
        }
    }

    match api.chat_channels().await {
        Ok(channels) => {
            log::info!("Fetched {} existing channels", channels.len());
            for channel in channels.iter() {
                client.insert_channel(channel);
            }

            for channel in channels {
                let channel_type = match channel.channel_type {
                    ChannelType::Private => ChatType::Person,
                    ChannelType::Public => ChatType::Channel,
                    _ => {
                        log::error!(
                            "Unrecognized channel type: {:?} (fetch_initial_data)",
                            channel
                        );
                        ChatType::Channel
                    }
                };
                tx.send(AppMessageIn::channel_joined(channel.name, channel_type))
                    .unwrap_or_else(|e| log::error!("Failed to send channel join: {e}"));
            }
        }
        Err(e) => {
            log::error!("Failed to fetch existing channels: {e}");
        }
    }

    (own_username, own_user_id)
}

pub async fn websocket_thread_main_with_auth_check(
    tx: UnboundedSender<AppMessageIn>,
    settings: APISettings,
    client: Client,
) -> SteelApplicationResult<Arc<rosu_v2::Osu>> {
    websocket_thread_main_impl(tx, settings, client).await
}

async fn websocket_thread_main_impl(
    tx: UnboundedSender<AppMessageIn>,
    settings: APISettings,
    client: Client,
) -> SteelApplicationResult<Arc<rosu_v2::Osu>> {
    let send_disconnect = |tx: &UnboundedSender<AppMessageIn>, by_user: bool, auth_failed: bool| {
        tx.send(AppMessageIn::connection_changed(
            ConnectionStatus::Disconnected {
                by_user,
                auth_failed,
            },
        ))
        .unwrap_or_else(|e| log::error!("Failed to send disconnect: {e}"));
    };

    let (api, token_state) = {
        if let Some(result) = try_build_with_stored_token(&settings).await {
            log::info!("Using stored authentication token");
            result
        } else {
            log::info!("No valid stored token, requesting user authentication");
            return Err(SteelApplicationError::InvalidOAuth);
        }
    };

    if let Some(new_access_token) = api.token().access() {
        if token_state.access_token != new_access_token {
            log::info!("Access token has changed (refreshed), saving new state");
            token_storage::create_and_save_new_state(new_access_token, api.token().refresh())
                .unwrap_or(token_state);
        }
    } else {
        log::warn!("API client does not have an access token after initialization");
        return Err(SteelApplicationError::InvalidOAuth);
    }

    let token = api.token();
    if let Some(token) = token.access() {
        let jwt_claims = match parse_jwt_token(token) {
            Some(claims) => {
                log::debug!(
                    "JWT token parsed successfully: iat={}, nbf={}, exp={}",
                    claims.iat,
                    claims.nbf,
                    claims.exp
                );
                claims
            }
            None => {
                log::error!("Failed to parse JWT token, cannot proceed with connection");
                return Err(SteelApplicationError::InvalidOAuth);
            }
        };

        let (mut ws, resp) = match try_connect_websocket(&settings, token, &jwt_claims).await {
            Ok(result) => result,
            Err(e) => {
                log::error!("Failed to connect to WebSocket: {:?}", e);
                if let Err(clear_err) = token_storage::clear_token_state() {
                    log::error!("Failed to clear token state: {clear_err}");
                }

                send_disconnect(&tx, false, true);
                return Err(e);
            }
        };

        log::debug!("WS: Connected with: {resp:?}");

        let mut shutdown_interval = tokio::time::interval(std::time::Duration::from_millis(100));

        loop {
            if client.is_shutdown_requested() {
                log::info!("WS: Shutdown requested, closing connection gracefully");
                if let Err(e) = ws.send(Message::Close(None)).await {
                    log::error!("Failed to send close frame: {e}");
                }
                send_disconnect(&tx, true, false);
                break;
            }

            tokio::select! {
                _ = shutdown_interval.tick() => {
                    continue;
                }

                msg_result = ws.next() => {
                    let Some(msg) = msg_result else {
                        log::info!("WS: Stream ended");
                        send_disconnect(&tx, false, false);
                        break;
                    };

                    let utcnow = chrono::Utc::now();

                    tx.send(AppMessageIn::connection_activity())
                        .unwrap_or_else(|e| log::error!("Failed to send activity: {e}"));

                    match msg {
                        Err(e) => {
                            let error_msg = e.to_string();
                            log::error!("WS: received error from the server, leaving: {error_msg}");

                            let is_auth_error = error_msg.contains("authentication failed")
                                || error_msg.contains("Connection reset");

                            if is_auth_error {
                                log::warn!("Authentication error detected - clearing invalid token");
                                if let Err(clear_err) = token_storage::clear_token_state() {
                                    log::error!("Failed to clear token state: {clear_err}");
                                }
                            }

                            send_disconnect(&tx, false, is_auth_error);
                            break;
                        }

                        Ok(msg) => match msg {
                            Message::Text(t) => {
                                let is_ready = handle_text_message(t, &tx, &mut ws, &client).await;
                                if is_ready {
                                    let (username, user_id) = fetch_initial_data(&api, &tx, &client).await;

                                    if let (Some(ref username), Some(user_id)) = (username, user_id) {
                                        client.set_own_user(username.clone(), user_id);
                                    }

                                    // Only send this after fetching information about oneself to avoid cache misses
                                    // and errors caused by queued join-on-connect requests.
                                    tx.send(AppMessageIn::connection_changed(
                                        ConnectionStatus::Connected,
                                    ))
                                        .unwrap_or_else(|e| log::error!("Failed to send connection status: {e}"));

                                    if let Err(e) = api.chat_keepalive().await {
                                        log::error!("Failed to send initial keepalive: {e}")
                                    }

                                    let chat_start = serde_json::to_string(&GeneralWebsocketEvent::new(
                                        EventType::ChatStart,
                                    ))
                                    .unwrap();
                                    if let Err(e) = ws.send(Message::Text(chat_start.into())).await {
                                        log::error!("Failed to send chat.start: {e}");
                                    } else {
                                        log::info!("Sent chat.start message");
                                    }
                                }
                            }

                            Message::Ping(_) => {
                                if let Err(e) = api.chat_keepalive().await {
                                    log::error!("Failed to send keepalive in response to ping: {e}")
                                }
                            }

                            Message::Pong(_) => {}

                            Message::Close(frame) => {
                                log::info!("WS: Server closed connection: {frame:?}");
                                send_disconnect(&tx, false, false);
                                break;
                            }

                            _ => log::debug!("[{utcnow}] WS: unknown message {msg:?}"),
                        },
                    }
                }
            }
        }
    }

    Ok(api.clone())
}

async fn handle_text_message<S>(
    t: Utf8Bytes,
    tx: &UnboundedSender<AppMessageIn>,
    _ws: &mut WebSocketStream<S>,
    client: &Client,
) -> bool {
    log::debug!("WS message arrived: {}", t.as_str());
    let evt: GeneralWebsocketEvent = match serde_json::from_str(t.as_str()) {
        Ok(e) => e,
        Err(e) => {
            log::error!("Failed to parse websocket event: {e}");
            log::error!("Unparsed event: {}", t.as_str());
            return false;
        }
    };

    if let Some(error_msg) = evt.error {
        log::error!("WebSocket error: {error_msg}");
        tx.send(AppMessageIn::ui_show_error(
            Box::new(std::io::Error::other(error_msg)),
            false,
        ))
        .unwrap_or_else(|e| log::error!("Failed to send error: {e}"));
        return false;
    }

    let mut is_ready = false;
    match evt.event {
        Some(EventType::ChatMessageNew) => {
            if let Some(data) = evt.data {
                match serde_json::from_value::<ChatMessageNewData>(data) {
                    Ok(chat_update) => {
                        for user in chat_update.users.iter() {
                            client.insert_user(user);

                            let is_privileged = matches!(
                                user.default_group.as_str(),
                                "gmt" | "nat" | "dev"
                            );

                            if is_privileged {
                                tx.send(AppMessageIn::moderator_added(
                                    user.username.clone().into_string(),
                                ))
                                .unwrap_or_else(|e| {
                                    log::error!("Failed to send moderator: {e}")
                                });
                            }
                        }

                        for message in chat_update.messages {
                            let username = chat_update
                                .users
                                .iter()
                                .find(|u| u.user_id == message.sender_id)
                                .map(|u| u.username.clone().into_string())
                                .unwrap_or(format!("(id={})", message.sender_id));

                            let target = match client.get_or_fetch_channel(message.channel_id).await
                            {
                                Ok(channel) => channel.name,
                                Err(e) => {
                                    log::error!(
                                        "Failed to read information about a channel #{}: {}",
                                        message.channel_id,
                                        e
                                    );
                                    format!("(id={})", message.channel_id)
                                }
                            };

                            let timestamp = chrono::DateTime::from_timestamp(
                                message.timestamp.unix_timestamp(),
                                message.timestamp.nanosecond(),
                            )
                            .unwrap_or_else(chrono::Utc::now)
                            .with_timezone(&chrono::Local);

                            let message_type = match message.is_action
                                || matches!(message.message_type, ChannelMessageType::Action)
                            {
                                true => MessageType::Action,
                                false => MessageType::Text,
                            };

                            let msg = steel_core::chat::Message::new(
                                &username,
                                &message.content,
                                message_type,
                            )
                            .with_time(timestamp);

                            tx.send(AppMessageIn::chat_message_received(target, msg))
                                .unwrap_or_else(|e| log::error!("Failed to send message: {e}"));
                        }
                    }
                    Err(e) => log::error!("Failed to parse chat message: {e}"),
                }
            }
        }

        Some(EventType::ConnectionReady) => {
            log::info!("WebSocket connection ready");
            is_ready = true;
        }

        Some(EventType::ChatChannelJoin) => log::info!("Received join event from server"),
        Some(EventType::ChatChannelPart) => log::info!("Received part event from server"),

        Some(EventType::Logout) => {
            log::warn!(
                "Received logout event from server - authentication failure, clearing token"
            );

            if let Err(clear_err) = token_storage::clear_token_state() {
                log::error!("Failed to clear token state: {clear_err}");
            }

            tx.send(AppMessageIn::connection_changed(
                ConnectionStatus::Disconnected {
                    by_user: false,
                    auth_failed: true,
                },
            ))
            .unwrap_or_else(|e| log::error!("Failed to send logout: {e}"));
        }

        Some(EventType::New) => {
            if let Some(data) = evt.data {
                log::debug!("New notification received: {data}");
            }
        }

        Some(EventType::Read) => {
            // Read notifications - no action needed.
        }

        Some(EventType::ChatStart) | Some(EventType::ChatStop) => {
            // Client-side events, should not be received from server.
            log::warn!("Received client-side event from server: {:?}", evt.event);
        }

        None => {
            // No event type - this might be just an error response.
            log::warn!("Received message without event type");
        }
    }

    is_ready
}
