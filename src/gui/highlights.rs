use std::collections::{BTreeMap, BTreeSet};

use crate::core::chat;

#[derive(Debug)]
pub enum UnreadType {
    Regular,
    Highlight,
}

#[derive(Debug, Default)]
pub struct HighlightTracker {
    // Ordered highlights are displayed in a separate UI tab.
    // The vector stores copies, since they may be needed even after a chat window is cleared (and the original messages are removed).
    ordered: Vec<(String, chat::Message)>,
    keywords: BTreeSet<String>,

    unread_tabs: BTreeMap<String, UnreadType>,
}

impl HighlightTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_highlights(&mut self, hl: &[String]) {
        self.keywords = hl.iter().filter(|s| !s.is_empty()).cloned().collect();
    }

    pub fn keywords(&self) -> &BTreeSet<String> {
        &self.keywords
    }

    pub fn add(&mut self, normalized_chat_name: &str, msg: &chat::Message) {
        self.ordered
            .push((normalized_chat_name.to_owned(), msg.clone()));
    }

    pub fn ordered(&self) -> &Vec<(String, chat::Message)> {
        &self.ordered
    }

    pub fn unread_type(&self, tab_name: &str) -> Option<&UnreadType> {
        self.unread_tabs.get(tab_name)
    }

    pub fn drop(&mut self, name: &str) {
        self.mark_as_read(name);
    }

    pub fn mark_as_read(&mut self, name: &str) {
        self.unread_tabs.remove(name);
    }

    pub fn mark_as_unread(&mut self, name: &str) {
        if !self.unread_tabs.contains_key(name) {
            self.unread_tabs
                .insert(name.to_owned(), UnreadType::Regular);
        }
    }

    pub fn mark_as_highlighted(&mut self, name: &str) {
        self.unread_tabs
            .insert(name.to_owned(), UnreadType::Highlight);
    }
}
