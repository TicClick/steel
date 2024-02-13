use crate::chat::Message;

#[derive(Debug, PartialEq, Clone)]
pub enum ProtocolType {
    HTTP,
    HTTPS,
    OSU,
    OSUMP,
}

pub const PROTOCOL_HTTP: &str = "http://";
pub const PROTOCOL_HTTPS: &str = "https://";
pub const PROTOCOL_OSU: &str = "osu://";
pub const PROTOCOL_OSUMP: &str = "osump://";

pub const KNOWN_PROTOCOLS: [&str; 4] =
    [PROTOCOL_HTTP, PROTOCOL_HTTPS, PROTOCOL_OSU, PROTOCOL_OSUMP];

#[derive(PartialEq, Debug)]
pub enum LinkLocation {
    Raw {
        pos: (usize, usize),
        protocol: ProtocolType,
    },
    Markdown {
        pos: (usize, usize),
        title: (usize, usize),
        location: (usize, usize),
        protocol: ProtocolType,
    },
    Wiki {
        pos: (usize, usize),
        title: (usize, usize),
        protocol: ProtocolType,
    },
}

impl LinkLocation {
    pub fn position(&self) -> &(usize, usize) {
        match self {
            Self::Raw { pos, .. } | Self::Markdown { pos, .. } | Self::Wiki { pos, .. } => pos,
        }
    }

    pub fn protocol(&self) -> ProtocolType {
        match self {
            Self::Raw { protocol, .. }
            | Self::Markdown { protocol, .. }
            | Self::Wiki { protocol, .. } => protocol.clone(),
        }
    }

    pub fn title(&self, s: &str) -> String {
        match self {
            Self::Raw { pos, .. } => s[pos.0..pos.1].to_owned(),
            Self::Wiki { title, .. } => format!("wiki:{}", &s[title.0..title.1]),
            Self::Markdown { title, .. } => s[title.0..title.1].to_owned(),
        }
    }

    pub fn location(&self, s: &str) -> String {
        match self {
            Self::Raw { pos, .. } => s[pos.0..pos.1].to_owned(),
            Self::Wiki { title, .. } => format!("https://osu.ppy.sh/wiki/{}", &s[title.0..title.1]),
            Self::Markdown { location, .. } => s[location.0..location.1].to_owned(),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum MessageChunk {
    Text(String),
    Link {
        title: String,
        location: String,
        protocol: ProtocolType,
    },
}

impl Message {
    pub fn parse_for_links(&mut self) {
        let mut ret: Vec<MessageChunk> = Vec::new();
        let mut links: Vec<LinkLocation> = Vec::new();

        let mut i = 0;
        let bs = self.text.as_bytes();

        let protocol_found = |pos: usize| -> Option<ProtocolType> {
            for (proto_type, proto_repr) in [
                (ProtocolType::HTTP, PROTOCOL_HTTP),
                (ProtocolType::HTTPS, PROTOCOL_HTTPS),
                (ProtocolType::OSU, PROTOCOL_OSU),
                (ProtocolType::OSUMP, PROTOCOL_OSUMP),
            ] {
                if pos + proto_repr.len() < bs.len()
                    && &bs[pos..pos + proto_repr.len()] == proto_repr.as_bytes()
                {
                    return Some(proto_type);
                }
            }
            return None;
        };

        while i < bs.len() {
            // Only consider [[...]], [...], http(s)://, or osu(mp)://.
            // Yeah, I know there are other protocols and formats, but no.
            while i < bs.len() && (bs[i] != b'[' && bs[i] != b'h' && bs[i] != b'o') {
                i += 1;
            }
            if i == bs.len() {
                break;
            }

            let start = i;

            // Plain link starting with a protocol, no title.
            if let Some(protocol) = protocol_found(i) {
                while i < bs.len() && bs[i] != b' ' {
                    i += 1;
                }
                links.push(LinkLocation::Raw {
                    pos: (start, i),
                    protocol,
                });
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
                        protocol: ProtocolType::HTTPS,
                    });
                } else {
                    // Reset failed state and see what the next loop iteration will bring.
                    // FIXME: Wow, this makes it quadratic -- I guess I should use KMP or a suffix tree once it becomes an issue?
                    i = start + 1;
                }
                continue;
            }

            // Link with title
            if let Some(protocol) = protocol_found(i + 1) {
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
                            protocol,
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
            self.chunks = Some(vec![MessageChunk::Text(self.text.clone())]);
            return;
        }

        for i in 0..links.len() {
            let pos = links[i].position();
            if i == 0 && pos.0 > 0 {
                ret.push(MessageChunk::Text(self.text[0..pos.0].to_owned()));
            }

            ret.push(MessageChunk::Link {
                title: links[i].title(&self.text),
                location: links[i].location(&self.text),
                protocol: links[i].protocol(),
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
        self.chunks = Some(ret);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::MessageType;

    fn m(s: &str) -> Message {
        let mut m = Message {
            time: chrono::Local::now(),
            r#type: MessageType::Text,
            username: "abc".into(),
            text: s.into(),
            chunks: None,
            id: None,
            highlight: false,
        };
        m.parse_for_links();
        m
    }

    #[test]
    fn no_links() {
        let message = m("Test (no links here)");
        match message.chunks.unwrap().first().unwrap() {
            MessageChunk::Text(text) => assert_eq!(text, &message.text),
            _ => assert!(false),
        }
    }

    #[test]
    fn simple_markdown() {
        let message = m("[http://test Test (links here)]]");
        assert_eq!(
            message.chunks.unwrap(),
            vec![MessageChunk::Link {
                location: "http://test".into(),
                title: "Test (links here)]".into(),
                protocol: ProtocolType::HTTP,
            }]
        );

        let message = m("[http://test Test (links here)");
        assert_eq!(
            message.chunks.unwrap(),
            vec![
                MessageChunk::Text("[".into()),
                MessageChunk::Link {
                    location: "http://test".into(),
                    title: "http://test".into(),
                    protocol: ProtocolType::HTTP,
                },
                MessageChunk::Text(" Test (links here)".into()),
            ]
        );
    }

    #[test]
    fn two_markdown_links() {
        let message = m("[http://test Test (links here)] [http://test Test (links here)]");
        assert_eq!(
            message.chunks.unwrap(),
            vec![
                MessageChunk::Link {
                    location: "http://test".into(),
                    title: "Test (links here)".into(),
                    protocol: ProtocolType::HTTP,
                },
                MessageChunk::Text(" ".into()),
                MessageChunk::Link {
                    location: "http://test".into(),
                    title: "Test (links here)".into(),
                    protocol: ProtocolType::HTTP,
                }
            ]
        );

        let message = m("[http://test Test (links here)][http://test Test (links here)] and after");
        assert_eq!(
            message.chunks.unwrap(),
            vec![
                MessageChunk::Link {
                    location: "http://test".into(),
                    title: "Test (links here)".into(),
                    protocol: ProtocolType::HTTP,
                },
                MessageChunk::Link {
                    location: "http://test".into(),
                    title: "Test (links here)".into(),
                    protocol: ProtocolType::HTTP,
                },
                MessageChunk::Text(" and after".into()),
            ]
        );
    }

    #[test]
    fn wiki() {
        let message = m("[[rules]] is the way to go");
        assert_eq!(
            message.chunks.unwrap(),
            vec![
                MessageChunk::Link {
                    location: "https://osu.ppy.sh/wiki/rules".into(),
                    title: "wiki:rules".into(),
                    protocol: ProtocolType::HTTPS,
                },
                MessageChunk::Text(" is the way to go".into()),
            ]
        );

        let message = m("[[rule]]s]] is the way to go");
        assert_eq!(
            message.chunks.unwrap(),
            vec![
                MessageChunk::Link {
                    location: "https://osu.ppy.sh/wiki/rule".into(),
                    title: "wiki:rule".into(),
                    protocol: ProtocolType::HTTPS,
                },
                MessageChunk::Text("s]] is the way to go".into()),
            ]
        );
    }

    #[test]
    fn raw() {
        let message = m("https://a https://bhttps:// c");
        assert_eq!(
            message.chunks.unwrap(),
            vec![
                MessageChunk::Link {
                    location: "https://a".into(),
                    title: "https://a".into(),
                    protocol: ProtocolType::HTTPS,
                },
                MessageChunk::Text(" ".into()),
                MessageChunk::Link {
                    location: "https://bhttps://".into(),
                    title: "https://bhttps://".into(),
                    protocol: ProtocolType::HTTPS,
                },
                MessageChunk::Text(" c".into()),
            ]
        );
    }

    #[test]
    fn multiple() {
        let message = m("https://ya.ru [http://example.com example][[silence]]");
        assert_eq!(
            message.chunks.unwrap(),
            vec![
                MessageChunk::Link {
                    location: "https://ya.ru".into(),
                    title: "https://ya.ru".into(),
                    protocol: ProtocolType::HTTPS,
                },
                MessageChunk::Text(" ".into()),
                MessageChunk::Link {
                    location: "http://example.com".into(),
                    title: "example".into(),
                    protocol: ProtocolType::HTTP,
                },
                MessageChunk::Link {
                    location: "https://osu.ppy.sh/wiki/silence".into(),
                    title: "wiki:silence".into(),
                    protocol: ProtocolType::HTTPS,
                },
            ]
        );
    }

    #[test]
    fn osu_specific_raw() {
        let message = m("osump://12345 osu://chan/#russian");
        assert_eq!(
            message.chunks.unwrap(),
            vec![
                MessageChunk::Link {
                    title: "osump://12345".into(),
                    location: "osump://12345".into(),
                    protocol: ProtocolType::OSUMP,
                },
                MessageChunk::Text(" ".into()),
                MessageChunk::Link {
                    title: "osu://chan/#russian".into(),
                    location: "osu://chan/#russian".into(),
                    protocol: ProtocolType::OSU,
                }
            ]
        );
    }

    #[test]
    fn osu_specific_markdown() {
        let message = m("[osump://12345 join my room] [osu://chan/#osu #chaos]");
        assert_eq!(
            message.chunks.unwrap(),
            vec![
                MessageChunk::Link {
                    location: "osump://12345".into(),
                    title: "join my room".into(),
                    protocol: ProtocolType::OSUMP,
                },
                MessageChunk::Text(" ".into()),
                MessageChunk::Link {
                    location: "osu://chan/#osu".into(),
                    title: "#chaos".into(),
                    protocol: ProtocolType::OSU,
                }
            ]
        );
    }
}
