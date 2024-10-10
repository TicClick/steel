use serde::{Deserialize, Serialize};

use crate::DEFAULT_DATETIME_FORMAT;

pub const DEFAULT_LOG_DIRECTORY: &str = "./chat-logs";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Journal {
    pub app_events: AppEvents,
    pub chat_events: ChatEvents,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppEvents {
    #[serde(with = "LevelFilterDef")]
    pub level: log::LevelFilter,
}

impl Default for AppEvents {
    fn default() -> Self {
        Self {
            level: log::LevelFilter::Warn,
        }
    }
}

// Unfortunate copypaste: https://serde.rs/remote-derive.html
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[serde(remote = "log::LevelFilter")]
enum LevelFilterDef {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ChatEvents {
    pub enabled: bool,
    pub directory: String,
    pub format: String,
    pub log_system_events: bool,
}

impl Default for ChatEvents {
    fn default() -> Self {
        Self {
            enabled: true,
            directory: DEFAULT_LOG_DIRECTORY.to_owned(),
            format: format!(
                "{{date:{}}} <{{username}}> {{text}}",
                DEFAULT_DATETIME_FORMAT
            ),
            log_system_events: true,
        }
    }
}
