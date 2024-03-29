use serde::{Deserialize, Serialize};

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
}

impl Default for ChatBehaviour {
    fn default() -> Self {
        Self {
            handle_osu_chat_links: true,
            handle_osu_beatmap_links: true,
        }
    }
}
