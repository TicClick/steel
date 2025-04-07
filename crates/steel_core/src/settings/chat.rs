use std::fmt::Display;

use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chat {
    pub backend: ChatBackend,
    pub autoconnect: bool,
    #[serde(default)]
    pub reconnect: bool,
    pub autojoin: Vec<String>,
    pub irc: IRCChatSettings,
    pub api: HTTPChatSettings,

    #[serde(default)]
    pub behaviour: ChatBehaviour,
}

impl Default for Chat {
    fn default() -> Self {
        Self {
            backend: ChatBackend::default(),
            autoconnect: false,
            reconnect: true,
            autojoin: Vec::default(),
            irc: IRCChatSettings::default(),
            api: HTTPChatSettings::default(),

            behaviour: ChatBehaviour::default(),
        }
    }
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatBehaviour {
    pub handle_osu_chat_links: bool,

    #[serde(default)]
    pub handle_osu_beatmap_links: bool,

    #[serde(default)]
    pub chat_position: ChatPosition,

    #[serde(default = "default_true")]
    pub track_unread_messages: bool,
}

impl Default for ChatBehaviour {
    fn default() -> Self {
        Self {
            handle_osu_chat_links: true,
            handle_osu_beatmap_links: true,
            chat_position: ChatPosition::Bottom,
            track_unread_messages: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub enum ChatPosition {
    Top,

    #[default]
    Bottom,
}

impl Display for ChatPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ChatPosition::Top => "top",
                ChatPosition::Bottom => "bottom (osu! style)",
            }
        )
    }
}
