pub mod about;
pub mod chat;
pub mod chat_tabs;
pub mod highlights;
pub mod menu;
pub mod settings;
pub mod window;

use std::collections::BTreeMap;

use tokio::sync::mpsc::Sender;

use crate::app::AppMessageIn;
use crate::core::chat::{Chat, ChatLike, ChatType, Message, MessageChunk};
use crate::core::irc::ConnectionStatus;
use crate::core::settings::Settings;
use crate::core::updater::Updater;

#[derive(Debug)]
pub enum UIMessageIn {
    SettingsChanged(Settings),
    ConnectionStatusChanged(ConnectionStatus),
    NewMessageReceived { target: String, message: Message },
    NewServerMessageReceived(String),
    NewChatOpened(String),
    ChatClosed(String),
}

#[derive(Debug)]
pub struct UIState {
    pub connection: ConnectionStatus,
    pub settings: Settings,
    pub chats: BTreeMap<String, Chat>,
    pub active_chat_tab_name: String,

    pub app_queue_handle: Sender<AppMessageIn>,
    pub highlights: highlights::HighlightTracker,
    pub message_chunks: BTreeMap<String, BTreeMap<usize, Vec<MessageChunk>>>,

    pub updater: Updater,
}

impl UIState {
    pub fn new(app_queue_handle: Sender<AppMessageIn>) -> Self {
        Self {
            connection: ConnectionStatus::default(),
            settings: Settings::default(),
            chats: BTreeMap::default(),
            active_chat_tab_name: String::new(),
            app_queue_handle,
            highlights: highlights::HighlightTracker::new(),
            message_chunks: BTreeMap::default(),
            updater: Updater::new(),
        }
    }

    pub fn set_settings(&mut self, settings: Settings) {
        self.settings = settings;
        self.highlights
            .set_username(&self.settings.chat.irc.username);
        self.highlights
            .set_highlights(&self.settings.notifications.highlights.words);
    }

    pub fn update_highlights(&mut self, words: &str) {
        self.settings.notifications.highlights.words = words
            .split_whitespace()
            .map(|el| el.to_lowercase())
            .collect();
        self.settings.notifications.highlights.words.sort();
        self.highlights
            .set_highlights(&self.settings.notifications.highlights.words);
    }

    pub fn active_chat(&self) -> Option<&Chat> {
        self.chats.get(&self.active_chat_tab_name)
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.connection, ConnectionStatus::Connected)
    }

    pub fn add_new_chat(&mut self, target: String) {
        let name = match target.chat_type() {
            ChatType::Channel => {
                let tmp = target.to_lowercase();
                if !tmp.is_channel() {
                    format!("#{}", tmp)
                } else {
                    tmp
                }
            }
            ChatType::Person => target,
        };
        self.chats
            .insert(name.to_owned(), Chat::new(name.to_owned()));
        if !name.is_channel() {
            self.active_chat_tab_name = name;
        }
    }

    fn is_active_tab(&self, target: &str) -> bool {
        self.active_chat_tab_name == target
    }

    fn push_chat_message(&mut self, target: String, message: Message) {
        let inactive = !self.is_active_tab(&target);
        if let Some(ch) = self.chats.get_mut(&target) {
            let id = ch.messages.len();

            if let Some(chunks) = message.chunked_text() {
                self.message_chunks
                    .entry(ch.name.clone())
                    .or_default()
                    .insert(id, chunks);
            }

            ch.push(message);
            self.highlights.maybe_add(ch, id, inactive);
            if !target.is_channel() && inactive {
                self.highlights.mark_as_unread(&ch.name);
            }
        }
    }

    pub fn get_chunks(&self, target: &str, message_id: usize) -> Vec<MessageChunk> {
        if let Some(messages) = self.message_chunks.get(target) {
            if let Some(val) = messages.get(&message_id) {
                return val.clone();
            }
        }
        if let Some(ch) = self.chats.get(target) {
            return vec![MessageChunk::Text(ch.messages[message_id].text.clone())];
        }
        Vec::new()
    }

    pub fn remove_chat(&mut self, target: String) {
        self.chats.remove(&target);
        self.highlights.drop(&target);
    }
}
