pub mod irc;
pub mod links;

use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;

use super::DATETIME_FORMAT_WITH_TZ;
pub use links::MessageChunk;

#[derive(Clone, Debug, Hash)]
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

#[derive(Clone, Debug, Hash)]
pub struct Message {
    pub time: chrono::DateTime<chrono::Local>,
    pub r#type: MessageType,
    pub username: String,
    pub text: String,

    // Cached lowercase versions for performance
    pub username_lowercase: String,
    pub text_lowercase: String,

    // Chat-oriented metadata, which is only used by UI.
    pub chunks: Option<Vec<MessageChunk>>,
    pub id: Option<usize>,
    pub highlight: bool,

    pub original_chat: Option<String>,
}

#[derive(Clone, Debug, Hash, PartialEq)]
pub enum ChatType {
    Channel,
    Person,
    System,
}

impl fmt::Display for ChatType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ChatType::Channel => "channel",
                ChatType::Person => "person",
                ChatType::System => "system",
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
            username_lowercase: username.to_lowercase(),
            text_lowercase: text.to_lowercase(),

            chunks: None,
            id: None,
            highlight: false,

            original_chat: None,
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

    pub fn set_original_chat(&mut self, origin: &str) {
        self.original_chat = Some(origin.to_owned());
    }

    pub fn detect_highlights(&mut self, keywords: &HashSet<String>, username: Option<&String>) {
        let full_message_text = self.text_lowercase.trim();

        let mut kw: HashSet<String> = HashSet::new();
        let keywords = if let Some(u) = username {
            kw = keywords.clone();
            kw.insert(u.to_lowercase());
            &kw
        } else {
            keywords
        };

        'iterate_over_keywords: for keyword in keywords {
            let mut starting_pos = 0;
            while starting_pos < full_message_text.len() {
                let message_substring = &full_message_text[starting_pos..];
                if let Some(keyword_start_pos) = message_substring.find(keyword) {
                    if Self::highlight_found(
                        full_message_text,
                        keyword,
                        keyword_start_pos + starting_pos,
                    ) {
                        self.highlight = true;
                        break 'iterate_over_keywords;
                    } else {
                        starting_pos += keyword_start_pos + 1;
                        while !full_message_text.is_char_boundary(starting_pos) {
                            starting_pos += 1;
                        }
                    }
                } else {
                    continue 'iterate_over_keywords;
                }
            }
        }
    }

    fn highlight_found(text: &str, keyword: &str, keyword_start_pos: usize) -> bool {
        let is_message_prefix_matched = keyword_start_pos == 0;
        let is_keyword_prefix_alphanumeric = keyword.starts_with(|ch: char| ch.is_alphanumeric());
        let is_left_end_alphanumeric = keyword_start_pos > 0
            && text[..keyword_start_pos].ends_with(|ch: char| ch.is_alphanumeric());

        let keyword_end_pos = keyword_start_pos + keyword.len();
        let is_message_suffix_matched = keyword_end_pos == text.len();
        let is_keyword_suffix_alphanumeric = keyword.ends_with(|ch: char| ch.is_alphanumeric());
        let is_right_end_alphanumeric =
            text[keyword_end_pos..].starts_with(|ch: char| ch.is_alphanumeric());

        (is_message_prefix_matched || !is_keyword_prefix_alphanumeric || !is_left_end_alphanumeric)
            && (is_message_suffix_matched
                || !is_keyword_suffix_alphanumeric
                || !is_right_end_alphanumeric)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Hash)]
pub enum ChatState {
    #[default]
    Left,
    JoinInProgress,
    Joined,
    Disconnected,
}

#[derive(Debug)]
pub enum TabState {
    Read,
    Unread,
    Highlight,
}

#[derive(Clone, Debug, Hash)]
pub struct Chat {
    pub name: String,
    pub normalized_name: String,
    pub messages: Box<Vec<Message>>,
    pub state: ChatState,
    pub category: ChatType,

    pub unread_pointer: usize,
    pub prev_unread_pointer: usize,
    pub highlights: Vec<usize>,
}

impl Chat {
    pub fn new(name: &str) -> Self {
        Self {
            name: match name.strip_prefix('$') {
                Some(trimmed) => trimmed.to_owned(),
                None => name.to_owned(),
            },
            normalized_name: name.to_lowercase(),
            messages: Box::new(Vec::new()),
            state: ChatState::Left,
            category: name.chat_type(),
            unread_pointer: 0,
            prev_unread_pointer: 0,
            highlights: Vec::new(),
        }
    }

    pub fn push(&mut self, msg: Message, is_chat_active: bool) {
        let idx = self.messages.len();
        let is_highlight = msg.highlight;

        self.messages.push(msg);

        if is_highlight && self.category != ChatType::System {
            self.highlights.push(idx)
        }
        if is_chat_active {
            if self.unread_pointer == self.prev_unread_pointer {
                self.prev_unread_pointer += 1;
            }
            self.unread_pointer += 1;
        }
    }

    pub fn set_state(&mut self, state: ChatState) {
        self.state = state;
    }

    pub fn tab_state(&self) -> TabState {
        if self.unread_pointer == self.messages.len() {
            TabState::Read
        } else {
            match self.highlights.last() {
                None => TabState::Unread,
                Some(last_highlight) => {
                    if *last_highlight >= self.unread_pointer {
                        TabState::Highlight
                    } else {
                        TabState::Unread
                    }
                }
            }
        }
    }

    pub fn mark_as_read(&mut self) {
        self.prev_unread_pointer = self.unread_pointer;
        self.unread_pointer = self.messages.len();
    }

    pub fn clear(&mut self) {
        self.unread_pointer = 0;
        self.messages.clear();
        self.highlights.clear();
    }
}

pub trait ChatLike {
    fn chat_type(&self) -> ChatType;
    fn is_channel(&self) -> bool;
    fn is_person(&self) -> bool;
    fn is_system(&self) -> bool;
}

impl ChatLike for &str {
    fn chat_type(&self) -> ChatType {
        if self.starts_with('#') {
            ChatType::Channel
        } else if self.starts_with('$') {
            ChatType::System
        } else {
            ChatType::Person
        }
    }

    fn is_channel(&self) -> bool {
        matches!(self.chat_type(), ChatType::Channel)
    }

    fn is_person(&self) -> bool {
        matches!(self.chat_type(), ChatType::Person)
    }

    fn is_system(&self) -> bool {
        matches!(self.chat_type(), ChatType::System)
    }
}

impl ChatLike for String {
    fn chat_type(&self) -> ChatType {
        self.as_str().chat_type()
    }

    fn is_channel(&self) -> bool {
        self.as_str().is_channel()
    }

    fn is_person(&self) -> bool {
        self.as_str().is_person()
    }

    fn is_system(&self) -> bool {
        self.as_str().is_system()
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

    fn hls(words: &[&str]) -> HashSet<String> {
        HashSet::from_iter(words.iter().map(|w| w.to_string()))
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
            ("what do ты have to offer?", vec!["ты"], None),

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
            ("раз,два,три!четыре", vec!["три"], None),

            // Username in a message.
            ("oliver twist, c'mere boy!", vec![], Some(&"Oliver".to_string())),

            // Case-insensitive matching.
            ("over the rainbow", vec!["OVER"], None),
            ("ЗАПРЕЩЕНО ЗАПРЕЩАТЬ", vec!["запрещено"], None),

            // Several words in a highlight.
            ("jackdaws love my big sphinx of quartz", vec!["jackdaws love"], None),
            ("jackdaws love my big sphinx of quartz", vec!["love my"], None),
            ("jackdaws love my big sphinx of quartz", vec!["sphinx of quartz"], None),
            ("Белая гвардия, белый снег, белая музыка революций", vec!["белый снег"], None),

            // Punctuation in a highlight.
            ("Players of.the.world, unite!", vec!["of.the.world"], None),
            ("?of.the.world!", vec!["of.the.world"], None),
            ("the match has finished!", vec!["finished!"], None),

            // Several highlights, only one matches.
            ("the match has finished!", vec!["no", "one", "has", "lived", "forever"], None),

            // Several occurrences, but only a standalone word should match.
            ("airlock is sealed against air", vec!["air"], None),
            ("одна сорока, да ещё сорок сороко̀в соро̀к", vec!["сорок"], None),
        ] {
            let mut message = Message::new_text("Someone", message_text);
            message.detect_highlights(&hls(&keywords), active_username);
            assert!(message.highlight, "{message_text:?} did not match {keywords:?}");
        }
    }

    #[test]
    fn negative_highlights() {
        for (message_text, keywords, active_username) in [

            // Substrings of a single word.
            ("jackdaws love my big sphinx of quartz", vec!["jack"], None),
            ("jackdaws love my big sphinx of quartz", vec!["aws"], None),
            ("jackdaws love my big sphinx of quartz", vec!["phi"], None),
            ("jackdaws love my big sphinx of quartz", vec!["artz"], None),

            // Unicode.
            ("он посмотрел видео", vec!["вид"], None),
            ("я не курю", vec!["рю"], None),

            // Punctuation must match.
            ("clickers(of.the.world)unite", vec![".of.the.world."], None),
        ] {
            let mut message = Message::new_text("Someone", message_text);
            message.detect_highlights(&hls(&keywords), active_username);
            assert!(!message.highlight, "{message_text:?} matched {keywords:?} (it shouldn't have)");
        }
    }
}
