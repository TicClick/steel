pub mod application;
pub mod chat;
pub mod colour;
pub mod errors;
pub mod logging;
pub mod notifications;
pub mod ui;

use std::io::Write;

use serde::{Deserialize, Serialize};
use serde_yaml;

pub use application::Application;
pub use chat::{Chat, ChatBackend, HTTPChatSettings, IRCChatSettings};
pub use colour::Colour;
pub use errors::SettingsError;
pub use logging::{AppLoggingConfig, LoggingConfig};
pub use notifications::{
    BuiltInSound, Highlights, NotificationEvents, NotificationStyle, Notifications, Sound,
};
pub use ui::{ChatColours, ThemeMode, UI};

pub const SETTINGS_FILE_NAME: &str = "settings.yaml";

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
    fn from_file(source: &str) -> Result<Self, SettingsError> {
        log::info!("Loading settings from {:?}", source);
        match std::fs::read_to_string(source) {
            Ok(contents) => match serde_yaml::from_str::<Self>(&contents) {
                Ok(obj) => Ok(obj),
                Err(e) => Err(SettingsError::YamlError(
                    format!("Failed to parse structure of the settings file {source} on startup"),
                    e,
                )),
            },
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    return Ok(Self::default());
                }
                Err(SettingsError::IoError(
                    format!("Failed to read the settings file {source} on startup"),
                    e,
                ))
            }
        }
    }

    fn to_file(&self, path: &str) -> Result<(), SettingsError> {
        let s = serde_yaml::to_string(self).map_err(|e| {
            SettingsError::YamlError(
                format!("Failed to serialize settings to YAML for {path}"),
                e,
            )
        })?;
        
        let mut f = std::fs::File::create(path).map_err(|e| {
            SettingsError::IoError(
                format!("Failed to create settings file {path}"),
                e,
            )
        })?;
        
        f.write_all(s.as_bytes()).map_err(|e| {
            SettingsError::IoError(
                format!("Failed to write settings to file {path}"),
                e,
            )
        })?;
        
        Ok(())
    }

    fn as_string(&self) -> String {
        serde_yaml::to_string(&self).unwrap_or_else(|_| String::new())
    }
}

impl Loadable for Settings {}
