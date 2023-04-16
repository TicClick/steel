pub mod about;
pub mod chat;
pub mod chat_tabs;
pub mod highlights;
pub mod menu;
pub mod settings;
pub mod state;
pub mod window;

use crate::core::chat::{ChatState, Message};
use crate::core::irc::ConnectionStatus;
use crate::core::settings::Settings;

const HIGHLIGHTS_TAB_NAME: &str = "highlights";
const SERVER_TAB_NAME: &str = "server";

#[derive(Debug)]
pub enum UIMessageIn {
    SettingsChanged(Settings),
    ConnectionStatusChanged(ConnectionStatus),
    NewMessageReceived { target: String, message: Message },
    NewServerMessageReceived(String),
    NewChatRequested(String, ChatState),
    ChatSwitchRequested(String, usize),
    ChannelJoined(String),
    ChatClosed(String),
    DateChanged,
}
