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

fn default_irc_server() -> String {
    "irc.ppy.sh".into()
}

fn default_irc_ping_timeout() -> u32 {
    40
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IRCChatSettings {
    pub username: String,
    pub password: String,

    #[serde(default = "default_irc_server")]
    pub server: String,

    #[serde(default = "default_irc_ping_timeout")]
    pub ping_timeout: u32,
}

impl Default for IRCChatSettings {
    fn default() -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            server: default_irc_server(),
            ping_timeout: default_irc_ping_timeout(),
        }
    }
}

pub fn default_api_client_id() -> u64 {
    32234
}

fn default_api_client_id_string() -> String {
    default_api_client_id().to_string()
}

fn default_api_redirect_uri() -> String {
    "http://localhost:19181/auth".into()
}

fn default_api_ws_uri() -> String {
    "wss://notify.ppy.sh".into()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HTTPChatSettings {
    #[serde(default = "default_api_client_id_string")]
    pub client_id: String,

    #[serde(default = "String::new")]
    pub client_secret: String,

    #[serde(default = "default_api_redirect_uri")]
    pub redirect_uri: String,

    #[serde(default = "default_api_ws_uri")]
    pub ws_base_uri: String,
}

impl Default for HTTPChatSettings {
    fn default() -> Self {
        Self {
            client_id: default_api_client_id_string(),
            client_secret: String::new(),
            redirect_uri: default_api_redirect_uri(),
            ws_base_uri: default_api_ws_uri(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatBehaviour {
    pub handle_osu_chat_links: bool,

    #[serde(default)]
    pub handle_osu_beatmap_links: bool,

    #[serde(default)]
    pub chat_position: ChatPosition,

    #[serde(default = "default_true")]
    pub track_unread_messages: bool,

    #[serde(default = "default_true")]
    pub keep_focus_on_input: bool,
}

impl Default for ChatBehaviour {
    fn default() -> Self {
        Self {
            handle_osu_chat_links: true,
            handle_osu_beatmap_links: true,
            chat_position: ChatPosition::Bottom,
            track_unread_messages: true,
            keep_focus_on_input: true,
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
