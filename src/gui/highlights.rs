use std::collections::BTreeSet;

use crate::core::chat;

#[derive(Debug, Default)]
pub struct HighlightTracker {
    username: String,

    // Ordered highlights are displayed in a separate UI tab.
    // The vector stores copies, since they may be needed even after a chat window is cleared (and the original messages are removed).
    ordered: Vec<(String, chat::Message)>,
    keywords: BTreeSet<String>,

    unread_tabs: BTreeSet<String>,
}

impl HighlightTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_highlights(&mut self, hl: &[String]) {
        self.keywords = hl.iter().cloned().collect();
        self.keywords.insert(self.username.to_owned());
    }

    pub fn keywords(&self) -> &BTreeSet<String> {
        &self.keywords
    }

    pub fn set_username(&mut self, username: &str) {
        self.username = username.to_lowercase();
    }

    pub fn add(&mut self, normalized_chat_name: &str, msg: &chat::Message) {
        self.ordered
            .push((normalized_chat_name.to_owned(), msg.clone()));
    }

    pub fn ordered(&self) -> &Vec<(String, chat::Message)> {
        &self.ordered
    }

    pub fn tab_contains_highlight(&self, tab_name: &str) -> bool {
        self.unread_tabs.contains(tab_name)
    }

    pub fn drop(&mut self, name: &str) {
        self.mark_as_read(name);
    }

    pub fn mark_as_read(&mut self, name: &str) {
        self.unread_tabs.remove(name);
    }

    pub fn mark_as_unread(&mut self, name: &str) {
        self.unread_tabs.insert(name.to_owned());
    }
}
