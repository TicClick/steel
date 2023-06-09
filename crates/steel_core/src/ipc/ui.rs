use crate::chat::ConnectionStatus;
use crate::chat::{ChatState, Message};
use crate::settings::Settings;

#[derive(Debug)]
pub enum UIMessageIn {
    SettingsChanged(Settings),
    ConnectionStatusChanged(ConnectionStatus),
    NewMessageReceived {
        target: String,
        message: Message,
    },
    NewServerMessageReceived(String),
    NewChatStatusReceived {
        target: String,
        state: ChatState,
        details: String,
    },
    NewChatRequested(String, ChatState, bool),
    ChatSwitchRequested(String, usize),
    ChannelJoined(String),
    ChatClosed(String),
    DateChanged(chrono::DateTime<chrono::Local>),
}
