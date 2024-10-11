pub mod application;
pub mod chat;
pub mod colour;
pub mod logging;
pub mod notifications;
pub mod ui;

use std::io::Write;

use serde::{Deserialize, Serialize};
use serde_yaml;

pub use application::Application;
pub use chat::{Chat, ChatBackend, HTTPChatSettings, IRCChatSettings};
pub use colour::Colour;
pub use logging::{AppLoggingConfig, LoggingConfig};
pub use notifications::{BuiltInSound, Highlights, Notifications, Sound};
pub use ui::{ChatColours, ThemeMode, UI};

pub const SETTINGS_FILE_PATH: &str = "./settings.yaml";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub application: Application,
    pub chat: Chat,
    pub notifications: Notifications,
    pub ui: UI,
    pub logging: LoggingConfig,
}

pub trait Loadable: Sized + Default + Serialize + for<'de> Deserialize<'de> {
    fn from_file(source: &str, fallback: bool) -> Self {
        log::info!("Loading settings from {:?}", source);
        match std::fs::read_to_string(source) {
            Ok(contents) => match serde_yaml::from_str::<Self>(&contents) {
                Ok(obj) => obj,
                Err(e) => {
                    panic!("Error while loading the config: {}", e);
                }
            },
            Err(e) => {
                if fallback {
                    return Self::default();
                }
                panic!("Error reading file at {:?}: {}", source, e);
            }
        }
    }

    fn to_file(&self, path: &str) {
        match serde_yaml::to_string(self) {
            Ok(s) => match std::fs::File::create(path) {
                Ok(mut f) => {
                    if f.write(s.as_bytes()).is_err() {
                        panic!("Failed to save settings")
                    }
                }
                Err(e) => {
                    panic!("Failed to save settings to {:?}: {}", path, e);
                }
            },
            Err(e) => {
                panic!("Error saving settings: {}", e);
            }
        }
    }
}

impl Loadable for Settings {}
