use rosu_v2::{
    model::chat::{ChannelType, ChatChannel},
    request::UserId,
};
use std::sync::{Arc, Mutex};
use steel_core::{
    chat::{ChatType, ConnectionStatus, MessageType},
    ipc::server::AppMessageIn,
};
use async_trait::async_trait;
use tokio::{
    runtime::Runtime,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
};

use crate::{
    actor::Actor,
    core::http::{
        state::HTTPState, websocket::client::websocket_thread_main_with_auth_check, APISettings,
        HTTPMessageIn,
    },
};

pub struct HTTPActor {
    input: UnboundedReceiver<HTTPMessageIn>,
    output: UnboundedSender<AppMessageIn>,

    state: Arc<Mutex<HTTPState>>,
}

#[async_trait]
impl Actor<HTTPMessageIn, AppMessageIn> for HTTPActor {
    fn new(input: UnboundedReceiver<HTTPMessageIn>, output: UnboundedSender<AppMessageIn>) -> Self {
        Self {
            input,
            output,
            state: Arc::new(Mutex::new(HTTPState::new())),
        }
    }

    async fn run(&mut self) {
        while let Some(msg) = self.input.recv().await {
            log::debug!("Handling UI message: {msg:?}");
            self.handle_message(msg).await;
        }
    }

    async fn handle_message(&mut self, message: HTTPMessageIn) {
        match message {
            HTTPMessageIn::Connect(settings) => self.connect(settings),
            HTTPMessageIn::Disconnect => self.disconnect(),
            HTTPMessageIn::JoinChannel(channel) => {
                self.join_channel(channel).await;
            }
            HTTPMessageIn::LeaveChannel(channel) => {
                self.leave_channel(channel).await;
            }
            HTTPMessageIn::SendMessage {
                r#type,
                destination,
                chat_type,
                content,
            } => {
                self.send_message(r#type, destination, chat_type, content).await;
            }
        }
    }
}

impl HTTPActor {
    fn push_error_to_ui(&mut self, error: &str, is_fatal: bool) {
        let e = AppMessageIn::ui_show_error(
            Box::new(std::io::Error::other(format!("HTTP chat error: {error}"))),
            is_fatal,
        );

        self.output
            .send(e)
            .unwrap_or_else(|e| log::error!("Failed to send error: {e}"));
    }

    fn get_api_and_channel(
        &mut self,
        channel_name: &str,
        operation: &str,
    ) -> Option<(Arc<rosu_v2::Osu>, ChatChannel)> {
        let (api, channel) = {
            let state = self.state.lock().unwrap();
            (
                state.api.clone(),
                state.cache.find_channel(channel_name).cloned(),
            )
        };

        let channel = match channel {
            Some(c) => c,
            None => {
                log::error!("Cannot {operation} in {channel_name}: channel not found");
                self.push_error_to_ui(
                    &format!("Cannot {operation} in {channel_name}: channel not found"),
                    false,
                );
                return None;
            }
        };

        let api = match api {
            Some(a) => a,
            None => {
                log::error!("Cannot {operation}: not connected");
                self.push_error_to_ui(&format!("Cannot {operation}: not connected"), false);
                return None;
            }
        };

        Some((api, channel))
    }

    fn get_api(&mut self, operation: &str) -> Option<Arc<rosu_v2::Osu>> {
        let api = {
            let state = self.state.lock().unwrap();
            state.api.clone()
        };

        match api {
            Some(a) => Some(a),
            None => {
                log::error!("Cannot {operation}: not connected to the API");
                self.push_error_to_ui(&format!("Cannot {operation}: not connected to the API"), false);
                None
            }
        }
    }

    fn connect(&mut self, settings: APISettings) {
        self.output
            .send(AppMessageIn::connection_changed(
                ConnectionStatus::InProgress,
            ))
            .unwrap_or_else(|e| log::error!("Failed to send InProgress connection status: {e}"));

        let tx = self.output.clone();
        let state = self.state.clone();

        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(websocket_thread_main_with_auth_check(
                    tx.clone(),
                    settings,
                    state,
                ))
        });
    }

    fn disconnect(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.request_shutdown();
        }
    }

    async fn send_message(
        &mut self,
        r#type: MessageType,
        destination: String,
        chat_type: ChatType,
        content: String,
    ) {
        match chat_type {
            ChatType::Channel => {
                let Some((api, channel)) = self.get_api_and_channel(&destination, "send message")
                else {
                    return;
                };

                let tx = self.output.clone();
                tokio::spawn(async move {
                    let is_action = matches!(r#type, MessageType::Action);
                    let result = api
                        .chat_send_message(channel.channel_id, content, is_action)
                        .await;

                    if let Err(e) = result {
                        log::error!("Failed to send message to channel {}: {e}", channel.name);
                        tx.send(AppMessageIn::ui_show_error(
                            Box::new(std::io::Error::other(format!(
                                "Failed to send message to channel {}: {e}",
                                channel.name
                            ))),
                            false,
                        ))
                        .unwrap_or_else(|e| log::error!("Failed to send error: {e}"));
                    }
                });
            }
            ChatType::Person => {
                let cached_uid = {
                    let state = self.state.lock().unwrap();
                    state.cache.get_user_by_username(&destination)
                };

                let Some(api) = self.get_api("send message") else {
                    return;
                };

                let state = self.state.clone();
                let tx = self.output.clone();
                let destination_for_error = destination.clone();

                tokio::spawn(async move {
                    let uid = match cached_uid {
                        Some(uid) => uid,
                        None => {
                            match api
                                .user(UserId::Name(format!("@{destination}").into()))
                                .await
                            {
                                Ok(u) => {
                                    if let Ok(mut state_guard) = state.lock() {
                                        state_guard.cache.insert_user(u.clone().into());
                                    }
                                    u.user_id
                                }
                                Err(e) => {
                                    log::error!(
                                        "Failed to send a message to user {destination_for_error}: failed to look up in API: {e}"
                                    );
                                    tx.send(AppMessageIn::ui_show_error(
                                        Box::new(std::io::Error::other(format!(
                                            "Cannot send message to user: lookup failed for {destination_for_error}"
                                        ))),
                                        false,
                                    ))
                                    .unwrap_or_else(|e| log::error!("Failed to send error: {e}"));
                                    return;
                                }
                            }
                        }
                    };

                    let is_action = matches!(r#type, MessageType::Action);
                    let result = api
                        .chat_create_private_channel(uid, content, is_action)
                        .await;

                    if let Err(e) = result {
                        log::error!("Failed to send private message to user {}: {e}", uid);
                        tx.send(AppMessageIn::ui_show_error(
                            Box::new(std::io::Error::other(format!(
                                "Failed to send private message to user {}: {e}",
                                uid
                            ))),
                            false,
                        ))
                        .unwrap_or_else(|e| log::error!("Failed to send error: {e}"));
                    }
                });
            }
            _ => {
                self.push_error_to_ui(
                    &format!("Failed to send message to an unknown channel type {chat_type}"),
                    false,
                );
            }
        }
    }

    async fn join_channel(&mut self, channel_name: String) {
        let own_user_id = {
            let state = self.state.lock().unwrap();
            state.own_user_id
        };

        let Some((api, channel)) = self.get_api_and_channel(&channel_name, "join channel") else {
            return;
        };

        let tx = self.output.clone();
        tokio::spawn(async move {
            let user_id = if let Some(uid) = own_user_id {
                uid
            } else {
                match api.own_data().await {
                    Ok(user) => user.user_id,
                    Err(e) => {
                        log::error!("Failed to get own user data: {e}");
                        return;
                    }
                }
            };

            match api.chat_join_channel(channel.channel_id, user_id).await {
                Ok(channel_info) => {
                    log::debug!("Joined channel: {}", channel_info.name);
                    let channel_type = match channel.channel_type {
                        ChannelType::Private => ChatType::Person,
                        ChannelType::Public => ChatType::Channel,
                        _ => {
                            log::error!("Unrecognized channel type: {:?} (join_channel)", channel);
                            ChatType::Channel
                        }
                    };

                    tx.send(AppMessageIn::channel_joined(
                        channel_info.name,
                        channel_type,
                    ))
                    .unwrap_or_else(|e| log::error!("Failed to send channel join: {e}"));
                }
                Err(e) => {
                    log::error!("Failed to join channel: {e}");
                    tx.send(AppMessageIn::ui_show_error(
                        Box::new(std::io::Error::other(format!(
                            "Failed to join channel: {e}"
                        ))),
                        false,
                    ))
                    .unwrap_or_else(|e| log::error!("Failed to send error: {e}"));
                }
            }
        });
    }

    async fn leave_channel(&mut self, channel_name: String) {
        let own_user_id = {
            let state = self.state.lock().unwrap();
            state.own_user_id
        };

        let Some((api, channel)) = self.get_api_and_channel(&channel_name, "leave channel") else {
            return;
        };

        let tx = self.output.clone();
        tokio::spawn(async move {
            let user_id = if let Some(uid) = own_user_id {
                uid
            } else {
                match api.own_data().await {
                    Ok(user) => user.user_id,
                    Err(e) => {
                        log::error!("Failed to get own user data: {e}");
                        return;
                    }
                }
            };

            if let Err(e) = api.chat_leave_channel(channel.channel_id, user_id).await {
                log::error!("Failed to leave channel: {e}");
                tx.send(AppMessageIn::ui_show_error(
                    Box::new(std::io::Error::other(format!(
                        "Failed to leave channel: {e}"
                    ))),
                    false,
                ))
                .unwrap_or_else(|e| log::error!("Failed to send error: {e}"));
            }
        });
    }
}
