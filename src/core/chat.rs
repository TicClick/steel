use std::fmt;

use super::{DEFAULT_DATE_FORMAT, DEFAULT_TIME_FORMAT};

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
}

#[derive(Clone, Debug, Hash)]
pub enum ChatType {
    Channel,
    Person,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.time.format(DEFAULT_DATE_FORMAT)).and_then(|_| match self.r#type {
            MessageType::Text => {
                write!(f, " <{}> {}", self.username, self.text)
            }
            MessageType::Action => {
                write!(f, " * {} {}", self.username, self.text)
            }
            MessageType::System => {
                write!(f, " * {}", self.text)
            }
        })
    }
}

#[derive(PartialEq, Debug)]
pub enum LinkLocation {
    Raw {
        pos: (usize, usize),
    },
    Markdown {
        pos: (usize, usize),
        title: (usize, usize),
        location: (usize, usize),
    },
    Wiki {
        pos: (usize, usize),
        title: (usize, usize),
    },
}

impl LinkLocation {
    pub fn position(&self) -> &(usize, usize) {
        match self {
            Self::Raw { pos } | Self::Markdown { pos, .. } | Self::Wiki { pos, .. } => pos,
        }
    }

    pub fn title(&self, s: &str) -> String {
        match self {
            Self::Raw { pos } => s[pos.0..pos.1].to_owned(),
            Self::Wiki { title, .. } => format!("wiki:{}", &s[title.0..title.1]),
            Self::Markdown { title, .. } => s[title.0..title.1].to_owned(),
        }
    }

    pub fn location(&self, s: &str) -> String {
        match self {
            Self::Raw { pos } => s[pos.0..pos.1].to_owned(),
            Self::Wiki { title, .. } => format!("https://osu.ppy.sh/wiki/{}", &s[title.0..title.1]),
            Self::Markdown { location, .. } => s[location.0..location.1].to_owned(),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum MessageChunk {
    Text(String),
    Link { title: String, location: String },
}

impl Message {
    pub fn new(username: &str, text: &str, r#type: MessageType) -> Self {
        Self {
            time: chrono::Local::now(),
            r#type,
            username: username.to_string(),
            text: text.to_string(),
        }
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

    pub fn chunked_text(&self) -> Option<Vec<MessageChunk>> {
        let mut ret: Vec<MessageChunk> = Vec::new();
        let mut links: Vec<LinkLocation> = Vec::new();

        let mut i = 0;
        let bs = self.text.as_bytes();
        while i < bs.len() {
            // Only consider [[...]], [...], or http(s)://. Yeah, I know there are other protocols and formats, but no.
            while i < bs.len() && (bs[i] != b'[' && bs[i] != b'h') {
                i += 1;
            }
            if i == bs.len() {
                break;
            }

            let start = i;

            // HTTP(s), no title.
            if (i + 7 < bs.len() && &bs[i..i + 7] == "http://".as_bytes())
                || (i + 8 < bs.len() && &bs[i..i + 8] == "https://".as_bytes())
            {
                while i < bs.len() && bs[i] != b' ' {
                    i += 1;
                }
                links.push(LinkLocation::Raw { pos: (start, i) });
                continue;
            }

            // Wiki link.
            if i + 1 < bs.len() && bs[i + 1] == b'[' {
                while i < bs.len() && bs[i] != b']' {
                    i += 1;
                }
                if i + 1 < bs.len() && bs[i + 1] == b']' {
                    links.push(LinkLocation::Wiki {
                        pos: (start, i + 2),
                        title: (start + 2, i),
                    });
                } else {
                    // Reset failed state and see what the next loop iteration will bring.
                    // FIXME: Wow, this makes it quadratic -- I guess I should use KMP or a suffix tree once it becomes an issue?
                    i = start + 1;
                }
                continue;
            }

            // Link with title
            if ((i + 1) + 7 < bs.len() && &bs[(i + 1)..(i + 1) + 7] == "http://".as_bytes())
                || ((i + 1) + 8 < bs.len() && &bs[(i + 1)..(i + 1) + 8] == "https://".as_bytes())
            {
                // Extract the location.
                let location_start = i + 1;
                while i < bs.len() && bs[i] != b' ' {
                    i += 1;
                }
                let location_end = i;
                if i < bs.len() && bs[i] == b' ' {
                    i += 1;
                    let title_start = i;
                    // Find the end of the link.
                    while i < bs.len() && bs[i] != b']' {
                        i += 1;
                    }
                    if i < bs.len() {
                        // Catch all trailing closing brackets, accounting for difficulty names in /np.
                        while i < bs.len() && bs[i] == b']' {
                            i += 1;
                        }
                        let title_end = i - 1;
                        let end = i;
                        links.push(LinkLocation::Markdown {
                            pos: (start, end),
                            title: (title_start, title_end),
                            location: (location_start, location_end),
                        });
                        continue;
                    } else {
                        // Reset failed state and see what the next loop iteration will bring.
                        i = start + 1;
                    }
                } else {
                    // Reset failed state and see what the next loop iteration will bring.
                    i = start + 1;
                }
                continue;
            }

            // None of the above matched and completed the link -- advance to the next byte.
            i += 1;
        }

        if links.is_empty() {
            return None;
        }

        for i in 0..links.len() {
            let pos = links[i].position();
            if i == 0 && pos.0 > 0 {
                ret.push(MessageChunk::Text(self.text[0..pos.0].to_owned()));
            }

            ret.push(MessageChunk::Link {
                title: links[i].title(&self.text),
                location: links[i].location(&self.text),
            });
            if i + 1 < links.len() {
                let next_pos = links[i + 1].position();
                if pos.1 < next_pos.0 {
                    ret.push(MessageChunk::Text(self.text[pos.1..next_pos.0].to_owned()));
                }
            }
        }
        let last_pos = links.last().unwrap().position();
        if last_pos.1 < self.text.len() {
            ret.push(MessageChunk::Text(
                self.text[last_pos.1..self.text.len()].to_owned(),
            ));
        }
        Some(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(s: &str) -> Message {
        Message {
            time: chrono::Local::now(),
            r#type: MessageType::Text,
            username: "abc".into(),
            text: s.into(),
        }
    }

    #[test]
    fn no_links() {
        let message = m("Test (no links here)");
        assert!(message.chunked_text().is_none());
    }

    #[test]
    fn simple_markdown() {
        let message = m("[http://test Test (links here)]]");
        assert_eq!(
            message.chunked_text().unwrap(),
            vec![MessageChunk::Link {
                location: "http://test".into(),
                title: "Test (links here)]".into()
            }]
        );

        let message = m("[http://test Test (links here)");
        assert_eq!(
            message.chunked_text().unwrap(),
            vec![
                MessageChunk::Text("[".into()),
                MessageChunk::Link {
                    location: "http://test".into(),
                    title: "http://test".into()
                },
                MessageChunk::Text(" Test (links here)".into()),
            ]
        );
    }

    #[test]
    fn two_markdown_links() {
        let message = m("[http://test Test (links here)] [http://test Test (links here)]");
        assert_eq!(
            message.chunked_text().unwrap(),
            vec![
                MessageChunk::Link {
                    location: "http://test".into(),
                    title: "Test (links here)".into()
                },
                MessageChunk::Text(" ".into()),
                MessageChunk::Link {
                    location: "http://test".into(),
                    title: "Test (links here)".into()
                }
            ]
        );

        let message = m("[http://test Test (links here)][http://test Test (links here)] and after");
        assert_eq!(
            message.chunked_text().unwrap(),
            vec![
                MessageChunk::Link {
                    location: "http://test".into(),
                    title: "Test (links here)".into()
                },
                MessageChunk::Link {
                    location: "http://test".into(),
                    title: "Test (links here)".into()
                },
                MessageChunk::Text(" and after".into()),
            ]
        );
    }

    #[test]
    fn wiki() {
        let message = m("[[rules]] is the way to go");
        assert_eq!(
            message.chunked_text().unwrap(),
            vec![
                MessageChunk::Link {
                    location: "https://osu.ppy.sh/wiki/rules".into(),
                    title: "wiki:rules".into()
                },
                MessageChunk::Text(" is the way to go".into()),
            ]
        );

        let message = m("[[rule]]s]] is the way to go");
        assert_eq!(
            message.chunked_text().unwrap(),
            vec![
                MessageChunk::Link {
                    location: "https://osu.ppy.sh/wiki/rule".into(),
                    title: "wiki:rule".into()
                },
                MessageChunk::Text("s]] is the way to go".into()),
            ]
        );
    }

    #[test]
    fn raw() {
        let message = m("https://a https://bhttps:// c");
        assert_eq!(
            message.chunked_text().unwrap(),
            vec![
                MessageChunk::Link {
                    location: "https://a".into(),
                    title: "https://a".into()
                },
                MessageChunk::Text(" ".into()),
                MessageChunk::Link {
                    location: "https://bhttps://".into(),
                    title: "https://bhttps://".into()
                },
                MessageChunk::Text(" c".into()),
            ]
        );
    }

    #[test]
    fn multiple() {
        let message = m("https://ya.ru [http://example.com example][[silence]]");
        assert_eq!(
            message.chunked_text().unwrap(),
            vec![
                MessageChunk::Link {
                    location: "https://ya.ru".into(),
                    title: "https://ya.ru".into()
                },
                MessageChunk::Text(" ".into()),
                MessageChunk::Link {
                    location: "http://example.com".into(),
                    title: "example".into()
                },
                MessageChunk::Link {
                    location: "https://osu.ppy.sh/wiki/silence".into(),
                    title: "wiki:silence".into()
                },
            ]
        );
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
