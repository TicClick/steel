use serde::{Deserialize, Serialize};

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
    pub with_system_events: bool,
}

impl Default for ChatEvents {
    fn default() -> Self {
        Self {
            enabled: true,
            directory: "./chat-logs".to_owned(),
            format: "[{date:%Y-%m-%d %H:%M:%S}] <{username}> {text}".to_owned(),
            with_system_events: true,
        }
    }
}
