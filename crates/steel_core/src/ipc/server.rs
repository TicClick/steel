use crate::chat::irc::IRCError;
use crate::chat::{ConnectionStatus, Message};
use crate::settings::Settings;

#[derive(Debug)]
pub enum AppMessageIn {
    ConnectionChanged(ConnectionStatus),
    ChatError(IRCError),
    ChatMessageReceived { target: String, message: Message },
    ServerMessageReceived { content: String },
    ChannelJoined(String),

    UIConnectRequested,
    UIDisconnectRequested,
    UIExitRequested,
    UIChannelOpened(String),
    UIChannelJoinRequested(String),
    UIPrivateChatOpened(String),
    UIChatClosed(String),
    UIChatSwitchRequested(String, usize),
    UIChatMessageSent { target: String, text: String },
    UIChatActionSent { target: String, text: String },
    UISettingsRequested,
    UISettingsUpdated(Settings),

    ChatModeratorAdded(String),
}
