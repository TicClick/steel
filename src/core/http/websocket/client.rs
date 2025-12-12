use futures::{SinkExt, StreamExt};
use rosu_v2::{
    model::chat::{ChannelMessageType, ChannelType},
    prelude::{Scopes, Token},
};

use std::sync::{Arc, Mutex};
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
        state::HTTPState,
        token_storage::{self, PersistedTokenState},
        websocket::{ChatMessageNewData, EventType, GeneralWebsocketEvent},
        APISettings,
    },
};

pub struct WebsocketResult {
    pub api: Option<Arc<rosu_v2::Osu>>,
}

async fn try_build_with_stored_token(
    settings: &APISettings,
    _default_chat_scopes: Scopes,
) -> Option<(Arc<rosu_v2::Osu>, PersistedTokenState)> {
    match token_storage::load_token_state() {
        Ok(token_state) => {
            if token_state.has_valid_token() {
                log::info!("Found valid stored token, attempting to use it");

                let access_token = token_state
                    .access_token
                    .strip_prefix("Bearer ")
                    .unwrap_or(&token_state.access_token);

                let refresh_token = token_state
                    .refresh_token
                    .clone()
                    .map(|s| s.into_boxed_str());

                let token = Token::new(access_token, refresh_token);
                let now = chrono::Utc::now();
                let expires_in = token_state
                    .access_expires_at
                    .signed_duration_since(now)
                    .num_seconds();

                match rosu_v2::OsuBuilder::new()
                    .client_id(settings.client_id)
                    .client_secret(settings.client_secret.clone())
                    .with_token(token, Some(expires_in))
                    .build()
                    .await
                {
                    Ok(api) => {
                        log::info!("Successfully built API client with stored token");
                        return Some((Arc::new(api), token_state));
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

async fn build_with_fresh_oauth(
    settings: &APISettings,
    default_chat_scopes: Scopes,
) -> Result<(Arc<rosu_v2::Osu>, PersistedTokenState), Box<dyn std::error::Error>> {
    log::info!("Starting fresh OAuth flow");

    let api = Arc::new(
        rosu_v2::OsuBuilder::new()
            .with_local_authorization(settings.redirect_uri.clone(), default_chat_scopes)
            .client_id(settings.client_id)
            .client_secret(settings.client_secret.clone())
            .build()
            .await?,
    );

    let token_lifetime_secs = 24 * 60 * 60;
    let access_token = api
        .token()
        .access()
        .ok_or("No access token available")?
        .to_string();

    let refresh_token = api.token().refresh().map(|s| s.to_string());

    let token_state = PersistedTokenState::new(access_token, refresh_token, token_lifetime_secs);

    if let Err(e) = token_storage::save_token_state(&token_state) {
        log::error!("Failed to save token state: {e}");
    }

    Ok((api, token_state))
}

async fn fetch_initial_data(
    api: &rosu_v2::Osu,
    tx: &UnboundedSender<AppMessageIn>,
    state: &Arc<Mutex<HTTPState>>,
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

            if let Ok(state_guard) = state.lock() {
                state_guard.cache.insert_channels(channels.clone());
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
    state: Arc<Mutex<HTTPState>>,
) -> SteelApplicationResult<Arc<rosu_v2::Osu>> {
    websocket_thread_main_impl(tx, settings, state).await
}

async fn websocket_thread_main_impl(
    tx: UnboundedSender<AppMessageIn>,
    settings: APISettings,
    state: Arc<Mutex<HTTPState>>,
) -> SteelApplicationResult<Arc<rosu_v2::Osu>> {
    let default_chat_scopes = Scopes::Public
        | Scopes::Identify
        | Scopes::ChatRead
        | Scopes::ChatWrite
        | Scopes::ChatWriteManage;

    let (api, token_state) = {
        if let Some(result) = try_build_with_stored_token(&settings, default_chat_scopes).await {
            log::info!("Using stored authentication token");
            result
        } else {
            log::info!("No valid stored token, requesting user authentication");
            tx.send(AppMessageIn::http_auth_required())
                .unwrap_or_else(|e| log::error!("Failed to send auth required: {e}"));

            match build_with_fresh_oauth(&settings, default_chat_scopes).await {
                Ok(result) => {
                    tx.send(AppMessageIn::http_auth_success())
                        .unwrap_or_else(|e| log::error!("Failed to send auth success: {e}"));
                    result
                }
                Err(e) => {
                    log::error!("OAuth authentication failed: {e}");
                    return Err(SteelApplicationError::InvalidOAuth);
                }
            }
        }
    };

    if let Ok(mut state_guard) = state.lock() {
        state_guard.set_token_expiry(&token_state);
        state_guard.set_api_client(api.clone());
    }

    let token = api.token();
    if let Some(token) = token.access() {
        let mut request = settings.ws_base_uri.into_client_request().unwrap();

        let token_header = HeaderValue::from_str(token).unwrap();
        request.headers_mut().insert("Authorization", token_header);

        log::debug!("WS handshake request: {request:?}");

        let ws_config = WebSocketConfig::default();

        // Avoid getting the authentication failed error from the websocket -- apparently it takes time for the token
        // to get into the Redis cache used by osu-notification-server.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let (mut ws, resp) = connect_async_with_config(request, Some(ws_config), true)
            .await
            .unwrap();

        log::debug!("WS: Connected with: {resp:?}");

        let mut shutdown_interval = tokio::time::interval(std::time::Duration::from_millis(100));

        loop {
            let shutdown_requested = state
                .lock()
                .map(|s| s.is_shutdown_requested())
                .unwrap_or(false);

            if shutdown_requested {
                log::info!("WS: Shutdown requested, closing connection gracefully");
                if let Err(e) = ws.send(Message::Close(None)).await {
                    log::error!("Failed to send close frame: {e}");
                }
                tx.send(AppMessageIn::connection_changed(
                    ConnectionStatus::Disconnected { by_user: true },
                ))
                .unwrap_or_else(|e| log::error!("Failed to send disconnect: {e}"));

                if let Ok(mut state_guard) = state.lock() {
                    state_guard.clear();
                }
                break;
            }

            tokio::select! {
                _ = shutdown_interval.tick() => {
                    continue;
                }

                msg_result = ws.next() => {
                    let Some(msg) = msg_result else {
                        log::info!("WS: Stream ended");
                        tx.send(AppMessageIn::connection_changed(
                            ConnectionStatus::Disconnected { by_user: false },
                        ))
                        .unwrap_or_else(|e| log::error!("Failed to send disconnect: {e}"));
                        break;
                    };

                    let utcnow = chrono::Utc::now();

                    tx.send(AppMessageIn::connection_activity())
                        .unwrap_or_else(|e| log::error!("Failed to send activity: {e}"));

                    match msg {
                        Err(e) => {
                            log::error!("WS: received error from the server, leaving: {e}");
                            tx.send(AppMessageIn::connection_changed(
                                ConnectionStatus::Disconnected { by_user: false },
                            ))
                            .unwrap_or_else(|e| log::error!("Failed to send disconnect: {e}"));
                            break;
                        }

                        Ok(msg) => match msg {
                            Message::Text(t) => {
                                let is_ready = handle_text_message(t, &tx, &mut ws, &state).await;
                                if is_ready {
                                    let (username, user_id) = fetch_initial_data(&api, &tx, &state).await;

                                    if let (Some(ref username), Some(user_id)) = (username, user_id) {
                                        if let Ok(mut state_guard) = state.lock() {
                                            state_guard.set_own_user(username.clone(), user_id);
                                        }
                                    }

                                    // Only send this after fetching information about oneself to avoid cache misses
                                    // and errors caused by queued join-on-connect requests.
                                    tx.send(AppMessageIn::connection_changed(
                                        ConnectionStatus::Connected,
                                    ))
                                        .unwrap_or_else(|e| log::error!("Failed to send connection status: {e}"));

                                    if let Err(e) = api.chat_keepalive().await {
                                        log::error!("Failed to send keepalive: {e}")
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
                                    log::error!("Failed to send keepalive: {e}")
                                }
                            }

                            Message::Pong(_) => {}

                            Message::Close(frame) => {
                                log::info!("WS: Server closed connection: {frame:?}");
                                tx.send(AppMessageIn::connection_changed(
                                    ConnectionStatus::Disconnected { by_user: false },
                                ))
                                .unwrap_or_else(|e| log::error!("Failed to send disconnect: {e}"));
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
    state: &Arc<Mutex<HTTPState>>,
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
                        if let Ok(state_guard) = state.lock() {
                            state_guard.cache.insert_users(chat_update.users.clone());
                        }

                        for message in chat_update.messages {
                            let cache = {
                                if let Ok(state_guard) = state.lock() {
                                    Some(Arc::clone(&state_guard.cache))
                                } else {
                                    None
                                }
                            };

                            let target = if let Some(cache) = cache {
                                match cache.get_or_fetch_channel(message.channel_id).await {
                                    Ok(channel) => channel.name,
                                    Err(e) => {
                                        log::error!(
                                            "Failed to recall chat name by channel_id: {e}"
                                        );
                                        format!("#{}", message.channel_id)
                                    }
                                }
                            } else {
                                format!("#{}", message.channel_id)
                            };

                            let username = chat_update
                                .users
                                .iter()
                                .find(|u| u.user_id == message.sender_id)
                                .map(|u| u.username.clone().into_string())
                                .unwrap_or(format!("(id={})", message.sender_id));

                            let timestamp = chrono::DateTime::from_timestamp(
                                message.timestamp.unix_timestamp(),
                                message.timestamp.nanosecond(),
                            )
                            .unwrap_or_else(chrono::Utc::now)
                            .with_timezone(&chrono::Local);

                            let message_type = if message.is_action
                                || matches!(message.message_type, ChannelMessageType::Action)
                            {
                                MessageType::Action
                            } else {
                                MessageType::Text
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
            log::info!("Received logout event from server");
            tx.send(AppMessageIn::connection_changed(
                ConnectionStatus::Disconnected { by_user: false },
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
