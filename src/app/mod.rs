pub mod server;
pub mod state;

use crate::core::chat;
use crate::core::irc::{ConnectionStatus, IRCError};
use crate::core::settings::Settings;

#[derive(Debug)]
pub enum AppMessageIn {
    Connect,
    Disconnect,

    ConnectionChanged(ConnectionStatus),
    ChatError(IRCError),
    ChatMessageReceived {
        target: String,
        message: chat::Message,
    },
    ServerMessageReceived {
        content: String,
    },
    ChannelJoined(String),

    UIConnectRequested,
    UIDisconnectRequested,
    UIExitRequested,
    UIChannelOpened(String),
    UIPrivateChatOpened(String),
    UIChatClosed(String),
    UIChatMessageSent {
        target: String,
        text: String,
    },
    UISettingsRequested,
    UISettingsUpdated(Settings),
}
