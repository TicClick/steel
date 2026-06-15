use futures::{SinkExt, StreamExt};
use rosu_v2::model::chat::{ChannelMessageType, ChannelType};

use steel_core::{
    chat::{ChatType, ConnectionStatus, MessageType},
    ipc::server::{AppMessageIn, ConnectionDetails},
};
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::{
    tungstenite::{Message, Utf8Bytes},
    WebSocketStream,
};

use crate::core::http::{
    api::Client,
    send_progress,
    websocket::{ChatMessageNewData, EventType, GeneralWebsocketEvent},
    APISettings,
};

pub(super) enum SessionEnd {
    Shutdown,
    AuthFailure,
    ConnectionLost,
}

enum TextMessageOutcome {
    Ready,
    Logout,
    Nothing,
}

async fn fetch_initial_data(
    tx: &UnboundedSender<AppMessageIn>,
    client: &Client,
) -> (Option<String>, Option<u32>) {
    let mut own_username = None;
    let mut own_user_id = None;

    match client.own_data().await {
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

    match client.chat_channels().await {
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

pub(super) async fn run_session(
    mut ws: WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tx: &UnboundedSender<AppMessageIn>,
    settings: &APISettings,
    client: &Client,
    refresh_attempted: &mut bool,
) -> SessionEnd {
    let mut shutdown_interval = tokio::time::interval(std::time::Duration::from_millis(100));

    loop {
        if client.is_shutdown_requested() {
            log::info!("WS: Shutdown requested, closing connection gracefully");
            if let Err(e) = ws.send(Message::Close(None)).await {
                log::error!("Failed to send close frame: {e}");
            }
            return SessionEnd::Shutdown;
        }

        tokio::select! {
            _ = shutdown_interval.tick() => {
                continue;
            }

            msg_result = ws.next() => {
                let Some(msg) = msg_result else {
                    log::info!("WS: Stream ended");
                    return SessionEnd::ConnectionLost;
                };

                let utcnow = chrono::Utc::now();

                tx.send(AppMessageIn::connection_activity())
                    .unwrap_or_else(|e| log::error!("Failed to send activity: {e}"));

                match msg {
                    Err(e) => {
                        let error_msg = e.to_string();
                        log::error!("WS: received error from the server, leaving: {error_msg}");

                        return match super::is_auth_error(&error_msg) {
                            true => SessionEnd::AuthFailure,
                            false => SessionEnd::ConnectionLost,
                        };
                    }

                    Ok(msg) => match msg {
                        Message::Text(t) => {
                            match handle_text_message(t, tx, client).await {
                                TextMessageOutcome::Ready => {
                                    *refresh_attempted = false;

                                    send_progress(tx, "authenticated, fetching account information");
                                    let (username, user_id) = fetch_initial_data(tx, client).await;

                                    if let (Some(ref username), Some(user_id)) = (username, user_id) {
                                        client.set_own_user(username.clone(), user_id);
                                    }

                                    // Only send this after fetching information about oneself to avoid cache misses
                                    // and errors caused by queued join-on-connect requests.
                                    tx.send(AppMessageIn::connection_changed(
                                        ConnectionStatus::Connected,
                                    ))
                                        .unwrap_or_else(|e| log::error!("Failed to send connection status: {e}"));

                                    tx.send(AppMessageIn::connection_details_changed(
                                        ConnectionDetails::API {
                                            server: settings.ws_base_uri.clone(),
                                            token_expires_at: client.get_token_expires_at(),
                                            refresh_token_expires_at: client.get_refresh_token_expires_at(),
                                        },
                                    ))
                                        .unwrap_or_else(|e| log::error!("Failed to send connection details: {e}"));

                                    if let Err(e) = client.chat_keepalive().await {
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

                                TextMessageOutcome::Logout => {
                                    return SessionEnd::AuthFailure;
                                }

                                TextMessageOutcome::Nothing => {}
                            }
                        }

                        Message::Ping(_) => {
                            if let Err(e) = client.chat_keepalive().await {
                                log::error!("Failed to send keepalive in response to ping: {e}")
                            }
                        }

                        Message::Pong(_) => {}

                        Message::Close(frame) => {
                            log::info!("WS: Server closed connection: {frame:?}");
                            return SessionEnd::ConnectionLost;
                        }

                        _ => log::debug!("[{utcnow}] WS: unknown message {msg:?}"),
                    },
                }
            }
        }
    }
}

async fn handle_text_message(
    t: Utf8Bytes,
    tx: &UnboundedSender<AppMessageIn>,
    client: &Client,
) -> TextMessageOutcome {
    log::debug!("WS message arrived: {}", t.as_str());
    let evt: GeneralWebsocketEvent = match serde_json::from_str(t.as_str()) {
        Ok(e) => e,
        Err(e) => {
            log::error!("Failed to parse websocket event: {e}");
            log::error!("Unparsed event: {}", t.as_str());
            return TextMessageOutcome::Nothing;
        }
    };

    if let Some(error_msg) = evt.error {
        log::error!("WebSocket error: {error_msg}");
        if !error_msg.contains("authentication failed") {
            tx.send(AppMessageIn::ui_show_error(
                Box::new(std::io::Error::other(error_msg)),
                false,
            ))
            .unwrap_or_else(|e| log::error!("Failed to send error: {e}"));
        }
        return TextMessageOutcome::Nothing;
    }

    let mut outcome = TextMessageOutcome::Nothing;
    match evt.event {
        Some(EventType::ChatMessageNew) => {
            if let Some(data) = evt.data {
                match serde_json::from_value::<ChatMessageNewData>(data) {
                    Ok(chat_update) => {
                        for user in chat_update.users.iter() {
                            client.insert_user(user);

                            let is_privileged =
                                matches!(user.default_group.as_str(), "gmt" | "nat" | "dev");

                            if is_privileged {
                                tx.send(AppMessageIn::moderator_added(
                                    user.username.clone().into_string(),
                                ))
                                .unwrap_or_else(|e| log::error!("Failed to send moderator: {e}"));
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
            outcome = TextMessageOutcome::Ready;
        }

        Some(EventType::ChatChannelJoin) => log::info!("Received join event from server"),
        Some(EventType::ChatChannelPart) => log::info!("Received part event from server"),

        Some(EventType::Logout) => {
            log::warn!("Received logout event from server - authentication failure");
            outcome = TextMessageOutcome::Logout;
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

    outcome
}
