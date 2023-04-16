pub mod chat;
pub mod colour;
pub mod journal;
pub mod notifications;
pub mod ui;

use std::env;
use std::io::Write;

use serde::{Deserialize, Serialize};
use serde_yaml;

pub use chat::{Chat, ChatBackend, HTTPChatSettings, IRCChatSettings};
pub use colour::Colour;
pub use journal::{AppEvents, Journal};
pub use notifications::{BuiltInSound, Highlights, Notifications, Sound};
pub use ui::{ChatColours, ThemeMode, UI};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    #[serde(skip)]
    settings_path: Source,

    pub chat: Chat,
    pub notifications: Notifications,
    pub ui: UI,
    pub journal: Journal,
}

impl Settings {
    pub fn from_file(source: &Source, fallback: bool) -> Self {
        let path = source.expand();
        match std::fs::read_to_string(&path) {
            Ok(contents) => match serde_yaml::from_str::<Settings>(contents.as_str()) {
                Ok(mut obj) => {
                    obj.settings_path = source.to_owned();
                    obj
                }
                Err(e) => {
                    panic!("Error while loading the config: {}", e);
                }
            },
            Err(e) => {
                if fallback {
                    return Self::default();
                }
                panic!("Error reading file at {}: {}", path, e);
            }
        }
    }

    pub fn to_file(&self, path: &Source) {
        let p = path.expand();
        match serde_yaml::to_string(self) {
            Ok(s) => match std::fs::File::create(&p) {
                Ok(mut f) => {
                    if f.write(s.as_bytes()).is_err() {
                        panic!("Failed to save settings")
                    }
                }
                Err(e) => {
                    panic!("Failed to create the file at {}: {}", p, e);
                }
            },
            Err(e) => {
                panic!("Error saving config: {}", e);
            }
        }
    }

    pub fn save(&self) {
        self.to_file(&self.settings_path);
    }

    pub fn reload(&mut self) {
        *self = Self::from_file(&self.settings_path, false);
    }
}

const DEFAULT_FILE_NAME: &str = "settings.yaml";

#[derive(Clone, Debug, Default)]
pub enum Source {
    #[default]
    DefaultPath,
    CustomPath(String),
}

impl Source {
    pub fn expand(&self) -> String {
        match self {
            Source::DefaultPath => match env::current_dir() {
                Ok(p) => p.join(DEFAULT_FILE_NAME).display().to_string(),
                Err(e) => {
                    panic!(
                        "Failed to read current directory while looking for {}: {}",
                        DEFAULT_FILE_NAME, e
                    )
                }
            },
            Source::CustomPath(p) => p.to_owned(),
        }
    }
}
