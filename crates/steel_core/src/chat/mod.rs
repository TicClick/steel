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
        let text = self.text.to_lowercase();
        let text = text.trim();
        let keywords = if let Some(u) = username {
            let mut kw: BTreeSet<String> = keywords.into_iter().map(|s| s.to_lowercase()).collect();
            kw.insert(u.to_lowercase());
            kw
        } else {
            keywords.into_iter().map(|s| s.to_lowercase()).collect()
        };

        for keyword in &keywords {
            if let Some(keyword_start_pos) = text.find(keyword) {
                let is_message_prefix_matched = keyword_start_pos == 0;
                let is_keyword_prefix_alphanumeric =
                    keyword.starts_with(|ch: char| ch.is_alphanumeric());
                let is_left_end_alphanumeric = keyword_start_pos > 0 && {
                    let previous_byte: char = text.as_bytes()[keyword_start_pos - 1] as char;
                    previous_byte.is_alphanumeric()
                };

                let keyword_end_pos = keyword_start_pos + keyword.len();
                let is_message_suffix_matched = keyword_end_pos == text.len();
                let is_keyword_suffix_alphanumeric =
                    keyword.ends_with(|ch: char| ch.is_alphanumeric());
                let is_right_end_alphanumeric = keyword_end_pos < text.len() && {
                    let next_byte: char = text.as_bytes()[keyword_end_pos] as char;
                    next_byte.is_alphanumeric()
                };

                if (is_message_prefix_matched
                    || !is_keyword_prefix_alphanumeric
                    || !is_left_end_alphanumeric)
                    && (is_message_suffix_matched
                        || !is_keyword_suffix_alphanumeric
                        || !is_right_end_alphanumeric)
                {
                    self.highlight = true;
                    break;
                }
            }
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum ChatState {
    #[default]
    Left,
    JoinInProgress,
    Joined,
    Disconnected,
}

#[derive(Clone, Debug, Default)]
pub struct Chat {
    pub name: String,
    pub messages: Vec<Message>,
    pub state: ChatState,
}

impl Chat {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
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

    pub fn set_state(&mut self, state: ChatState) {
        self.state = state;
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

#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use super::*;

    fn hls(words: &[&str]) -> BTreeSet<String> {
        BTreeSet::from_iter(words.iter().map(|w| w.to_string()))
    }

    #[test]
    fn positive_highlights() {
        for (message_text, keywords, active_username) in [

            // One-word highlight, space delimiters.
            ("fullmatch", vec!["fullmatch"], None),
            ("apples and oranges", vec!["apples"], None),
            ("apples and oranges", vec!["and"], None),
            ("apples and oranges", vec!["oranges"], None),
            ("hell upside down is 1134", vec!["1134"], None),

            // One-word highlight, message contains punctuation.
            ("apples,and!oranges#are[both]fruits??im_telling(you)so..", vec!["apples"], None),
            ("apples,and!oranges#are[both]fruits??im_telling(you)so..", vec!["and"], None),
            ("apples,and!oranges#are[both]fruits??im_telling(you)so..", vec!["oranges"], None),
            ("apples,and!oranges#are[both]fruits??im_telling(you)so..", vec!["are"], None),
            ("apples,and!oranges#are[both]fruits??im_telling(you)so..", vec!["both"], None),
            ("apples,and!oranges#are[both]fruits??im_telling(you)so..", vec!["fruits"], None),
            ("apples,and!oranges#are[both]fruits??im_telling(you)so..", vec!["im"], None),
            ("apples,and!oranges#are[both]fruits??im_telling(you)so..", vec!["telling"], None),
            ("apples,and!oranges#are[both]fruits??im_telling(you)so..", vec!["you"], None),
            ("apples,and!oranges#are[both]fruits??im_telling(you)so..", vec!["so"], None),

            // Username in a message.
            ("oliver twist, c'mere boy!", vec![], Some(&"Oliver".to_string())),

            // Case-insensitive matching.
            ("over the rainbow", vec!["OVER"], None),

            // Several words in a highlight.
            ("jackdaws love my big sphinx of quartz", vec!["jackdaws love"], None),
            ("jackdaws love my big sphinx of quartz", vec!["love my"], None),
            ("jackdaws love my big sphinx of quartz", vec!["sphinx of quartz"], None),

            // Punctuation in a highlight.
            ("Players of.the.world, unite!", vec!["of.the.world"], None),
            ("?of.the.world!", vec!["of.the.world"], None),
            ("the match has finished!", vec!["finished!"], None),

            // Several highlights, only one matches.
            ("the match has finished!", vec!["no", "one", "has", "lived", "forever"], None),
        ] {
            let mut message = Message::new_text("Someone", message_text);
            message.detect_highlights(&hls(&keywords), active_username);
            assert!(message.highlight, "{:?} did not match {:?}", message_text, keywords);
        }
    }

    #[test]
    fn negative_highlights() {
        for (message_text, keywords, active_username) in [

            // Substrings of a single word.
            ("jackdaws love my big sphinx of quartz", vec!["jack"], None),
            ("jackdaws love my big sphinx of quartz", vec!["aws"], None),
            ("jackdaws love my big sphinx of quartz", vec!["phi"], None),

            // Punctuation must match.
            ("clickers(of.the.world)unite", vec![".of.the.world."], None),
        ] {
            let mut message = Message::new_text("Someone", message_text);
            message.detect_highlights(&hls(&keywords), active_username);
            assert!(!message.highlight, "{:?} matched {:?} (it shouldn't have)", message_text, keywords);
        }
    }
}
