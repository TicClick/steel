use std::error::Error;
use std::path::PathBuf;

use crate::chat::irc::IRCError;
use crate::chat::{ConnectionStatus, Message};
use crate::ipc::updater::UpdateState;
use crate::settings::application::AutoUpdate;
use crate::settings::Settings;

#[derive(Debug)]
pub enum AppMessageIn {
    ConnectionChanged(ConnectionStatus),
    ConnectionActivity,
    ChatError(IRCError),
    ChatMessageReceived {
        target: String,
        message: Message,
    },
    ServerMessageReceived {
        content: String,
    },
    ChannelJoined(String),
    DateChanged(chrono::DateTime<chrono::Local>, String),

    UIConnectRequested,
    UIDisconnectRequested,
    UIRestartRequested(Option<PathBuf>),
    UIExitRequested(i32),
    UIChatOpened(String),
    UIChatClosed(String),
    UIChatCleared(String),
    UIChatSwitchRequested(String, Option<usize>),
    UIChatMessageSent {
        target: String,
        text: String,
    },
    UIChatActionSent {
        target: String,
        text: String,
    },
    UIShowError {
        error: Box<dyn Error + Sync + Send>,
        is_fatal: bool,
    },
    UIUserMentionRequested(String),
    UISettingsRequested,
    UISettingsUpdated(Settings),
    UIUsageWindowRequested,
    UIFilesystemPathRequested(String),
    UIChatLogRequested(String),

    ChatModeratorAdded(String),

    UpdateStateChanged(UpdateState),
    UpdateSettingsChanged(AutoUpdate),
    CheckApplicationUpdates,
    DownloadApplicationUpdate,
    AbortApplicationUpdate,

    UIGlassSettingsRequested,
    UIGlassSettingsUpdated(String), // YAML data
}
