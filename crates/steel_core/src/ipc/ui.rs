use std::error::Error;

use crate::chat::ConnectionStatus;
use crate::chat::{ChatState, Message};
use crate::ipc::updater::UpdateState;
use crate::settings::Settings;

#[derive(Debug)]
pub enum UIMessageIn {
    SettingsChanged(Box<Settings>),
    ConnectionStatusChanged(ConnectionStatus),
    ConnectionActivity,
    NewSystemMessage {
        target: String,
        message: Message,
    },
    NewMessageReceived {
        target: String,
        message: Message,
    },
    NewServerMessageReceived(String),
    NewChatStateReceived {
        target: String,
        state: ChatState,
    },
    NewChatRequested {
        target: String,
        switch: bool,
    },
    ChatSwitchRequested(String, Option<usize>),
    ChatClosed(String),
    ChatCleared(String),
    ChatModeratorAdded(String),
    UIUserMentionRequested(String),
    UsageWindowRequested,
    UpdateStateChanged(UpdateState),
    BackendError {
        error: Box<dyn Error + Send + Sync>,
        is_fatal: bool,
    },

    // Pass raw settings to avoid depending on the glass module.
    GlassSettingsChanged {
        settings_data_yaml: String,
    },
}
