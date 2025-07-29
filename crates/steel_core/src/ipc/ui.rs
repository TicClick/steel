use crate::chat::ConnectionStatus;
use crate::chat::{ChatState, Message};
use crate::ipc::updater::UpdateState;
use crate::settings::Settings;

#[derive(Debug)]
pub enum UIMessageIn {
    SettingsChanged(Settings),
    ConnectionStatusChanged(ConnectionStatus),
    ConnectionActivity,
    NewSystemMessage { target: String, message: Message },
    NewMessageReceived { target: String, message: Message },
    NewServerMessageReceived(String),
    NewChatStateReceived { target: String, state: ChatState },
    NewChatRequested { target: String, switch: bool },
    ChatSwitchRequested(String, Option<usize>),
    ChatClosed(String),
    ChatCleared(String),
    ChatModeratorAdded(String),
    UsageWindowRequested,
    UpdateStateChanged(UpdateState),
}
