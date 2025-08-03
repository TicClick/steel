use std::collections::BTreeSet;
use std::error::Error;

use date_announcer::DateAnnouncer;
use steel_core::ipc::updater::UpdateState;
use steel_core::settings::application::AutoUpdate;
use steel_core::settings::{Loadable, Settings, SETTINGS_FILE_NAME};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use steel_core::chat::irc::IRCError;
use steel_core::chat::{ChatLike, ChatState, ConnectionStatus, Message};

use crate::core::irc::IRCActorHandle;
use crate::core::logging::ChatLoggerHandle;
use crate::core::os::open_in_file_explorer;
use crate::core::updater::Updater;
use crate::core::{settings, updater};
use steel_core::ipc::{server::AppMessageIn, ui::UIMessageIn};

pub mod date_announcer;

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
    _date_announcer: DateAnnouncer,
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
            _date_announcer: DateAnnouncer::new(app_queue.clone()),
            irc: IRCActorHandle::new(app_queue.clone()),
            chat_logger: None,
            ui_queue,
            app_queue,
        }
    }

    pub fn run(&mut self) {
        while let Some(event) = self.events.blocking_recv() {
            match event {
                AppMessageIn::DateChanged(_date, message) => {
                    for chat in self.state.chats.clone() {
                        self.send_system_message(&chat, &message);
                    }
                }
                AppMessageIn::ConnectionChanged(status) => {
                    self.handle_connection_status(status);
                }
                AppMessageIn::ConnectionActivity => {
                    self.ui_queue.send(UIMessageIn::ConnectionActivity).unwrap();
                }
                AppMessageIn::ChatMessageReceived { target, message } => {
                    self.handle_chat_message(&target, message);
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

                AppMessageIn::UIRestartRequested(path) => {
                    if let Err(e) = crate::core::os::restart(path) {
                        log::error!("Failed to restart application: {:?}", e);
                        self.ui_push_backend_error(Box::new(e), false);
                    }
                }

                AppMessageIn::UIExitRequested(return_code) => {
                    std::process::exit(return_code);
                }

                AppMessageIn::UIChatOpened(target) => {
                    self.ui_handle_chat_opened(&target);
                }

                AppMessageIn::UIChatSwitchRequested(target, id) => {
                    self.ui_handle_chat_switch_requested(&target, id);
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

                AppMessageIn::UIShowError { error, is_fatal } => {
                    self.ui_push_backend_error(error, is_fatal);
                }

                AppMessageIn::UIFilesystemPathRequested(path) => {
                    if let Err(e) = open_in_file_explorer(&path) {
                        log::error!("Failed to open filesystem path {}: {}", path, e);
                        self.ui_push_backend_error(Box::new(e), false);
                    }
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

                AppMessageIn::UIGlassSettingsRequested => {
                    #[cfg(feature = "glass")]
                    self.ui_handle_glass_settings_requested();
                }
                #[allow(unused_variables)] // glass
                AppMessageIn::UIGlassSettingsUpdated(settings_yaml) => {
                    #[cfg(feature = "glass")]
                    self.ui_handle_glass_settings_updated(settings_yaml);
                }
            }
        }
    }
}

impl Application {
    pub fn ui_handle_chat_switch_requested(&self, chat: &str, message_id: Option<usize>) {
        self.ui_queue
            .send(UIMessageIn::ChatSwitchRequested(
                chat.to_owned(),
                message_id,
            ))
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
            let url_changed = u.settings.url != s.url;
            u.change_settings(s);
            if url_changed {
                u.force_check_after_url_change();
            }
        }
    }

    pub fn ui_push_backend_error(&self, error: Box<dyn Error + Send + Sync>, is_fatal: bool) {
        self.ui_queue
            .send(UIMessageIn::BackendError { error, is_fatal })
            .unwrap();
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

    pub fn initialize(&mut self) -> Result<(), steel_core::settings::SettingsError> {
        self.load_settings()?;

        log::set_max_level(self.state.settings.logging.application.level);
        self.enable_chat_logger(&self.state.settings.logging.clone());

        self.start_updater();
        if self.state.settings.chat.autoconnect {
            self.connect();
        }

        Ok(())
    }

    pub fn load_settings(&mut self) -> Result<(), steel_core::settings::SettingsError> {
        self.state.settings = settings::Settings::from_file(SETTINGS_FILE_NAME)?;

        if self.state.settings.application.autoupdate.url.is_empty() {
            self.state.settings.application.autoupdate.url = updater::default_update_url();
        }

        self.handle_chat_moderator_added("BanchoBot".into());
        self.ui_handle_settings_requested();
        Ok(())
    }

    pub fn current_settings(&self) -> &Settings {
        &self.state.settings
    }

    fn enable_chat_logger(&mut self, logging_settings: &settings::LoggingConfig) {
        self.chat_logger = Some(ChatLoggerHandle::new(&logging_settings.chat));
    }

    fn handle_logging_settings_change(&mut self, new_settings: &settings::LoggingConfig) {
        let old_settings = self.state.settings.logging.clone();
        if old_settings.application.level != new_settings.application.level {
            log::set_max_level(new_settings.application.level);
        }

        if old_settings.chat.enabled != new_settings.chat.enabled {
            match new_settings.chat.enabled {
                true => self.enable_chat_logger(new_settings),
                false => {
                    if let Some(cl) = self.chat_logger.as_ref() {
                        cl.shutdown()
                    }
                }
            }
        }

        if let Some(chat_logger) = &mut self.chat_logger {
            if old_settings.chat.directory != new_settings.chat.directory {
                chat_logger.change_directory(new_settings.chat.directory.clone());
            }

            if old_settings.chat.format != new_settings.chat.format {
                chat_logger.change_format(&new_settings.chat.format);
            }

            if old_settings.chat.log_system_events != new_settings.chat.log_system_events {
                chat_logger.log_system_messages(new_settings.chat.log_system_events);
            }
        }
    }

    pub fn ui_handle_settings_requested(&self) {
        self.ui_queue
            .send(UIMessageIn::SettingsChanged(self.state.settings.clone()))
            .unwrap();
    }

    #[cfg(feature = "glass")]
    pub fn ui_send_glass_settings(&self, settings_yaml: String) {
        self.ui_queue
            .send(UIMessageIn::GlassSettingsChanged {
                settings_data_yaml: settings_yaml,
            })
            .unwrap();
    }

    #[cfg(feature = "glass")]
    pub fn ui_handle_glass_settings_requested(&self) {
        let mut glass = glass::Glass::default();
        match glass.load_settings() {
            Ok(settings) => {
                self.ui_send_glass_settings(settings.as_string());
            }
            Err(e) => {
                self.ui_push_backend_error(Box::new(e), false);
            }
        }
    }

    #[cfg(feature = "glass")]
    pub fn ui_handle_glass_settings_updated(&self, settings_yaml: String) {
        match serde_yaml::from_str::<glass::config::GlassSettings>(&settings_yaml) {
            Ok(glass_settings) => {
                if let Err(e) = glass_settings.to_file(glass::DEFAULT_SETTINGS_PATH) {
                    self.ui_push_backend_error(Box::new(e), false);
                }
            }
            Err(e) => {
                self.ui_push_backend_error(
                    Box::new(steel_core::settings::SettingsError::YamlError(
                        "Failed to parse glass settings YAML".to_string(),
                        e,
                    )),
                    false,
                );
            }
        }
    }

    pub fn ui_handle_settings_updated(&mut self, settings: settings::Settings) {
        self.handle_logging_settings_change(&settings.logging);

        self.state.settings = settings;
        if let Err(e) = self.state.settings.to_file(SETTINGS_FILE_NAME) {
            self.ui_push_backend_error(Box::new(e), false);
        }
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
        let cold_start =
            self.state.chats.is_empty() && matches!(status, ConnectionStatus::Connected);
        self.state.connection = status;
        self.ui_queue
            .send(UIMessageIn::ConnectionStatusChanged(status))
            .unwrap();

        log::debug!("IRC connection status changed to {:?}", status);
        match status {
            ConnectionStatus::Connected => {
                for chat in self.state.chats.clone() {
                    self.rejoin_chat(&chat);
                }

                let wanted_chats = self
                    .state
                    .settings
                    .chat
                    .autojoin
                    .iter()
                    .filter(|ch| !self.state.chats.contains(&ch.to_lowercase()));
                for (idx, chat) in wanted_chats
                    .cloned()
                    .collect::<Vec<String>>()
                    .iter()
                    .enumerate()
                {
                    let switch_to_chat = cold_start && idx == 0;
                    self.save_chat(chat);
                    self.ui_add_chat(chat, switch_to_chat);
                    self.rejoin_chat(chat);
                }
            }
            ConnectionStatus::InProgress | ConnectionStatus::Scheduled(_) => (),
            ConnectionStatus::Disconnected { by_user } => {
                for chat in self.state.chats.iter().cloned().collect::<Vec<String>>() {
                    self.change_chat_state(&chat, ChatState::Disconnected);
                }
                if self.state.settings.chat.reconnect && !by_user {
                    self.queue_reconnect();
                }
            }
        }
    }

    fn rejoin_chat(&mut self, chat: &str) {
        match chat.is_channel() {
            true => {
                self.change_chat_state(chat, ChatState::JoinInProgress);
                self.join_channel(chat);
            }
            false => {
                self.change_chat_state(chat, ChatState::Joined);
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

    pub fn handle_chat_message(&mut self, target: &str, message: Message) {
        if !self.state.chats.contains(&target.to_lowercase()) {
            self.save_chat(target);
            self.ui_add_chat(target, false);
        }

        if let Some(chat_logger) = &self.chat_logger {
            chat_logger.log(target, &message);
        }

        self.ui_queue
            .send(UIMessageIn::NewMessageReceived {
                target: target.to_owned(),
                message,
            })
            .unwrap();
    }

    fn ui_handle_chat_opened(&mut self, target: &str) {
        if !self.state.chats.contains(&target.to_lowercase()) {
            self.save_chat(target);
            self.ui_add_chat(target, true);
        }
        self.ui_handle_chat_switch_requested(target, None);

        match target.is_channel() {
            true => {
                self.change_chat_state(target, ChatState::JoinInProgress);
                self.join_channel(target);
            }
            false => {
                self.change_chat_state(target, ChatState::Joined);
            }
        }
    }

    fn save_chat(&mut self, target: &str) {
        let normalized = target.to_lowercase();
        self.state.chats.insert(normalized);
    }

    fn ui_add_chat(&self, target: &str, switch: bool) {
        self.ui_queue
            .send(UIMessageIn::NewChatRequested {
                target: target.to_owned(),
                switch,
            })
            .unwrap();
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
            self.send_system_message(&chat, &content);
            if chat.is_channel() {
                self.change_chat_state(&chat, ChatState::Left);
            }
        }
        self.ui_queue
            .send(UIMessageIn::NewServerMessageReceived(error_text))
            .unwrap();
    }

    fn change_chat_state(&mut self, chat: &str, state: ChatState) {
        self.ui_queue
            .send(UIMessageIn::NewChatStateReceived {
                target: chat.to_owned(),
                state: state.clone(),
            })
            .unwrap();

        match state {
            ChatState::Left => self.send_system_message(chat, "You have left the chat"),
            ChatState::JoinInProgress => self.send_system_message(chat, "Joining the chat..."),
            ChatState::Joined => match chat.is_channel() {
                true => self.send_system_message(chat, "You have joined the chat"),
                false => self.send_system_message(chat, "You have opened the chat"),
            },
            ChatState::Disconnected => {
                self.send_system_message(chat, "You were disconnected from server");
            }
        }
    }

    fn handle_channel_join(&mut self, channel: String) {
        self.change_chat_state(&channel, ChatState::Joined);
    }

    fn send_system_message(&mut self, target: &str, text: &str) {
        let message = Message::new_system(text);
        if let Some(chat_logger) = &self.chat_logger {
            chat_logger.log(target, &message);
        }
        self.ui_queue
            .send(UIMessageIn::NewSystemMessage {
                target: target.to_owned(),
                message,
            })
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
                self.irc.connect(
                    &irc_config.username,
                    &irc_config.password,
                    &irc_config.server,
                    irc_config.ping_timeout,
                );
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
        self.handle_chat_message(target, message);
    }

    pub fn send_action(&mut self, target: &str, text: &str) {
        self.irc.send_action(target, text);
        let message = Message::new_action(&self.state.settings.chat.irc.username, text);
        self.handle_chat_message(target, message);
    }

    pub fn join_channel(&self, channel: &str) {
        self.irc.join_channel(channel);
    }

    pub fn leave_channel(&self, channel: &str) {
        self.irc.leave_channel(channel);
    }
}
