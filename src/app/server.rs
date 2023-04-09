use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::core::chat;
use crate::core::chat::{ChatLike, Message};
use crate::core::irc::{ConnectionStatus, IRCActorHandle, IRCError};
use crate::core::logger;
use crate::core::settings;
use crate::gui::UIMessageIn;

use super::{state, AppMessageIn};

const EVENT_QUEUE_SIZE: usize = 1000;

pub struct Application {
    state: state::ApplicationState,
    events: Receiver<AppMessageIn>,

    irc: IRCActorHandle,
    ui_queue: Sender<UIMessageIn>,
    pub app_queue: Sender<AppMessageIn>,
}

impl Application {
    pub fn new(ui_queue: Sender<UIMessageIn>) -> Self {
        let (app_queue, events) = channel(EVENT_QUEUE_SIZE);
        Self {
            state: state::ApplicationState::default(),
            events,
            irc: IRCActorHandle::new(app_queue.clone()),
            ui_queue,
            app_queue,
        }
    }

    pub fn run(&mut self) {
        while let Some(event) = self.events.blocking_recv() {
            match event {
                AppMessageIn::Connect => {
                    self.connect();
                }
                AppMessageIn::Disconnect => {
                    self.disconnect();
                }
                AppMessageIn::ConnectionChanged(status) => {
                    self.handle_connection_status(status);
                }
                AppMessageIn::ChatMessageReceived { target, message } => {
                    self.handle_chat_message(target, message);
                }
                AppMessageIn::ServerMessageReceived { content } => {
                    self.handle_server_message(content);
                }
                AppMessageIn::ChatError(e) => {
                    self.handle_chat_error(e);
                }
                AppMessageIn::ChannelJoined(channel) => {
                    self.handle_channel_join(channel);
                }

                AppMessageIn::UIConnectRequested => {
                    self.connect();
                }
                AppMessageIn::UIDisconnectRequested => {
                    self.disconnect();
                }
                AppMessageIn::UIExitRequested => {
                    break;
                }
                AppMessageIn::UIChannelOpened(channel) => {
                    self.handle_ui_channel_opened(channel);
                }
                AppMessageIn::UIPrivateChatOpened(user) => {
                    self.maybe_remember_chat(&user);
                }
                AppMessageIn::UIChatClosed(target) => {
                    self.ui_handle_close_chat(&target);
                }
                AppMessageIn::UIChatMessageSent { target, text } => {
                    self.send_text_message(&target, &text);
                }
                AppMessageIn::UISettingsRequested => {
                    self.ui_handle_settings_requested();
                }
                AppMessageIn::UISettingsUpdated(settings) => {
                    self.ui_handle_settings_updated(settings);
                }
            }
        }
    }
}

impl Application {
    pub fn handle_ui_channel_opened(&mut self, channel: String) {
        self.join_channel(&channel);
    }

    pub fn initialize(&mut self) {
        self.load_settings(settings::Source::DefaultPath, false);
        if self.state.settings.chat.autoconnect {
            self.connect();
        }
    }

    pub fn load_settings(&mut self, source: settings::Source, fallback: bool) {
        self.state.settings = settings::Settings::from_file(&source, fallback);
        self.ui_handle_settings_requested();
    }

    pub fn ui_handle_settings_requested(&self) {
        self.ui_queue
            .blocking_send(UIMessageIn::SettingsChanged(self.state.settings.clone()))
            .unwrap();
    }

    pub fn ui_handle_settings_updated(&mut self, settings: settings::Settings) {
        self.state.settings = settings;
        self.state.settings.save();
    }

    pub fn handle_connection_status(&mut self, status: ConnectionStatus) {
        self.ui_queue
            .blocking_send(UIMessageIn::ConnectionStatusChanged(status))
            .unwrap();
        self.state.logger.lock().unwrap().log_irc(
            logger::EventSeverity::Info,
            logger::EventDetails {
                message: format!("irc: {}", status),
            },
        );
        if matches!(status, ConnectionStatus::Connected) {
            for channel in self.state.settings.chat.autojoin.iter() {
                self.join_channel(channel);
            }
        }
    }

    pub fn handle_chat_message(&mut self, target: String, message: Message) {
        self.maybe_remember_chat(&target);
        self.ui_queue
            .blocking_send(UIMessageIn::NewMessageReceived { target, message })
            .unwrap();
    }

    fn maybe_remember_chat(&mut self, target: &str) {
        if !self.state.chats.contains(target) {
            self.state.chats.insert(target.to_owned());
            self.ui_queue
                .blocking_send(UIMessageIn::NewChatOpened(target.to_owned()))
                .unwrap();
        }
    }

    pub fn handle_server_message(&mut self, content: String) {
        self.state.logger.lock().unwrap().log_irc(
            logger::EventSeverity::Info,
            logger::EventDetails {
                message: content.clone(),
            },
        );
        self.ui_queue
            .blocking_send(UIMessageIn::NewServerMessageReceived(content))
            .unwrap();
    }

    pub fn handle_chat_error(&mut self, e: IRCError) {
        self.state.logger.lock().unwrap().log_irc(
            logger::EventSeverity::Error,
            logger::EventDetails {
                message: e.to_string(),
            },
        );
        if matches!(e, IRCError::FatalError(_)) {
            self.disconnect();
        }
    }

    pub fn handle_channel_join(&mut self, channel: String) {
        self.maybe_remember_chat(&channel);
    }

    pub fn connect(&mut self) {
        let irc_config = self.state.settings.chat.irc.clone();
        self.irc.connect(&irc_config.username, &irc_config.password);
    }

    pub fn disconnect(&self) {
        self.irc.disconnect();
    }

    pub fn ui_handle_close_chat(&mut self, name: &str) {
        self.state.chats.remove(name);
        if name.is_channel() {
            self.leave_channel(name);
        }
        self.ui_queue
            .blocking_send(UIMessageIn::ChatClosed(name.to_owned()))
            .unwrap();
    }

    pub fn send_text_message(&mut self, target: &str, text: &str) {
        self.irc.send_message(target, text);
        let message = chat::Message::new_text(&self.state.settings.chat.irc.username, text);
        self.ui_queue
            .blocking_send(UIMessageIn::NewMessageReceived {
                target: target.to_owned(),
                message,
            })
            .unwrap();
    }

    pub fn join_channel(&self, channel: &str) {
        self.irc.join_channel(channel);
    }

    pub fn leave_channel(&self, channel: &str) {
        self.irc.leave_channel(channel);
    }
}
