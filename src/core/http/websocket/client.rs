use futures::{SinkExt, StreamExt};
use rosu_v2::{
    model::chat::{ChannelMessageType, ChatChannel},
    prelude::Scopes,
};
use steel_core::{
    chat::{ConnectionStatus, MessageType},
    ipc::server::AppMessageIn,
};
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{client::IntoClientRequest, protocol::WebSocketConfig, Message, Utf8Bytes},
    WebSocketStream,
};
use ureq::http::HeaderValue;

use crate::core::http::{
    websocket::{ChatMessageNewData, EventType, GeneralWebsocketEvent},
    APISettings,
};

async fn send_keepalive(api: &rosu_v2::Osu, tx: &UnboundedSender<AppMessageIn>) {
    match api.chat_keepalive().await {
        Ok(_) => {
            log::debug!("Keepalive sent successfully");
            tx.send(AppMessageIn::ConnectionActivity)
                .unwrap_or_else(|e| log::error!("Failed to send activity: {}", e));
        }
        Err(e) => log::error!("Failed to send keepalive: {}", e),
    }
}

pub async fn websocket_thread_main(tx: UnboundedSender<AppMessageIn>, settings: APISettings) {
    let default_chat_scopes = Scopes::Public
        | Scopes::Identify
        | Scopes::ChatRead
        | Scopes::ChatWrite
        | Scopes::ChatWriteManage;

    let api = rosu_v2::OsuBuilder::new()
        .with_local_authorization(settings.redirect_uri, default_chat_scopes)
        .client_id(settings.client_id)
        .client_secret(settings.client_secret)
        .build()
        .await
        .unwrap();

    let token = api.token();
    if let Some(token) = token.access() {
        let mut request = settings.ws_base_uri.into_client_request().unwrap();

        let token_header = HeaderValue::from_str(token).unwrap();
        request.headers_mut().insert("Authorization", token_header);

        log::debug!("WS handshake request: {:?}", request);

        let ws_config = WebSocketConfig::default();
        let (mut ws, resp) = connect_async_with_config(request, Some(ws_config), true)
            .await
            .unwrap();

        log::debug!("WS: Connected with: {:?}", resp);

        while let Some(msg) = ws.next().await {
            let utcnow = chrono::Utc::now();

            match msg {
                Err(e) => {
                    log::error!("WS: received error from the server, leaving: {}", e);
                    tx.send(AppMessageIn::ConnectionChanged(
                        ConnectionStatus::Disconnected { by_user: false },
                    ))
                    .unwrap();
                }

                Ok(msg) => match msg {
                    Message::Text(t) => {
                        let is_ready = handle_text_message(t, &tx, &mut ws);
                        if is_ready {
                            send_keepalive(&api, &tx).await;
                            if let Err(e) = ws
                                .send(Message::Text("{\"event\": \"chat.start\"}".into()))
                                .await
                            {
                                log::error!("Failed to send chat.start: {}", e);
                            } else {
                                log::info!("Sent chat.start message");
                            }
                        }
                    }
                    Message::Ping(_payload) => {
                        send_keepalive(&api, &tx).await;
                    }
                    Message::Pong(_) => {
                        log::debug!("WS: Received pong from server");
                    }
                    Message::Close(frame) => {
                        log::info!("WS: Server closed connection: {:?}", frame);
                        tx.send(AppMessageIn::ConnectionChanged(
                            ConnectionStatus::Disconnected { by_user: false },
                        ))
                        .unwrap_or_else(|e| log::error!("Failed to send disconnect: {}", e));
                        break;
                    }
                    _ => log::debug!("[{}] WS: unknown message {:?}", utcnow, msg),
                },
            }
        }
    }
}

fn handle_text_message<S>(
    t: Utf8Bytes,
    tx: &UnboundedSender<AppMessageIn>,
    _ws: &mut WebSocketStream<S>,
) -> bool {
    let evt: GeneralWebsocketEvent = match serde_json::from_str(t.as_str()) {
        Ok(e) => e,
        Err(e) => {
            log::error!("Failed to parse websocket event: {}", e);
            log::error!("Unparsed event: {}", t.as_str());
            return false;
        }
    };

    if let Some(error_msg) = evt.error {
        log::error!("WebSocket error: {}", error_msg);
        tx.send(AppMessageIn::UIShowError {
            error: Box::new(std::io::Error::new(std::io::ErrorKind::Other, error_msg)),
            is_fatal: false,
        })
        .unwrap_or_else(|e| log::error!("Failed to send error: {}", e));
        return false;
    }

    let mut is_ready = false;
    match evt.event {
        Some(EventType::ChatMessageNew) => {
            if let Some(data) = evt.data {
                match serde_json::from_value::<ChatMessageNewData>(data) {
                    Ok(chat_update) => {
                        for message in chat_update.messages {
                            let target = format!("#{}", message.channel_id);
                            let username = chat_update
                                .users
                                .iter().find(|u| u.user_id == message.sender_id)
                                .map(|u| u.username.clone().into_string())
                                .unwrap_or(format!("(id={}", message.sender_id));

                            let timestamp = chrono::DateTime::from_timestamp(
                                message.timestamp.unix_timestamp(),
                                message.timestamp.nanosecond(),
                            )
                            .unwrap_or_else(chrono::Utc::now)
                            .with_timezone(&chrono::Local);

                            let message_type = if message.is_action || matches!(message.message_type, ChannelMessageType::Action) {
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

                            tx.send(AppMessageIn::ChatMessageReceived {
                                target,
                                message: msg,
                            })
                            .unwrap_or_else(|e| log::error!("Failed to send message: {}", e));
                        }
                    }
                    Err(e) => log::error!("Failed to parse chat message: {}", e),
                }
            }
        }

        Some(EventType::ConnectionReady) => {
            log::info!("WebSocket connection ready");
            tx.send(AppMessageIn::ConnectionChanged(ConnectionStatus::Connected))
                .unwrap_or_else(|e| log::error!("Failed to send connection status: {}", e));
            is_ready = true;
        }

        Some(EventType::ChatChannelJoin) => {
            if let Some(data) = evt.data {
                match serde_json::from_value::<ChatChannel>(data) {
                    Ok(channel_info) => {
                        let channel_name = format!("#{}", channel_info.name);
                        tx.send(AppMessageIn::ChannelJoined(channel_name))
                            .unwrap_or_else(|e| log::error!("Failed to send channel join: {}", e));
                    }
                    Err(e) => log::error!("Failed to parse channel info: {}", e),
                }
            }
        }

        Some(EventType::ChatChannelPart) => {
            if let Some(data) = evt.data {
                match serde_json::from_value::<ChatChannel>(data) {
                    Ok(channel_info) => {
                        let channel_name = format!("#{}", channel_info.name);
                        let msg = steel_core::chat::Message::new_system(&format!(
                            "Left channel {}",
                            channel_name
                        ));
                        tx.send(AppMessageIn::ChatMessageReceived {
                            target: channel_name,
                            message: msg,
                        })
                        .unwrap_or_else(|e| log::error!("Failed to send part message: {}", e));
                    }
                    Err(e) => log::error!("Failed to parse channel part: {}", e),
                }
            }
        }

        Some(EventType::Logout) => {
            log::info!("Received logout event from server");
            tx.send(AppMessageIn::ConnectionChanged(
                ConnectionStatus::Disconnected { by_user: false },
            ))
            .unwrap_or_else(|e| log::error!("Failed to send logout: {}", e));
        }

        Some(EventType::New) => {
            if let Some(data) = evt.data {
                log::debug!("New notification received: {}", data);
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
