pub mod client;

use std::collections::BTreeMap;

use tokio::sync::mpsc::Sender;

use crate::app::AppMessageIn;
use crate::core::chat::{Chat, ChatLike, ChatState, Message};
use crate::core::irc::ConnectionStatus;
use crate::core::settings::Settings;
use crate::core::updater::Updater;

use crate::gui::highlights;

#[derive(Debug)]
pub enum UIMessageIn {
    SettingsChanged(Settings),
    ConnectionStatusChanged(ConnectionStatus),
    NewMessageReceived { target: String, message: Message },
    NewServerMessageReceived(String),
    NewChatRequested(String, ChatState),
    ChannelJoined(String),
    ChatClosed(String),
    DateChanged,
}

#[derive(Debug)]
pub struct UIState {
    pub connection: ConnectionStatus,
    pub settings: Settings,
    chats: BTreeMap<String, Chat>,
    pub server_messages: Vec<Message>,
    pub active_chat_tab_name: String,

    pub core: client::CoreClient,
    pub highlights: highlights::HighlightTracker,

    pub updater: Updater,
    pub sound_player: crate::core::sound::SoundPlayer,
}

impl UIState {
    pub fn new(app_queue_handle: Sender<AppMessageIn>) -> Self {
        Self {
            connection: ConnectionStatus::default(),
            settings: Settings::default(),
            chats: BTreeMap::default(),
            server_messages: Vec::default(),
            active_chat_tab_name: String::new(),
            core: client::CoreClient::new(app_queue_handle),
            highlights: highlights::HighlightTracker::new(),
            updater: Updater::new(),
            sound_player: crate::core::sound::SoundPlayer::new(),
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

    pub fn chat_message_count(&self) -> usize {
        if let Some(ch) = self.active_chat() {
            ch.messages.len()
        } else {
            match self.active_chat_tab_name.as_str() {
                super::SERVER_TAB_NAME => self.server_messages.len(),
                super::HIGHLIGHTS_TAB_NAME => self.highlights.ordered().len(),
                _ => 0,
            }
        }
    }

    pub fn active_chat(&self) -> Option<&Chat> {
        self.chats.get(&self.active_chat_tab_name)
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.connection, ConnectionStatus::Connected)
    }

    pub fn add_new_chat(&mut self, name: String, state: ChatState, switch_to_chat: bool) {
        let mut chat = Chat::new(name);
        chat.state = state;

        let normalized = chat.name.to_lowercase();
        self.chats.insert(normalized.to_owned(), chat);
        if switch_to_chat {
            self.active_chat_tab_name = normalized;
        }
    }

    pub fn is_active_tab(&self, target: &str) -> bool {
        self.active_chat_tab_name == target
    }

    pub fn set_chat_state(&mut self, target: &str, state: ChatState, reason: Option<&str>) {
        let normalized = target.to_lowercase();
        if let Some(ch) = self.chats.get_mut(&normalized) {
            ch.set_state(state, reason);
        }
    }

    pub fn push_chat_message(
        &mut self,
        target: String,
        mut message: Message,
        frame: &mut eframe::Frame,
    ) {
        let normalized = target.to_lowercase();
        let tab_inactive = !self.is_active_tab(&normalized);
        if let Some(ch) = self.chats.get_mut(&normalized) {
            // If the chat was open with an improper case, fix it!
            if ch.name != target {
                ch.name = target;
            }

            message.id = Some(ch.messages.len());
            message.parse_for_links();
            message.detect_highlights(self.highlights.keywords());

            let highlight = message.highlight;
            if highlight {
                self.highlights.add(&normalized, &message);
            }
            ch.push(message);

            let requires_attention = highlight || !normalized.is_channel();
            if tab_inactive {
                if requires_attention {
                    self.highlights.mark_as_highlighted(&normalized);
                } else {
                    self.highlights.mark_as_unread(&normalized);
                }
            }

            if !frame.info().window_info.focused && requires_attention {
                frame.request_user_attention(eframe::egui::UserAttentionType::Critical);
                if let Some(sound) = &self.settings.notifications.highlights.sound {
                    self.sound_player.play(sound);
                }
            }
        }
    }

    pub fn validate_reference(&self, chat_name: &str, highlight: &Message) -> bool {
        match highlight.id {
            None => false,
            Some(id) => match self.chats.get(chat_name) {
                None => false,
                Some(ch) => match ch.messages.get(id) {
                    None => false,
                    Some(msg) => highlight.time == msg.time,
                },
            },
        }
    }

    pub fn push_server_message(&mut self, text: &str) {
        let mut msg = Message::new_system(text);
        msg.parse_for_links();
        self.server_messages.push(msg);
    }

    pub fn remove_chat(&mut self, target: String) {
        let normalized = target.to_lowercase();
        self.chats.remove(&normalized);
        self.highlights.drop(&normalized);
    }

    pub fn clear_chat(&mut self, target: &str) {
        if let Some(chat) = self.chats.get_mut(target) {
            chat.messages.clear();
        }
        self.highlights.drop(target);
    }

    pub fn filter_chats<F>(
        &self,
        f: F,
    ) -> std::iter::Filter<std::collections::btree_map::Values<'_, std::string::String, Chat>, F>
    where
        F: Fn(&&Chat) -> bool,
    {
        self.chats.values().filter(f)
    }

    pub fn has_chat(&self, target: &str) -> bool {
        self.chats.contains_key(&target.to_lowercase())
    }

    pub fn push_to_all_chats(&mut self, message: Message) {
        for chat in self.chats.values_mut() {
            chat.push(message.clone());
        }
    }

    pub fn mark_all_as_disconnected(&mut self) {
        for chat in self.chats.values_mut() {
            chat.set_state(
                ChatState::Left,
                Some("You have left the chat (disconnected)"),
            );
        }
    }

    pub fn mark_all_as_connected(&mut self) {
        for chat in self.chats.values_mut() {
            let (new_state, reason) = match chat.name.is_channel() {
                // Joins are handled by the app server
                true => (ChatState::JoinInProgress, None),
                false => (ChatState::Joined, Some("You are online")),
            };
            chat.set_state(new_state, reason);
        }
    }
}
