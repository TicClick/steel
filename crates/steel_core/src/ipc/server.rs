use std::error::Error;
use std::path::PathBuf;

use crate::chat::irc::IRCError;
use crate::chat::{ChatType, ConnectionStatus, Message};
use crate::ipc::updater::UpdateState;
use crate::settings::application::AutoUpdate;
use crate::settings::Settings;

#[derive(Debug)]
pub enum UICommand {
    ConnectRequested,
    DisconnectRequested,
    RestartRequested(Option<PathBuf>),
    ExitRequested(i32),
    ChatOpened(String, ChatType),
    ChatClosed(String, ChatType),
    ChatCleared(String, ChatType),
    ChatSwitchRequested(String, ChatType, Option<usize>),
    ChatFilterRequested,
    ChatMessageSent {
        target: String,
        chat_type: ChatType,
        text: String,
    },
    ChatActionSent {
        target: String,
        chat_type: ChatType,
        text: String,
    },
    UserMentionRequested(String),
    WindowTitleUpdateRequested,
    SettingsRequested,
    SettingsUpdated(Box<Settings>),
    UsageWindowRequested,
    FilesystemPathRequested(String),
    ChatLogRequested(String),
    GlassSettingsRequested,
    GlassSettingsUpdated(String),
    ReportDialogRequested {
        username: String,
        chat_name: String,
    },
    ShowError {
        error: Box<dyn Error + Sync + Send>,
        is_fatal: bool,
    },
}

#[derive(Debug)]
pub enum ChatEvent {
    ConnectionChanged(ConnectionStatus),
    ConnectionActivity,
    Error(IRCError),
    MessageReceived { target: String, message: Message },
    ServerMessageReceived { content: String },
    ChannelJoined(String, ChatType),
    ModeratorAdded(String),
    OwnUsernameDetected(String),
}

#[derive(Debug)]
pub enum HTTPEvent {
    AuthRequired,
    AuthSuccess,
}

#[derive(Debug)]
pub enum UpdateEvent {
    StateChanged(UpdateState),
    SettingsChanged(AutoUpdate),
    CheckRequested,
    DownloadRequested,
    AbortRequested,
}

#[derive(Debug)]
pub enum SystemEvent {
    DateChanged(chrono::DateTime<chrono::Local>, String),
}

#[derive(Debug)]
pub enum AppMessageIn {
    UI(UICommand),
    Chat(ChatEvent),
    HTTP(HTTPEvent),
    Update(UpdateEvent),
    System(SystemEvent),
}

impl AppMessageIn {
    // Chat events
    pub fn connection_changed(status: ConnectionStatus) -> Self {
        Self::Chat(ChatEvent::ConnectionChanged(status))
    }

    pub fn connection_activity() -> Self {
        Self::Chat(ChatEvent::ConnectionActivity)
    }

    pub fn chat_error(e: IRCError) -> Self {
        Self::Chat(ChatEvent::Error(e))
    }

    pub fn chat_message_received(target: String, message: Message) -> Self {
        Self::Chat(ChatEvent::MessageReceived { target, message })
    }

    pub fn server_message_received(content: String) -> Self {
        Self::Chat(ChatEvent::ServerMessageReceived { content })
    }

    pub fn channel_joined(channel: String, chat_type: ChatType) -> Self {
        Self::Chat(ChatEvent::ChannelJoined(channel, chat_type))
    }

    pub fn moderator_added(username: String) -> Self {
        Self::Chat(ChatEvent::ModeratorAdded(username))
    }

    pub fn own_username_detected(username: String) -> Self {
        Self::Chat(ChatEvent::OwnUsernameDetected(username))
    }

    // UI commands
    pub fn ui_connect_requested() -> Self {
        Self::UI(UICommand::ConnectRequested)
    }

    pub fn ui_disconnect_requested() -> Self {
        Self::UI(UICommand::DisconnectRequested)
    }

    pub fn ui_chat_opened(target: String, chat_type: ChatType) -> Self {
        Self::UI(UICommand::ChatOpened(target, chat_type))
    }

    pub fn ui_chat_message_sent(target: String, chat_type: ChatType, text: String) -> Self {
        Self::UI(UICommand::ChatMessageSent {
            target,
            chat_type,
            text,
        })
    }

    pub fn ui_show_error(error: Box<dyn Error + Send + Sync>, is_fatal: bool) -> Self {
        Self::UI(UICommand::ShowError { error, is_fatal })
    }

    // HTTP events
    pub fn http_auth_required() -> Self {
        Self::HTTP(HTTPEvent::AuthRequired)
    }

    pub fn http_auth_success() -> Self {
        Self::HTTP(HTTPEvent::AuthSuccess)
    }

    // Update events
    pub fn update_state_changed(state: UpdateState) -> Self {
        Self::Update(UpdateEvent::StateChanged(state))
    }

    // System events
    pub fn date_changed(date: chrono::DateTime<chrono::Local>, message: String) -> Self {
        Self::System(SystemEvent::DateChanged(date, message))
    }
}
