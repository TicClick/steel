use std::collections::{BTreeMap, BTreeSet};

use crate::core::chat;

#[derive(Debug)]
pub enum UnreadType {
    Regular,
    Highlight,
}

#[derive(Debug, Default)]
pub struct ReadTracker {
    // Tracks unread status of chat tabs
    unread_tabs: BTreeMap<String, UnreadType>,

    // Tracks the position of unread messages marker for each chat
    last_read_messages: BTreeMap<String, usize>,

    // Highlights tracking
    ordered_highlights: Vec<(String, chat::Message)>,
    highlight_keywords: BTreeSet<String>,
}

impl ReadTracker {
    pub fn new() -> Self {
        Self::default()
    }

    // Message tracking for unread markers
    pub fn set_last_read_position(&mut self, chat_name: &str, position: usize) {
        self.last_read_messages
            .insert(chat_name.to_owned(), position);
    }

    pub fn get_last_read_position(&self, chat_name: &str) -> Option<usize> {
        self.last_read_messages.get(chat_name).copied()
    }

    pub fn remove_last_read_position(&mut self, chat_name: &str) {
        self.last_read_messages.remove(chat_name);
    }

    pub fn update_chat_tracking(&mut self, old_chat: &str, new_chat: &str, message_count: usize) {
        // When switching chats, move the unread marker to the end of the current chat
        if !old_chat.is_empty() {
            self.set_last_read_position(old_chat, message_count);
        }

        // If we switch to a chat with an unread marker that matches the message count,
        // remove the marker as it would be at the end anyway
        if let Some(last_idx) = self.get_last_read_position(new_chat) {
            if last_idx == message_count {
                self.remove_last_read_position(new_chat);
            }
        }
    }

    // Highlight tracking
    pub fn set_highlights(&mut self, hl: &[String]) {
        self.highlight_keywords = hl.iter().filter(|s| !s.is_empty()).cloned().collect();
    }

    pub fn keywords(&self) -> &BTreeSet<String> {
        &self.highlight_keywords
    }

    pub fn add_highlight(&mut self, normalized_chat_name: &str, msg: &chat::Message) {
        self.ordered_highlights
            .push((normalized_chat_name.to_owned(), msg.clone()));
    }

    pub fn ordered_highlights(&self) -> &Vec<(String, chat::Message)> {
        &self.ordered_highlights
    }

    // Read/unread status tracking
    pub fn unread_type(&self, tab_name: &str) -> Option<&UnreadType> {
        self.unread_tabs.get(tab_name)
    }

    pub fn drop(&mut self, name: &str) {
        self.mark_as_read(name);
        self.remove_last_read_position(name);
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
