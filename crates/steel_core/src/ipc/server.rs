use crate::chat::irc::IRCError;
use crate::chat::{ConnectionStatus, Message};
use crate::ipc::updater::UpdateState;
use crate::settings::application::AutoUpdate;
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
    UIChatCleared(String),
    UIChatSwitchRequested(String, Option<usize>),
    UIChatMessageSent { target: String, text: String },
    UIChatActionSent { target: String, text: String },
    UISettingsRequested,
    UISettingsUpdated(Settings),
    UIUsageWindowRequested,

    ChatModeratorAdded(String),

    UpdateStateChanged(UpdateState),
    UpdateSettingsChanged(AutoUpdate),
    CheckApplicationUpdates,
    DownloadApplicationUpdate,
    AbortApplicationUpdate,
}
