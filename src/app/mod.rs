use std::collections::BTreeSet;

use steel_core::ipc::updater::UpdateState;
use steel_core::settings::application::AutoUpdate;
use steel_core::settings::Loadable;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use steel_core::chat::irc::IRCError;
use steel_core::chat::{ChatLike, ChatState, ConnectionStatus, Message};

use crate::core::irc::IRCActorHandle;
use crate::core::logging::ChatLoggerHandle;
use crate::core::updater::Updater;
use crate::core::{settings, updater};
use steel_core::ipc::{server::AppMessageIn, ui::UIMessageIn};

const DEFAULT_SETTINGS_PATH: &str = "settings.yaml";

#[derive(Clone, Default)]
pub struct ApplicationState {
    pub settings: settings::Settings,
    pub chats: BTreeSet<String>,
    pub connection: ConnectionStatus,
}

pub struct Application {
    state: ApplicationState,
    events: UnboundedReceiver<AppMessageIn>,

    irc: IRCActorHandle,
    chat_logger: Option<ChatLoggerHandle>,
    updater: Option<Updater>,
    ui_queue: UnboundedSender<UIMessageIn>,
    pub app_queue: UnboundedSender<AppMessageIn>,
}

impl Application {
    pub fn new(ui_queue: UnboundedSender<UIMessageIn>) -> Self {
        let (app_queue, events) = unbounded_channel();
        Self {
            state: ApplicationState::default(),
            events,
            updater: None,
            irc: IRCActorHandle::new(app_queue.clone()),
            chat_logger: None,
            ui_queue,
            app_queue,
        }
    }

    pub fn run(&mut self) {
        while let Some(event) = self.events.blocking_recv() {
            match event {
                AppMessageIn::ConnectionChanged(status) => {
                    self.handle_connection_status(status);
                }
                AppMessageIn::ChatMessageReceived { target, message } => {
                    self.handle_chat_message(target, message, false);
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
                AppMessageIn::ChatModeratorAdded(username) => {
                    self.handle_chat_moderator_added(username);
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
                AppMessageIn::UIChannelOpened(target)
                | AppMessageIn::UIPrivateChatOpened(target) => {
                    self.maybe_remember_chat(&target, true);
                }
                AppMessageIn::UIChatSwitchRequested(target, id) => {
                    self.ui_handle_chat_switch_requested(target, id);
                }
                AppMessageIn::UIChannelJoinRequested(channel) => {
                    self.handle_ui_channel_join_requested(channel);
                }
                AppMessageIn::UIChatClosed(target) => {
                    self.ui_handle_close_chat(&target);
                }
                AppMessageIn::UIChatCleared(target) => {
                    self.ui_handle_clear_chat(&target);
                }
                AppMessageIn::UIChatMessageSent { target, text } => {
                    self.send_text_message(&target, &text);
                }
                AppMessageIn::UIChatActionSent { target, text } => {
                    self.send_action(&target, &text);
                }
                AppMessageIn::UISettingsRequested => {
                    self.ui_handle_settings_requested();
                }
                AppMessageIn::UISettingsUpdated(settings) => {
                    self.ui_handle_settings_updated(settings);
                }
                AppMessageIn::UIUsageWindowRequested => {
                    self.ui_request_usage_window();
                }

                AppMessageIn::UpdateStateChanged(state) => {
                    self.ui_push_update_state(state);
                }
                AppMessageIn::UpdateSettingsChanged(s) => {
                    self.change_updater_settings(s);
                }
                AppMessageIn::CheckApplicationUpdates => {
                    self.check_application_updates();
                }
                AppMessageIn::DownloadApplicationUpdate => {
                    self.download_application_update();
                }
                AppMessageIn::AbortApplicationUpdate => {
                    self.abort_application_update();
                }
            }
        }
    }
}

impl Application {
    pub fn handle_ui_channel_join_requested(&mut self, channel: String) {
        self.maybe_remember_chat(&channel, true);
        self.join_channel(&channel);
    }

    pub fn ui_handle_chat_switch_requested(&self, chat: String, message_id: Option<usize>) {
        self.ui_queue
            .send(UIMessageIn::ChatSwitchRequested(chat, message_id))
            .unwrap();
    }

    pub fn start_updater(&mut self) {
        if self.updater.is_none() {
            self.updater = Some(Updater::new(
                self.app_queue.clone(),
                self.state.settings.application.autoupdate.clone(),
            ));
        }
    }

    pub fn change_updater_settings(&mut self, s: AutoUpdate) {
        if let Some(u) = &mut self.updater {
            u.change_settings(s);
        }
    }

    pub fn check_application_updates(&self) {
        if let Some(u) = &self.updater {
            u.check_version();
        }
    }

    pub fn download_application_update(&self) {
        if let Some(u) = &self.updater {
            u.download_new_version();
        }
    }

    pub fn abort_application_update(&self) {
        if let Some(u) = &self.updater {
            u.abort_update();
        }
    }

    pub fn initialize(&mut self) {
        self.load_settings(true);
        log::set_max_level(self.state.settings.journal.app_events.level);

        self.enable_chat_logger(&self.state.settings.journal.clone());

        self.start_updater();
        if self.state.settings.chat.autoconnect {
            self.connect();
        }
    }

    pub fn load_settings(&mut self, fallback: bool) {
        self.state.settings = settings::Settings::from_file(DEFAULT_SETTINGS_PATH, fallback);

        if self.state.settings.application.autoupdate.url.is_empty() {
            self.state.settings.application.autoupdate.url = updater::default_update_url();
        }

        self.handle_chat_moderator_added("BanchoBot".into());
        self.ui_handle_settings_requested();
    }

    fn enable_chat_logger(&mut self, logging_settings: &settings::Journal) {
        self.chat_logger = Some(ChatLoggerHandle::new(
            &logging_settings.chat_events.directory,
            &logging_settings.chat_events.format,
        ));
    }

    fn handle_logging_settings_change(&mut self, new_settings: &settings::Journal) {
        let old_settings = self.state.settings.journal.clone();
        if old_settings.app_events.level != new_settings.app_events.level {
            log::set_max_level(new_settings.app_events.level);
        }

        if old_settings.chat_events.enabled != new_settings.chat_events.enabled {
            match new_settings.chat_events.enabled {
                true => self.enable_chat_logger(new_settings),
                false => {
                    if let Some(cl) = self.chat_logger.as_ref() {
                        cl.shutdown()
                    }
                }
            }
        }

        if let Some(chat_logger) = &self.chat_logger {
            if old_settings.chat_events.directory != new_settings.chat_events.directory {
                chat_logger.change_logging_directory(new_settings.chat_events.directory.clone());
            }

            if old_settings.chat_events.format != new_settings.chat_events.format {
                chat_logger.change_log_format(new_settings.chat_events.format.clone());
            }
        }
    }

    pub fn ui_handle_settings_requested(&self) {
        self.ui_queue
            .send(UIMessageIn::SettingsChanged(self.state.settings.clone()))
            .unwrap();
    }

    pub fn ui_handle_settings_updated(&mut self, settings: settings::Settings) {
        self.handle_logging_settings_change(&settings.journal);

        self.state.settings = settings;
        self.state.settings.to_file(DEFAULT_SETTINGS_PATH);
    }

    pub fn ui_request_usage_window(&mut self) {
        self.ui_queue
            .send(UIMessageIn::UsageWindowRequested)
            .unwrap();
    }

    pub fn ui_push_update_state(&mut self, state: UpdateState) {
        self.ui_queue
            .send(UIMessageIn::UpdateStateChanged(state))
            .unwrap();
    }

    pub fn handle_connection_status(&mut self, status: ConnectionStatus) {
        self.state.connection = status;
        self.ui_queue
            .send(UIMessageIn::ConnectionStatusChanged(status))
            .unwrap();
        log::debug!("IRC connection status changed to {:?}", status);
        match status {
            ConnectionStatus::Connected => {
                let chats = self.state.settings.chat.autojoin.clone();
                let connected_to: Vec<String> = self.state.chats.iter().cloned().collect();
                for cs in [chats, connected_to] {
                    for chat in cs {
                        self.maybe_remember_chat(&chat, false);
                        if chat.is_channel() {
                            self.join_channel(&chat);
                        } else {
                            self.push_chat_to_ui(&chat, false);
                        }
                    }
                }
            }
            ConnectionStatus::InProgress | ConnectionStatus::Scheduled(_) => (),
            ConnectionStatus::Disconnected { by_user } => {
                if self.state.settings.chat.reconnect && !by_user {
                    self.queue_reconnect();
                }
            }
        }
    }

    fn queue_reconnect(&self) {
        let queue = self.app_queue.clone();
        let delta = chrono::Duration::seconds(15);
        let reconnect_time = chrono::Local::now() + delta;
        self.ui_queue
            .send(UIMessageIn::ConnectionStatusChanged(
                ConnectionStatus::Scheduled(reconnect_time),
            ))
            .unwrap();

        std::thread::spawn(move || {
            std::thread::sleep(delta.to_std().unwrap());
            queue
                .send(AppMessageIn::UIConnectRequested)
                .expect("failed to trigger reconnection");
        });
    }

    fn push_chat_to_ui(&self, target: &str, switch: bool) {
        let chat_state = if target.is_channel() {
            ChatState::JoinInProgress
        } else {
            ChatState::Joined
        };
        self.ui_queue
            .send(UIMessageIn::NewChatRequested(
                target.to_owned(),
                chat_state,
                switch,
            ))
            .unwrap();
    }

    pub fn handle_chat_message(
        &mut self,
        target: String,
        message: Message,
        switch_if_missing: bool,
    ) {
        if let Some(chat_logger) = &self.chat_logger {
            chat_logger.log(target.clone(), message.clone());
        }

        self.maybe_remember_chat(&target, switch_if_missing);
        self.ui_queue
            .send(UIMessageIn::NewMessageReceived { target, message })
            .unwrap();
    }

    fn maybe_remember_chat(&mut self, target: &str, switch_if_missing: bool) {
        let normalized = target.to_lowercase();
        if !self.state.chats.contains(&normalized) {
            self.state.chats.insert(normalized);
            self.push_chat_to_ui(target, switch_if_missing);
        }
    }

    pub fn handle_server_message(&mut self, content: String) {
        log::debug!("IRC server message: {}", content);
        self.ui_queue
            .send(UIMessageIn::NewServerMessageReceived(content))
            .unwrap();
    }

    pub fn handle_chat_error(&mut self, e: IRCError) {
        log::error!("IRC chat error: {:?}", e);
        if matches!(e, IRCError::FatalError(_)) {
            self.irc.disconnect();
        }

        let error_text = e.to_string();
        if let IRCError::ServerError {
            code: _,
            chat: Some(chat),
            content,
        } = e
        {
            let normalized = chat.to_lowercase();
            self.state.chats.remove(&normalized);
            self.ui_queue
                .send(UIMessageIn::NewChatStatusReceived {
                    target: chat,
                    state: ChatState::Left,
                    details: content,
                })
                .unwrap();
        }
        self.ui_queue
            .send(UIMessageIn::NewServerMessageReceived(error_text))
            .unwrap();
    }

    pub fn handle_channel_join(&mut self, channel: String) {
        self.ui_queue
            .send(UIMessageIn::ChannelJoined(channel))
            .unwrap();
    }

    pub fn handle_chat_moderator_added(&mut self, username: String) {
        self.ui_queue
            .send(UIMessageIn::ChatModeratorAdded(username))
            .unwrap();
    }

    pub fn connect(&mut self) {
        match self.state.connection {
            ConnectionStatus::Connected | ConnectionStatus::InProgress => {}
            ConnectionStatus::Disconnected { .. } | ConnectionStatus::Scheduled(_) => {
                let irc_config = self.state.settings.chat.irc.clone();
                self.irc.connect(&irc_config.username, &irc_config.password);
            }
        }
    }

    pub fn disconnect(&mut self) {
        if !matches!(self.state.connection, ConnectionStatus::Connected) {
            return;
        }
        self.irc.disconnect();
    }

    pub fn ui_handle_close_chat(&mut self, name: &str) {
        let normalized = name.to_lowercase();
        self.state.chats.remove(&normalized);
        if name.is_channel() {
            self.leave_channel(name);
        }

        if let Some(chat_logger) = &self.chat_logger {
            chat_logger.close_log(normalized);
        }

        self.ui_queue
            .send(UIMessageIn::ChatClosed(name.to_owned()))
            .unwrap();
    }

    pub fn ui_handle_clear_chat(&mut self, name: &str) {
        let normalized = name.to_lowercase();
        self.ui_queue
            .send(UIMessageIn::ChatCleared(normalized))
            .unwrap();
    }

    pub fn send_text_message(&mut self, target: &str, text: &str) {
        self.irc.send_message(target, text);
        let message = Message::new_text(&self.state.settings.chat.irc.username, text);
        self.ui_queue
            .send(UIMessageIn::NewMessageReceived {
                target: target.to_owned(),
                message,
            })
            .unwrap();
    }

    pub fn send_action(&mut self, target: &str, text: &str) {
        self.irc.send_action(target, text);
        let message = Message::new_action(&self.state.settings.chat.irc.username, text);
        self.ui_queue
            .send(UIMessageIn::NewMessageReceived {
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
