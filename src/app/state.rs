use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use crate::core::irc;
use crate::core::logger;
use crate::core::settings;

#[derive(Clone, Default)]
pub struct VisualCache {
    pub irc_status: irc::ConnectionStatus,
    pub active_channel_name: Option<String>,
}

#[derive(Clone, Default)]
pub struct ApplicationState {
    pub settings: settings::Settings,
    pub chats: BTreeSet<String>,
    pub logger: Arc<Mutex<logger::Logger>>,
}
