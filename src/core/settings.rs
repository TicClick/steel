use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Display;
use std::io::Write;

use eframe::egui::Color32;
use serde::{Deserialize, Serialize};
use serde_yaml;

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

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Notifications {
    pub highlights: Highlights,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Highlights {
    pub colour: Colour,
    pub words: Vec<String>,
    pub sound: Option<Sound>,
}

impl Default for Highlights {
    fn default() -> Self {
        Self {
            colour: Colour::from_rgb(250, 200, 255),
            words: Vec::default(),
            sound: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Sound {
    Coin,
    PartyHorn,
    Bleep,
}

impl Display for Sound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Coin => "Coin",
                Self::PartyHorn => "Party horn",
                Self::Bleep => "Bleep",
            }
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Chat {
    pub backend: ChatBackend,
    pub autoconnect: bool,
    pub autojoin: BTreeSet<String>,
    pub irc: IRCChatSettings,
    pub api: HTTPChatSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatBackend {
    #[default]
    IRC,
    API,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct IRCChatSettings {
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct HTTPChatSettings {}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(into = "String")]
#[serde(from = "String")]
pub struct Colour {
    pub rgb: [u8; 3],
}

impl Colour {
    pub fn as_u8(&mut self) -> &mut [u8; 3] {
        &mut self.rgb
    }
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { rgb: [r, g, b] }
    }
}

impl From<Colour> for Color32 {
    fn from(val: Colour) -> Self {
        Color32::from_rgb(val.rgb[0], val.rgb[1], val.rgb[2])
    }
}

impl From<Colour> for String {
    fn from(val: Colour) -> Self {
        format!("{} {} {}", val.rgb[0], val.rgb[1], val.rgb[2])
    }
}

impl From<String> for Colour {
    fn from(value: String) -> Self {
        let values: Vec<u8> = value
            .split_ascii_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        match values[0..3].try_into() {
            Ok(rgb) => Self { rgb },
            Err(e) => panic!(
                "invalid colour value {} (must have 3 elements): {}",
                value, e
            ),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ChatColours {
    pub own: Colour,
    pub users: BTreeMap<String, Colour>,
}

impl Default for ChatColours {
    fn default() -> Self {
        Self {
            own: Colour::from_rgb(200, 255, 250),
            users: BTreeMap::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct UI {
    pub theme: ThemeMode,
    pub colours: ChatColours,
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
