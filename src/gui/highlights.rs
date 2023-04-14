use std::collections::{BTreeMap, BTreeSet};

use crate::core::chat;

#[derive(Debug, Default)]
pub struct HighlightTracker {
    username: String,
    messages: BTreeMap<String, BTreeSet<usize>>,
    highlights: BTreeSet<String>,

    unread_tabs: BTreeSet<String>,
}

impl HighlightTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_highlights(&mut self, hl: &[String]) {
        self.highlights = hl.iter().cloned().collect();
    }

    pub fn set_username(&mut self, username: &str) {
        self.username = username.to_lowercase();
    }

    pub fn maybe_add(&mut self, chat: &chat::Chat, message_id: usize) -> bool {
        let msg = &chat.messages[message_id];
        for token in msg
            .text
            .to_lowercase()
            .split(|ch: char| ch.is_whitespace() || ch.is_ascii_punctuation())
        {
            if self.highlights.contains(token) || token == self.username {
                self.messages
                    .entry(chat.name.to_lowercase())
                    .or_default()
                    .insert(message_id);
                return true;
            }
        }
        false
    }

    pub fn message_contains_highlight(&self, chat: &chat::Chat, message_id: usize) -> bool {
        if let Some(ids) = self.messages.get(&chat.name.to_lowercase()) {
            ids.contains(&message_id)
        } else {
            false
        }
    }

    pub fn tab_contains_highlight(&self, tab_name: &str) -> bool {
        self.unread_tabs.contains(tab_name)
    }

    pub fn drop(&mut self, name: &str) {
        self.messages.remove(name);
        self.mark_as_read(name);
    }

    pub fn mark_as_read(&mut self, name: &str) {
        self.unread_tabs.remove(name);
    }

    pub fn mark_as_unread(&mut self, name: &str) {
        self.unread_tabs.insert(name.to_owned());
    }
}
