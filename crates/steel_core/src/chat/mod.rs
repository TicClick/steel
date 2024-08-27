pub mod irc;
pub mod links;

use std::collections::BTreeSet;
use std::fmt;

use super::{DATETIME_FORMAT_WITH_TZ, DEFAULT_DATETIME_FORMAT, DEFAULT_TIME_FORMAT};
pub use links::MessageChunk;

#[derive(Clone, Debug)]
pub enum MessageType {
    Text,
    Action,
    System,
}

#[derive(Clone, Debug)]
pub struct User {
    pub id: i32,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct Message {
    pub time: chrono::DateTime<chrono::Local>,
    pub r#type: MessageType,
    pub username: String,
    pub text: String,

    // Chat-oriented metadata, which is only used by UI.
    pub chunks: Option<Vec<MessageChunk>>,
    pub id: Option<usize>,
    pub highlight: bool,
}

#[derive(Clone, Debug, Hash)]
pub enum ChatType {
    Channel,
    Person,
}

impl fmt::Display for ChatType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ChatType::Channel => "channel",
                ChatType::Person => "person",
            }
        )
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.time.format(DATETIME_FORMAT_WITH_TZ)).and_then(|_| match self.r#type {
            MessageType::Text => {
                write!(f, " <{}> {}", self.username, self.text)
            }
            MessageType::Action => {
                write!(f, " * {} {}", self.username, self.text)
            }
            MessageType::System => {
                write!(f, " {}", self.text)
            }
        })
    }
}

impl Message {
    pub fn new(username: &str, text: &str, r#type: MessageType) -> Self {
        Self {
            time: chrono::Local::now(),
            r#type,
            username: username.to_string(),
            text: text.to_string(),

            chunks: None,
            id: None,
            highlight: false,
        }
    }

    pub fn with_time(mut self, dt: chrono::DateTime<chrono::Local>) -> Self {
        self.time = dt;
        self
    }

    pub fn new_text(username: &str, text: &str) -> Self {
        Self::new(username, text, MessageType::Text)
    }

    pub fn new_action(username: &str, text: &str) -> Self {
        Self::new(username, text, MessageType::Action)
    }

    pub fn new_system(text: &str) -> Self {
        Self::new("", text, MessageType::System)
    }

    pub fn formatted_time(&self) -> String {
        self.time.format(DEFAULT_TIME_FORMAT).to_string()
    }

    pub fn formatted_date_local(&self) -> String {
        self.time.format(DEFAULT_DATETIME_FORMAT).to_string()
    }

    pub fn formatted_date_utc(&self) -> String {
        self.time
            .naive_utc()
            .format(DEFAULT_DATETIME_FORMAT)
            .to_string()
    }

    pub fn detect_highlights(&mut self, keywords: &BTreeSet<String>, username: Option<&String>) {
        let separator = "$$";
        let wrap_with_separators = |v| format!("{separator}{v}{separator}");

        let normalized_text = self
            .text
            .to_lowercase()
            .split(|ch: char| {
                ch.is_whitespace() || (ch.is_ascii_punctuation() && !matches!(ch, '[' | ']' | '#'))
            })
            .collect::<Vec<&str>>()
            .join(separator);
        let normalized_text = wrap_with_separators(normalized_text);

        for keyword in keywords.iter().filter(|k| !k.is_empty()) {
            let keyword = wrap_with_separators(keyword.replace(' ', separator));
            if normalized_text.contains(&keyword) {
                self.highlight = true;
                break;
            }
        }

        if let Some(u) = username {
            self.highlight =
                self.highlight || normalized_text.contains(&wrap_with_separators(u.to_lowercase()));
        }
    }
}

#[derive(Clone, Debug, Default)]
pub enum ChatState {
    #[default]
    Left,
    JoinInProgress,
    Joined,
}

#[derive(Clone, Debug, Default)]
pub struct Chat {
    pub name: String,
    pub messages: Vec<Message>,
    pub state: ChatState,
}

impl Chat {
    pub fn new(name: String) -> Self {
        Self {
            name,
            messages: Vec::new(),
            state: ChatState::Left,
        }
    }

    pub fn push(&mut self, msg: Message) {
        self.messages.push(msg);
    }

    pub fn r#type(&self) -> ChatType {
        self.name.chat_type()
    }

    pub fn set_state(&mut self, state: ChatState, reason: Option<&str>) {
        self.state = state;
        if let Some(reason) = reason {
            self.push(Message::new_system(reason));
        }
    }
}

pub trait ChatLike {
    fn is_channel(&self) -> bool;
    fn chat_type(&self) -> ChatType;
}

impl ChatLike for &str {
    fn is_channel(&self) -> bool {
        self.starts_with('#')
    }

    fn chat_type(&self) -> ChatType {
        if self.is_channel() {
            ChatType::Channel
        } else {
            ChatType::Person
        }
    }
}

impl ChatLike for String {
    fn is_channel(&self) -> bool {
        self.as_str().is_channel()
    }

    fn chat_type(&self) -> ChatType {
        self.as_str().chat_type()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ConnectionStatus {
    Disconnected { by_user: bool },
    InProgress,
    Connected,
    Scheduled(chrono::DateTime<chrono::Local>),
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self::Disconnected { by_user: false }
    }
}

impl fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Connected => "connected".into(),
                Self::InProgress => "connecting".into(),
                Self::Disconnected { .. } => "disconnected".into(),
                Self::Scheduled(when) => format!("connecting in {}s", *when - chrono::Local::now()),
            }
        )
    }
}
