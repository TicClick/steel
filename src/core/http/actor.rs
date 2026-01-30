use rosu_v2::model::chat::{ChannelType, ChatChannel};
use steel_core::{
    chat::{ChatType, ConnectionStatus, MessageType},
    ipc::server::AppMessageIn,
};
use tokio::{
    runtime::Runtime,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
};

use crate::{
    actor::Actor,
    core::http::{
        api::Client, websocket::client::websocket_thread_main_with_auth_check, APISettings,
        HTTPMessageIn,
    },
};

pub struct HTTPActor {
    input: UnboundedReceiver<HTTPMessageIn>,
    output: UnboundedSender<AppMessageIn>,

    client: Option<Client>,

    runtime: Runtime,
}

impl Actor<HTTPMessageIn, AppMessageIn> for HTTPActor {
    fn new(input: UnboundedReceiver<HTTPMessageIn>, output: UnboundedSender<AppMessageIn>) -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime for HTTPActor");
        Self {
            input,
            output,
            client: None,
            runtime,
        }
    }

    fn run(&mut self) {
        while let Some(msg) = self.input.blocking_recv() {
            log::debug!("Handling UI message: {msg:?}");
            self.handle_message(msg);
        }
    }

    fn handle_message(&mut self, message: HTTPMessageIn) {
        match message {
            HTTPMessageIn::Connect(settings) => self.connect(settings),
            HTTPMessageIn::Disconnect => self.disconnect(),
            HTTPMessageIn::JoinChannel(channel) => {
                self.join_channel(channel);
            }
            HTTPMessageIn::LeaveChannel(channel) => {
                self.leave_channel(channel);
            }
            HTTPMessageIn::SendMessage {
                r#type,
                destination,
                chat_type,
                content,
            } => match chat_type {
                ChatType::Channel => {
                    self.channel_send_message(r#type, destination, chat_type, content)
                }
                ChatType::Person => self.user_send_message(r#type, destination, chat_type, content),
                _ => {
                    Self::push_error_to_ui(
                        &self.output,
                        &format!("Failed to send message to an unknown channel type {chat_type}"),
                        false,
                    );
                }
            },
        }
    }
}

impl HTTPActor {
    fn push_error_to_ui(output: &UnboundedSender<AppMessageIn>, error: &str, is_fatal: bool) {
        log::error!("{}", error);
        let e = AppMessageIn::ui_show_error(
            Box::new(std::io::Error::other(format!("HTTP chat error: {error}"))),
            is_fatal,
        );

        output
            .send(e)
            .unwrap_or_else(|e| log::error!("Failed to send error: {e}"));
    }

    fn get_client_and_channel(
        &mut self,
        channel_name: &str,
        operation: &str,
    ) -> Option<(Client, ChatChannel)> {
        if let Some(client) = self.get_client(operation) {
            let channel = match client.get_cached_channel_by_name(channel_name) {
                Some(c) => c,
                None => {
                    Self::push_error_to_ui(
                        &self.output,
                        &format!("Cannot {operation}: channel not found by name"),
                        false,
                    );
                    return None;
                }
            };
            Some((client, channel))
        } else {
            None
        }
    }

    fn get_client(&mut self, operation: &str) -> Option<Client> {
        match &self.client {
            Some(c) => Some(c.clone()),
            None => {
                Self::push_error_to_ui(
                    &self.output,
                    &format!("Cannot {operation}: not connected"),
                    false,
                );
                None
            }
        }
    }

    fn connect(&mut self, settings: APISettings) {
        self.output
            .send(AppMessageIn::connection_changed(
                ConnectionStatus::InProgress,
            ))
            .unwrap_or_else(|e| log::error!("Failed to send connection status: {e}"));

        let tx = self.output.clone();

        let client = match self.runtime.block_on(Client::from_stored_token(
            settings.client_id,
            settings.client_secret.clone(),
        )) {
            Ok(c) => c,
            Err(e) => {
                Self::push_error_to_ui(
                    &self.output,
                    &format!("Failed to create client from stored token: {e}"),
                    true,
                );
                return;
            }
        };

        self.client = Some(client.clone());

        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(websocket_thread_main_with_auth_check(
                    tx.clone(),
                    settings,
                    client,
                ))
        });
    }

    fn disconnect(&mut self) {
        if let Some(client) = &self.client {
            client.shutdown();
        }
    }

    fn channel_send_message(
        &mut self,
        r#type: MessageType,
        destination: String,
        _chat_type: ChatType,
        content: String,
    ) {
        let Some((client, channel)) = self.get_client_and_channel(&destination, "send message")
        else {
            return;
        };

        let tx = self.output.clone();
        self.runtime.block_on(async move {
            let is_action = matches!(r#type, MessageType::Action);
            let result = client
                .chat_send_message(channel.channel_id, content, is_action)
                .await;

            if let Err(e) = result {
                Self::push_error_to_ui(&tx, &format!("Failed to send message: {e}"), false);
            }
        });
    }

    fn user_send_message(
        &mut self,
        r#type: MessageType,
        destination: String,
        _chat_type: ChatType,
        content: String,
    ) {
        let Some(client) = self.get_client("send message") else {
            return;
        };

        let tx = self.output.clone();
        let destination_for_error = destination.clone();
        let destination_username = destination.clone();

        self.runtime.block_on(async move {
            let uid = match client.get_or_fetch_user(&destination).await {
                Ok(user) => user.user_id,
                Err(e) => {
                    Self::push_error_to_ui(
                        &tx,
                        &format!("Failed to send a message to user {destination_for_error}: {e}"),
                        false,
                    );
                    return;
                }
            };

            let is_action = matches!(r#type, MessageType::Action);

            let maybe_channel = client.get_cached_channel_by_name(&destination_username);
            let send_result = match maybe_channel {
                Some(channel) => {
                    client
                        .chat_send_message(channel.channel_id, content, is_action)
                        .await
                }
                None => {
                    match client
                        .chat_create_private_channel(uid, content, is_action)
                        .await
                    {
                        Ok(channel) => {
                            log::debug!(
                                "Created private channel: {} (ID: {})",
                                channel.name,
                                channel.channel_id
                            );
                            Ok(())
                        }
                        Err(e) => {
                            log::error!("Failed to create a private message channel: {:?}", e);
                            Err(e)
                        }
                    }
                }
            };

            if let Err(e) = send_result {
                log::error!("Failed to send message: {e}");
                tx.send(AppMessageIn::ui_show_error(
                    Box::new(std::io::Error::other(format!(
                        "Failed to send message: {e}"
                    ))),
                    false,
                ))
                .unwrap_or_else(|e| log::error!("Failed to send error: {e}"));
            }
        });
    }

    fn join_channel(&mut self, channel_name: String) {
        let Some((client, channel)) = self.get_client_and_channel(&channel_name, "join channel")
        else {
            return;
        };

        let tx = self.output.clone();
        self.runtime.block_on(async move {
            let user_id = if let Some(uid) = client.get_own_user_id() {
                uid
            } else {
                match client.own_data().await {
                    Ok(user) => user.user_id,
                    Err(e) => {
                        log::error!("Failed to get own user data: {e}");
                        return;
                    }
                }
            };

            match client.chat_join_channel(channel.channel_id, user_id).await {
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
                    Self::push_error_to_ui(&tx, &format!("Failed to join channel: {e}"), false);
                }
            }
        });
    }

    fn leave_channel(&mut self, channel_name: String) {
        let Some((client, channel)) = self.get_client_and_channel(&channel_name, "leave channel")
        else {
            return;
        };

        let tx = self.output.clone();
        self.runtime.block_on(async move {
            let user_id = if let Some(uid) = client.get_own_user_id() {
                uid
            } else {
                match client.own_data().await {
                    Ok(user) => user.user_id,
                    Err(e) => {
                        log::error!("Failed to get own user data: {e}");
                        return;
                    }
                }
            };

            if let Err(e) = client.chat_leave_channel(channel.channel_id, user_id).await {
                Self::push_error_to_ui(&tx, &format!("Failed to leave channel: {e}"), false);
            }
        });
    }
}
