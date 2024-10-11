use crate::chat::Message;

#[derive(Debug, PartialEq, Clone)]
pub enum LinkType {
    HTTP,
    HTTPS,
    Channel,
    OSU(Action),
}

impl LinkType {
    fn from(value: &[u8]) -> Option<Self> {
        if value.starts_with(PROTOCOL_HTTP.as_bytes()) {
            Some(Self::HTTP)
        } else if value.starts_with(PROTOCOL_HTTPS.as_bytes()) {
            Some(Self::HTTPS)
        } else {
            // osu! doesn't recognize multiplayer links without a trailing slash, but we do.
            // The rest of actions seem to work regardless of the slash's presence.

            let mut value = value;
            if value.ends_with(b"/") {
                value = &value[..value.len() - 1];
            }

            if value.starts_with(PROTOCOL_OSU.as_bytes()) {
                Action::extract_from_osu(value).map(Self::OSU)
            } else if value.starts_with(PROTOCOL_OSUMP.as_bytes()) {
                Action::extract_from_osump(value).map(Self::OSU)
            } else {
                None
            }
        }
    }
}

pub const PROTOCOL_HTTP: &str = "http://";
pub const PROTOCOL_HTTPS: &str = "https://";
pub const PROTOCOL_OSU: &str = "osu://";
pub const PROTOCOL_OSUMP: &str = "osump://";

pub const KNOWN_PROTOCOLS: [&str; 4] =
    [PROTOCOL_HTTP, PROTOCOL_HTTPS, PROTOCOL_OSU, PROTOCOL_OSUMP];

#[derive(Debug, PartialEq, Clone)]
pub enum Action {
    Chat(String),
    OpenBeatmap(u64), // Let's be optimistic
    OpenDifficulty(u64),
    Multiplayer(u32),
}

impl Action {
    fn extract_from_osu(s: &[u8]) -> Option<Self> {
        if s.len() < PROTOCOL_OSU.len() {
            return None;
        }

        let rest = &s[PROTOCOL_OSU.len()..];
        if rest.starts_with(b"chan/") {
            match std::str::from_utf8(&rest[5..]) {
                Ok(channel) => Some(Self::Chat(channel.to_owned())),
                Err(_) => None,
            }
        } else if rest.starts_with(b"dl/s/") {
            match std::str::from_utf8(&rest[5..]) {
                Ok(beatmap_id) => match beatmap_id.parse() {
                    Ok(beatmap_id) => Some(Self::OpenBeatmap(beatmap_id)),
                    Err(_) => None,
                },
                Err(_) => None,
            }
        } else if rest.starts_with(b"dl/b/") {
            match std::str::from_utf8(&rest[5..]) {
                Ok(difficulty_id) => match difficulty_id.parse() {
                    Ok(difficulty_id) => Some(Self::OpenDifficulty(difficulty_id)),
                    Err(_) => None,
                },
                Err(_) => None,
            }
        } else if rest.starts_with(b"dl/") {
            match std::str::from_utf8(&rest[3..]) {
                Ok(beatmap_id) => match beatmap_id.parse() {
                    Ok(beatmap_id) => Some(Self::OpenBeatmap(beatmap_id)),
                    Err(_) => None,
                },
                Err(_) => None,
            }
        } else if rest.starts_with(b"b/") {
            match std::str::from_utf8(&rest[2..]) {
                Ok(difficulty_id) => match difficulty_id.parse() {
                    Ok(difficulty_id) => Some(Self::OpenDifficulty(difficulty_id)),
                    Err(_) => None,
                },
                Err(_) => None,
            }
        } else {
            None
        }
    }

    fn extract_from_osump(s: &[u8]) -> Option<Self> {
        if s.len() < PROTOCOL_OSUMP.len() {
            return None;
        }

        let rest = &s[PROTOCOL_OSUMP.len()..];
        match std::str::from_utf8(rest) {
            Ok(lobby_id) => match lobby_id.parse() {
                Ok(lobby_id) => Some(Self::Multiplayer(lobby_id)),
                Err(_) => None,
            },
            Err(_) => None,
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum LinkLocation {
    Raw {
        pos: (usize, usize),
        protocol: LinkType,
    },
    Markdown {
        pos: (usize, usize),
        title: (usize, usize),
        location: (usize, usize),
        protocol: LinkType,
    },
    Wiki {
        pos: (usize, usize),
        title: (usize, usize),
        protocol: LinkType,
    },
}

impl LinkLocation {
    pub fn position(&self) -> &(usize, usize) {
        match self {
            Self::Raw { pos, .. } | Self::Markdown { pos, .. } | Self::Wiki { pos, .. } => pos,
        }
    }

    pub fn protocol(&self) -> LinkType {
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
        link_type: LinkType,
    },
}

impl Message {
    pub fn parse_for_links(&mut self) {
        let mut ret: Vec<MessageChunk> = Vec::new();
        let mut links: Vec<LinkLocation> = Vec::new();

        let mut i = 0;
        let bs = self.text.as_bytes();

        let protocol_lookahead = |pos: usize| -> bool {
            KNOWN_PROTOCOLS.iter().any(|protocol| {
                pos + protocol.len() < bs.len()
                    && &bs[pos..pos + protocol.len()] == protocol.as_bytes()
            })
        };

        while i < bs.len() {
            // Only consider [[...]], [...], http(s)://, or osu(mp)://.
            // Yeah, I know there are other protocols and formats, but no.
            while i < bs.len() && (bs[i] != b'[' && bs[i] != b'h' && bs[i] != b'o' && bs[i] != b'#')
            {
                i += 1;
            }
            if i == bs.len() {
                break;
            }

            let start = i;

            // Plain link starting with a protocol, no title.
            if protocol_lookahead(i) {
                while i < bs.len() && bs[i] != b' ' {
                    i += 1;
                }
                if let Some(protocol_type) = LinkType::from(&bs[start..i]) {
                    links.push(LinkLocation::Raw {
                        pos: (start, i),
                        protocol: protocol_type,
                    });
                }
                continue;
            }

            // Channel name.
            if i < bs.len() && bs[i] == b'#' {
                i += 1;
                while i < bs.len()
                    && ((b'a' <= bs[i] && bs[i] <= b'z')
                        || bs[i] == b'_'
                        || (b'0' <= bs[i] && bs[i] <= b'9'))
                {
                    i += 1;
                }
                // '#' is an invalid channel name -- skip it.
                if i > start + 1 {
                    links.push(LinkLocation::Raw {
                        pos: (start, i),
                        protocol: LinkType::Channel,
                    });
                }
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
                        protocol: LinkType::HTTPS,
                    });
                } else {
                    // Reset failed state and see what the next loop iteration will bring.
                    // FIXME: Wow, this makes it quadratic -- I guess I should use KMP or a suffix tree once it becomes an issue?
                    i = start + 1;
                }
                continue;
            }

            // Link with title
            if protocol_lookahead(i + 1) {
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

                        if let Some(protocol_type) =
                            LinkType::from(&bs[location_start..location_end])
                        {
                            links.push(LinkLocation::Markdown {
                                pos: (start, end),
                                title: (title_start, title_end),
                                location: (location_start, location_end),
                                protocol: protocol_type,
                            });
                        }
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
                link_type: links[i].protocol(),
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
                link_type: LinkType::HTTP,
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
                    link_type: LinkType::HTTP,
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
                    link_type: LinkType::HTTP,
                },
                MessageChunk::Text(" ".into()),
                MessageChunk::Link {
                    location: "http://test".into(),
                    title: "Test (links here)".into(),
                    link_type: LinkType::HTTP,
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
                    link_type: LinkType::HTTP,
                },
                MessageChunk::Link {
                    location: "http://test".into(),
                    title: "Test (links here)".into(),
                    link_type: LinkType::HTTP,
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
                    link_type: LinkType::HTTPS,
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
                    link_type: LinkType::HTTPS,
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
                    link_type: LinkType::HTTPS,
                },
                MessageChunk::Text(" ".into()),
                MessageChunk::Link {
                    location: "https://bhttps://".into(),
                    title: "https://bhttps://".into(),
                    link_type: LinkType::HTTPS,
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
                    link_type: LinkType::HTTPS,
                },
                MessageChunk::Text(" ".into()),
                MessageChunk::Link {
                    location: "http://example.com".into(),
                    title: "example".into(),
                    link_type: LinkType::HTTP,
                },
                MessageChunk::Link {
                    location: "https://osu.ppy.sh/wiki/silence".into(),
                    title: "wiki:silence".into(),
                    link_type: LinkType::HTTPS,
                },
            ]
        );
    }

    #[test]
    fn osu_multiplayer_raw() {
        let message = m("osump://12345");
        assert_eq!(
            message.chunks.unwrap(),
            vec![MessageChunk::Link {
                title: "osump://12345".into(),
                location: "osump://12345".into(),
                link_type: LinkType::OSU(Action::Multiplayer(12345)),
            },]
        );

        let message = m("osump://12345/");
        assert_eq!(
            message.chunks.unwrap(),
            vec![MessageChunk::Link {
                title: "osump://12345/".into(),
                location: "osump://12345/".into(),
                link_type: LinkType::OSU(Action::Multiplayer(12345)),
            },]
        );
    }

    #[test]
    fn osu_download_beatmapset_raw() {
        let message = m("osu://dl/42311");
        assert_eq!(
            message.chunks.unwrap(),
            vec![MessageChunk::Link {
                title: "osu://dl/42311".into(),
                location: "osu://dl/42311".into(),
                link_type: LinkType::OSU(Action::OpenBeatmap(42311)),
            },]
        );

        let message = m("osu://dl/42311/");
        assert_eq!(
            message.chunks.unwrap(),
            vec![MessageChunk::Link {
                title: "osu://dl/42311/".into(),
                location: "osu://dl/42311/".into(),
                link_type: LinkType::OSU(Action::OpenBeatmap(42311)),
            },]
        );

        let message = m("osu://dl/s/42311");
        assert_eq!(
            message.chunks.unwrap(),
            vec![MessageChunk::Link {
                title: "osu://dl/s/42311".into(),
                location: "osu://dl/s/42311".into(),
                link_type: LinkType::OSU(Action::OpenBeatmap(42311)),
            },]
        );

        let message = m("osu://dl/s/42311/");
        assert_eq!(
            message.chunks.unwrap(),
            vec![MessageChunk::Link {
                title: "osu://dl/s/42311/".into(),
                location: "osu://dl/s/42311/".into(),
                link_type: LinkType::OSU(Action::OpenBeatmap(42311)),
            },]
        );
    }

    #[test]
    fn osu_download_difficulty_raw() {
        let message = m("osu://dl/b/641387");
        assert_eq!(
            message.chunks.unwrap(),
            vec![MessageChunk::Link {
                title: "osu://dl/b/641387".into(),
                location: "osu://dl/b/641387".into(),
                link_type: LinkType::OSU(Action::OpenDifficulty(641387)),
            },]
        );

        let message = m("osu://dl/b/641387/");
        assert_eq!(
            message.chunks.unwrap(),
            vec![MessageChunk::Link {
                title: "osu://dl/b/641387/".into(),
                location: "osu://dl/b/641387/".into(),
                link_type: LinkType::OSU(Action::OpenDifficulty(641387)),
            },]
        );

        let message = m("osu://b/641387");
        assert_eq!(
            message.chunks.unwrap(),
            vec![MessageChunk::Link {
                title: "osu://b/641387".into(),
                location: "osu://b/641387".into(),
                link_type: LinkType::OSU(Action::OpenDifficulty(641387)),
            },]
        );

        let message = m("osu://b/641387/");
        assert_eq!(
            message.chunks.unwrap(),
            vec![MessageChunk::Link {
                title: "osu://b/641387/".into(),
                location: "osu://b/641387/".into(),
                link_type: LinkType::OSU(Action::OpenDifficulty(641387)),
            },]
        );
    }

    #[test]
    fn osu_specific_raw() {
        let message = m("osump://12345/ osu://chan/#russian");
        assert_eq!(
            message.chunks.unwrap(),
            vec![
                MessageChunk::Link {
                    title: "osump://12345/".into(),
                    location: "osump://12345/".into(),
                    link_type: LinkType::OSU(Action::Multiplayer(12345)),
                },
                MessageChunk::Text(" ".into()),
                MessageChunk::Link {
                    title: "osu://chan/#russian".into(),
                    location: "osu://chan/#russian".into(),
                    link_type: LinkType::OSU(Action::Chat("#russian".into())),
                }
            ]
        );
    }

    #[test]
    fn osu_specific_markdown() {
        let message = m("[osump://12345/ join my room] [osu://chan/#osu #chaos]");
        assert_eq!(
            message.chunks.unwrap(),
            vec![
                MessageChunk::Link {
                    location: "osump://12345/".into(),
                    title: "join my room".into(),
                    link_type: LinkType::OSU(Action::Multiplayer(12345)),
                },
                MessageChunk::Text(" ".into()),
                MessageChunk::Link {
                    location: "osu://chan/#osu".into(),
                    title: "#chaos".into(),
                    link_type: LinkType::OSU(Action::Chat("#osu".into())),
                }
            ]
        );
    }

    #[test]
    fn unicode() {
        let message =
            m("[osump://12345/ моя комната] [osu://chan/#osu #господичтоэто] [osu://dl/123 非]");
        assert_eq!(
            message.chunks.unwrap(),
            vec![
                MessageChunk::Link {
                    location: "osump://12345/".into(),
                    title: "моя комната".into(),
                    link_type: LinkType::OSU(Action::Multiplayer(12345)),
                },
                MessageChunk::Text(" ".into()),
                MessageChunk::Link {
                    location: "osu://chan/#osu".into(),
                    title: "#господичтоэто".into(),
                    link_type: LinkType::OSU(Action::Chat("#osu".into())),
                },
                MessageChunk::Text(" ".into()),
                MessageChunk::Link {
                    location: "osu://dl/123".into(),
                    title: "非".into(),
                    link_type: LinkType::OSU(Action::OpenBeatmap(123)),
                },
            ]
        );
    }

    #[test]
    fn plain_channel_names() {
        let message = m("Check this out: #russian + #mp_10966036 + #spect_672931 = ???");
        assert_eq!(
            message.chunks.unwrap(),
            vec![
                MessageChunk::Text("Check this out: ".into()),
                MessageChunk::Link {
                    location: "#russian".into(),
                    title: "#russian".into(),
                    link_type: LinkType::Channel,
                },
                MessageChunk::Text(" + ".into()),
                MessageChunk::Link {
                    location: "#mp_10966036".into(),
                    title: "#mp_10966036".into(),
                    link_type: LinkType::Channel,
                },
                MessageChunk::Text(" + ".into()),
                MessageChunk::Link {
                    location: "#spect_672931".into(),
                    title: "#spect_672931".into(),
                    link_type: LinkType::Channel,
                },
                MessageChunk::Text(" = ???".into()),
            ]
        );
    }
}
