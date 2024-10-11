use serde::{Deserialize, Serialize};

use crate::DEFAULT_DATETIME_FORMAT;

pub const DEFAULT_LOG_DIRECTORY: &str = "./chat-logs";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub application: AppLoggingConfig,
    pub chat: ChatLoggingConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppLoggingConfig {
    #[serde(with = "LevelFilterDef")]
    pub level: log::LevelFilter,
}

impl Default for AppLoggingConfig {
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ChatLoggingFormats {
    pub date: String,
    pub user_message: String,
    pub user_action: String,
    pub system_message: String,
}

impl Default for ChatLoggingFormats {
    fn default() -> Self {
        Self {
            date: DEFAULT_DATETIME_FORMAT.to_owned(),
            user_message: "{date} <{username}> {text}".to_owned(),
            user_action: "{date} * {username} {text}".to_owned(),
            system_message: "{date} * {text}".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ChatLoggingConfig {
    pub enabled: bool,
    pub directory: String,
    pub format: ChatLoggingFormats,
    pub log_system_events: bool,
}

impl Default for ChatLoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            directory: DEFAULT_LOG_DIRECTORY.to_owned(),
            format: ChatLoggingFormats::default(),
            log_system_events: true,
        }
    }
}
