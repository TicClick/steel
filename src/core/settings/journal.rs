use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Journal {
    pub app_events: AppEvents,
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
