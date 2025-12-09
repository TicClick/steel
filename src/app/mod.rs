use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;

use date_announcer::DateAnnouncer;
use steel_core::ipc::updater::UpdateState;
use steel_core::settings::application::AutoUpdate;
use steel_core::settings::{Loadable, Settings, SETTINGS_FILE_NAME};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use steel_core::chat::irc::IRCError;
use steel_core::chat::{ChatLike, ChatState, ChatType, ConnectionStatus, Message};

use crate::core::chat_backend::ChatBackend;
use crate::core::http::HTTPActorHandle;
use crate::core::irc::IRCActorHandle;
use crate::core::logging::{chat_log_path, ChatLoggerHandle};
use crate::core::os::open_in_file_explorer;
use crate::core::updater::Updater;
use crate::core::{settings, updater};
use steel_core::ipc::{
    server::{AppMessageIn, ChatEvent, HTTPEvent, SystemEvent, UICommand, UpdateEvent},
    ui::UIMessageIn,
};
use steel_core::settings::ChatBackend as ChatBackendEnum;

pub mod date_announcer;

#[derive(Clone, Default)]
pub struct ApplicationState {
    pub settings: settings::Settings,
    pub chats: BTreeSet<String>,
    pub connection: ConnectionStatus,
    pub own_username: Option<String>,
}

pub struct Application {
    state: ApplicationState,
    events: UnboundedReceiver<AppMessageIn>,

    irc: IRCActorHandle,
    http: HTTPActorHandle,
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
            http: HTTPActorHandle::new(app_queue.clone()),
            chat_logger: None,
            ui_queue,
            app_queue,
        }
    }

    fn ui_send_or_log(&self, message: UIMessageIn) {
        if let Err(e) = self.ui_queue.send(message) {
            log::error!("Failed to send UI message: channel closed ({e})");
        }
    }

    pub fn run(&mut self) {
        while let Some(event) = self.events.blocking_recv() {
            match event {
                AppMessageIn::UI(cmd) => self.handle_ui_command(cmd),
                AppMessageIn::Chat(evt) => self.handle_chat_event(evt),
                AppMessageIn::HTTP(evt) => self.handle_http_event(evt),
                AppMessageIn::Update(evt) => self.handle_update_event(evt),
                AppMessageIn::System(evt) => self.handle_system_event(evt),
            }
        }
    }

    fn handle_ui_command(&mut self, cmd: UICommand) {
        match cmd {
            UICommand::ConnectRequested => self.connect(),
            UICommand::DisconnectRequested => self.disconnect(),
            UICommand::RestartRequested(path) => {
                if let Err(e) = crate::core::os::restart(path) {
                    log::error!("Failed to restart application: {e:?}");
                    self.ui_push_backend_error(Box::new(e), false);
                }
            }
            UICommand::ExitRequested(return_code) => std::process::exit(return_code),
            UICommand::ChatOpened(target, chat_type) => {
                self.ui_handle_chat_opened(&target, chat_type);
            }
            UICommand::ChatClosed(target, chat_type) => {
                self.ui_handle_close_chat(&target, chat_type);
            }
            UICommand::ChatCleared(target, chat_type) => {
                self.ui_handle_clear_chat(&target, chat_type);
            }
            UICommand::ChatSwitchRequested(target, chat_type, id) => {
                self.ui_handle_chat_switch_requested(&target, chat_type, id);
            }
            UICommand::ChatFilterRequested => self.ui_request_chat_filter(),
            UICommand::ChatMessageSent {
                target,
                chat_type,
                text,
            } => {
                self.send_text_message(&target, chat_type, &text);
            }
            UICommand::ChatActionSent {
                target,
                chat_type,
                text,
            } => {
                self.send_action(&target, chat_type, &text);
            }
            UICommand::UserMentionRequested(username) => {
                self.ui_handle_user_mention_requested(username);
            }
            UICommand::WindowTitleUpdateRequested => {
                self.ui_send_or_log(UIMessageIn::WindowTitleRefreshRequested);
            }
            UICommand::SettingsRequested => self.ui_handle_settings_requested(),
            UICommand::SettingsUpdated(settings) => self.ui_handle_settings_updated(*settings),
            UICommand::UsageWindowRequested => self.ui_request_usage_window(),
            UICommand::FilesystemPathRequested(path) => {
                self.ui_handle_filesystem_path_request(path);
            }
            UICommand::ChatLogRequested(target) => {
                let path = chat_log_path(
                    Path::new(&self.state.settings.logging.chat.directory),
                    &target,
                );
                self.ui_handle_filesystem_path_request(path.to_str().unwrap().to_owned());
            }
            UICommand::GlassSettingsRequested => {
                #[cfg(feature = "glass")]
                self.ui_handle_glass_settings_requested();
            }
            #[allow(unused_variables)] // glass
            UICommand::GlassSettingsUpdated(settings_yaml) => {
                #[cfg(feature = "glass")]
                self.ui_handle_glass_settings_updated(settings_yaml);
            }
            UICommand::ReportDialogRequested {
                username,
                chat_name,
            } => {
                self.ui_send_or_log(UIMessageIn::ReportDialogRequested {
                    username,
                    chat_name,
                });
            }
            UICommand::ShowError { error, is_fatal } => {
                self.ui_push_backend_error(error, is_fatal);
            }
        }
    }

    fn handle_chat_event(&mut self, evt: ChatEvent) {
        match evt {
            ChatEvent::ConnectionChanged(status) => self.handle_connection_status(status),
            ChatEvent::ConnectionActivity => {
                self.ui_send_or_log(UIMessageIn::ConnectionActivity);
            }
            ChatEvent::Error(e) => self.handle_chat_error(e),
            ChatEvent::MessageReceived { target, message } => {
                self.handle_chat_message(&target, message);
            }
            ChatEvent::ServerMessageReceived { content } => self.handle_server_message(content),
            ChatEvent::ChannelJoined(channel, chat_type) => {
                self.handle_channel_join(channel, chat_type);
            }
            ChatEvent::ModeratorAdded(username) => self.handle_chat_moderator_added(username),
            ChatEvent::OwnUsernameDetected(username) => {
                self.state.own_username = Some(username.clone());
                self.ui_send_or_log(UIMessageIn::OwnUsernameChanged(username));
            }
        }
    }

    fn handle_http_event(&mut self, evt: HTTPEvent) {
        match evt {
            HTTPEvent::AuthRequired => {
                log::info!("HTTP authentication required - waiting for user to complete OAuth");
            }
            HTTPEvent::AuthSuccess => {
                log::info!("HTTP authentication succeeded");
            }
        }
    }

    fn handle_update_event(&mut self, evt: UpdateEvent) {
        match evt {
            UpdateEvent::StateChanged(state) => self.ui_push_update_state(state),
            UpdateEvent::SettingsChanged(s) => self.change_updater_settings(s),
            UpdateEvent::CheckRequested => self.check_application_updates(),
            UpdateEvent::DownloadRequested => self.download_application_update(),
            UpdateEvent::AbortRequested => self.abort_application_update(),
        }
    }

    fn handle_system_event(&mut self, evt: SystemEvent) {
        match evt {
            SystemEvent::DateChanged(_date, message) => {
                for chat in self.state.chats.clone() {
                    self.send_system_message(&chat, &message);
                }
            }
        }
    }
}

impl Application {
    pub fn ui_handle_filesystem_path_request(&self, path: String) {
        if let Err(e) = open_in_file_explorer(&path) {
            log::error!("Failed to open filesystem path {path}: {e}");
            self.ui_push_backend_error(Box::new(e), false);
        }
    }

    pub fn ui_handle_chat_switch_requested(
        &self,
        chat: &str,
        _chat_type: ChatType,
        message_id: Option<usize>,
    ) {
        self.ui_send_or_log(UIMessageIn::ChatSwitchRequested(
            chat.to_owned(),
            message_id,
        ));
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
        self.ui_send_or_log(UIMessageIn::BackendError { error, is_fatal });
    }

    pub fn ui_handle_user_mention_requested(&self, username: String) {
        self.ui_send_or_log(UIMessageIn::UIUserMentionRequested(username));
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
        self.ui_send_or_log(UIMessageIn::SettingsChanged(Box::new(
            self.state.settings.clone(),
        )));
    }

    #[cfg(feature = "glass")]
    pub fn ui_send_glass_settings(&self, settings_yaml: String) {
        self.ui_send_or_log(UIMessageIn::GlassSettingsChanged {
            settings_data_yaml: settings_yaml,
        });
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
        self.ui_send_or_log(UIMessageIn::UsageWindowRequested);
    }

    pub fn ui_request_chat_filter(&mut self) {
        self.ui_send_or_log(UIMessageIn::ChatFilterRequested);
    }

    pub fn ui_push_update_state(&mut self, state: UpdateState) {
        self.ui_send_or_log(UIMessageIn::UpdateStateChanged(state));
    }

    pub fn handle_connection_status(&mut self, status: ConnectionStatus) {
        let cold_start =
            self.state.chats.is_empty() && matches!(status, ConnectionStatus::Connected);
        self.state.connection = status;
        self.ui_send_or_log(UIMessageIn::ConnectionStatusChanged(status));

        log::debug!("IRC connection status changed to {status:?}");
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
        self.ui_send_or_log(UIMessageIn::ConnectionStatusChanged(
            ConnectionStatus::Scheduled(reconnect_time),
        ));

        std::thread::spawn(move || {
            std::thread::sleep(delta.to_std().unwrap());
            if let Err(e) = queue.send(AppMessageIn::ui_connect_requested()) {
                log::error!("Failed to trigger reconnection: channel closed ({e})");
            }
        });
    }

    pub fn handle_chat_message(&mut self, target: &str, mut message: Message) {
        if !self.state.chats.contains(&target.to_lowercase()) {
            self.save_chat(target);
            self.ui_add_chat(target, false);
        }

        message.parse_for_links();

        if let Some(chat_logger) = &self.chat_logger {
            chat_logger.log(target, &message);
        }

        self.ui_send_or_log(UIMessageIn::NewMessageReceived {
            target: target.to_owned(),
            message,
        });
    }

    fn ui_handle_chat_opened(&mut self, target: &str, chat_type: ChatType) {
        if !self.state.chats.contains(&target.to_lowercase()) {
            self.save_chat(target);
            self.ui_add_chat(target, true);
        }
        self.ui_handle_chat_switch_requested(target, chat_type.clone(), None);

        match chat_type {
            ChatType::Channel => {
                self.change_chat_state(target, ChatState::JoinInProgress);
                self.join_channel(target);
            }
            ChatType::Person => {
                self.change_chat_state(target, ChatState::Joined);
            }
            ChatType::System => {
                log::error!("Impossible chat type requested for join from ui_handle_chat_opened: {chat_type}");
            }
        }
    }

    fn save_chat(&mut self, target: &str) {
        let normalized = target.to_lowercase();
        self.state.chats.insert(normalized);
    }

    fn ui_add_chat(&self, target: &str, switch: bool) {
        self.ui_send_or_log(UIMessageIn::NewChatRequested {
            target: target.to_owned(),
            switch,
        });
    }

    pub fn handle_server_message(&mut self, content: String) {
        log::debug!("IRC server message: {content}");
        self.ui_send_or_log(UIMessageIn::NewServerMessageReceived(content));
    }

    pub fn handle_chat_error(&mut self, e: IRCError) {
        // FIXME: FIX LOGGING
        log::error!("IRC chat error: {e:?}");
        if matches!(e, IRCError::FatalError(_)) {
            self.get_backend().disconnect();
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
        self.ui_send_or_log(UIMessageIn::NewServerMessageReceived(error_text));
    }

    fn change_chat_state(&mut self, chat: &str, state: ChatState) {
        self.ui_send_or_log(UIMessageIn::NewChatStateReceived {
            target: chat.to_owned(),
            state: state.clone(),
        });

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

    fn handle_channel_join(&mut self, channel: String, _chat_type: ChatType) {
        self.change_chat_state(&channel, ChatState::Joined);
    }

    fn send_system_message(&mut self, target: &str, text: &str) {
        let message = Message::new_system(text);
        if let Some(chat_logger) = &self.chat_logger {
            chat_logger.log(target, &message);
        }
        self.ui_send_or_log(UIMessageIn::NewSystemMessage {
            target: target.to_owned(),
            message,
        });
    }

    pub fn handle_chat_moderator_added(&mut self, username: String) {
        self.ui_send_or_log(UIMessageIn::ChatModeratorAdded(username));
    }

    fn get_backend(&self) -> &dyn ChatBackend {
        match self.state.settings.chat.backend {
            ChatBackendEnum::IRC => &self.irc as &dyn ChatBackend,
            ChatBackendEnum::API => &self.http as &dyn ChatBackend,
        }
    }

    pub fn connect(&mut self) {
        match self.state.connection {
            ConnectionStatus::Connected | ConnectionStatus::InProgress => {}
            ConnectionStatus::Disconnected { .. } | ConnectionStatus::Scheduled(_) => {
                self.get_backend().connect(&self.state.settings.chat);
            }
        }
    }

    pub fn disconnect(&mut self) {
        if !matches!(self.state.connection, ConnectionStatus::Connected) {
            return;
        }
        self.get_backend().disconnect();
    }

    pub fn ui_handle_close_chat(&mut self, name: &str, chat_type: ChatType) {
        let normalized = name.to_lowercase();
        self.state.chats.remove(&normalized);

        match chat_type {
            ChatType::Channel => self.leave_channel(name),
            ChatType::Person | ChatType::System => (),
        }

        if let Some(chat_logger) = &self.chat_logger {
            chat_logger.close_log(normalized);
        }

        self.ui_send_or_log(UIMessageIn::ChatClosed(name.to_owned()));
    }

    pub fn ui_handle_clear_chat(&mut self, name: &str, _chat_type: ChatType) {
        let normalized = name.to_lowercase();
        self.ui_send_or_log(UIMessageIn::ChatCleared(normalized));
    }

    pub fn send_text_message(&mut self, target: &str, chat_type: ChatType, text: &str) {
        self.get_backend().send_message(target, chat_type, text);
    }

    pub fn send_action(&mut self, target: &str, chat_type: ChatType, text: &str) {
        self.get_backend().send_action(target, chat_type, text);
    }

    pub fn join_channel(&self, channel: &str) {
        self.get_backend().join_channel(channel);
    }

    pub fn leave_channel(&self, channel: &str) {
        self.get_backend().leave_channel(channel);
    }
}
