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
