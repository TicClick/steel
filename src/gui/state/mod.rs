use std::collections::BTreeMap;

use steel_core::chat::{Chat, ChatLike, ChatState, ConnectionStatus, Message, MessageType};
use steel_core::ipc::updater::UpdateState;
use steel_core::ipc::{client::CoreClient, server::AppMessageIn};

use eframe::egui;
use tokio::sync::mpsc::UnboundedSender;

use crate::core::settings::Settings;

use crate::gui::highlights;

use super::filter::FilterCollection;
use super::{HIGHLIGHTS_SEPARATOR, HIGHLIGHTS_TAB_NAME, SERVER_TAB_NAME};

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
    chats: Vec<Chat>,
    name_to_chat: BTreeMap<String, usize>,
    pub server_messages: Vec<Message>,
    pub active_chat_tab_name: String,

    pub core: CoreClient,
    pub highlights: highlights::HighlightTracker,

    pub update_state: UpdateState,
    pub sound_player: crate::core::sound::SoundPlayer,

    #[cfg(feature = "glass")]
    pub glass: glass::Glass,

    pub filter: FilterCollection,
}

impl UIState {
    pub fn new(app_queue_handle: UnboundedSender<AppMessageIn>, settings: Settings) -> Self {
        Self {
            connection: ConnectionStatus::default(),
            settings,
            chats: Vec::default(),
            name_to_chat: BTreeMap::default(),
            server_messages: Vec::default(),
            active_chat_tab_name: String::new(),
            core: CoreClient::new(app_queue_handle),
            highlights: highlights::HighlightTracker::new(),
            update_state: UpdateState::default(),
            sound_player: crate::core::sound::SoundPlayer::new(),

            #[cfg(feature = "glass")]
            glass: {
                let mut g = glass::Glass::default();
                g.load_settings();
                g
            },

            filter: FilterCollection::default(),
        }
    }

    pub fn update_settings(&mut self, settings: &Settings) {
        self.settings = settings.clone();

        // FIXME: Move this to a separate setter.
        self.highlights
            .set_highlights(&self.settings.notifications.highlights.words);
    }

    pub fn update_highlights(&mut self, words: &str) {
        self.settings.notifications.highlights.words = words
            .trim()
            .split(HIGHLIGHTS_SEPARATOR)
            .filter(|s| !s.is_empty())
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
        if let Some(pos) = self.name_to_chat.get(&self.active_chat_tab_name) {
            self.chats.get(*pos)
        } else {
            None
        }
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.connection, ConnectionStatus::Connected)
    }

    pub fn add_new_chat(&mut self, name: String, switch_to_chat: bool) {
        let chat = Chat::new(&name);
        let normalized = chat.name.to_lowercase();
        self.name_to_chat.insert(normalized, self.chats.len());
        self.chats.push(chat);
        if switch_to_chat {
            self.active_chat_tab_name = name;
        }
    }

    pub fn chat_count(&self) -> usize {
        self.chats.len()
    }

    pub fn place_tab_after(&mut self, original_tab_idx: usize, place_after_idx: usize) {
        let ch = self.chats.remove(original_tab_idx);
        self.chats.insert(place_after_idx, ch);
        for (pos, ch) in self.chats.iter().enumerate() {
            self.name_to_chat.insert(ch.name.to_lowercase(), pos);
        }
    }

    pub fn is_active_tab(&self, target: &str) -> bool {
        self.active_chat_tab_name == target
    }

    pub fn set_chat_state(&mut self, target: &str, state: ChatState) {
        let normalized = target.to_lowercase();
        if let Some(pos) = self.name_to_chat.get(&normalized) {
            if let Some(ch) = self.chats.get_mut(*pos) {
                if ch.state != state {
                    ch.set_state(state);
                }
            }
        }
    }

    pub fn push_chat_message(
        &mut self,
        target: String,
        mut message: Message,
        ctx: &egui::Context,
    ) -> bool {
        let normalized = target.to_lowercase();
        let is_tab_inactive = !self.is_active_tab(&normalized);
        let is_system_message = matches!(message.r#type, MessageType::System);
        let mut name_updated = false;

        if let Some(pos) = self.name_to_chat.get(&normalized) {
            if let Some(ch) = self.chats.get_mut(*pos) {
                message.id = Some(ch.messages.len());
                message.parse_for_links();

                #[allow(unused_mut)] // glass
                let mut current_username = Some(&self.settings.chat.irc.username);
                #[cfg(feature = "glass")]
                if self
                    .glass
                    .is_username_highlight_suppressed(&normalized, &message)
                {
                    current_username = None;
                }

                // If the chat was open with an improper case, fix it!
                if ch.name != target && !is_system_message {
                    ch.name = target;
                    name_updated = true;
                }

                message.detect_highlights(self.highlights.keywords(), current_username);

                let contains_highlight = message.highlight;
                let requires_attention =
                    !is_system_message && (contains_highlight || !normalized.is_channel());

                if contains_highlight {
                    self.highlights.add(&normalized, &message);
                    if self.active_chat_tab_name != HIGHLIGHTS_TAB_NAME {
                        self.highlights.mark_as_unread(HIGHLIGHTS_TAB_NAME);
                    }
                }
                ch.push(message);

                if is_tab_inactive && !is_system_message {
                    if requires_attention {
                        self.highlights.mark_as_highlighted(&normalized);
                    } else {
                        self.highlights.mark_as_unread(&normalized);
                    }
                }

                if !ctx.input(|i| i.viewport().focused.unwrap_or(false)) && requires_attention {
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                        eframe::egui::UserAttentionType::Critical,
                    ));
                    if let Some(sound) = &self.settings.notifications.highlights.sound {
                        self.sound_player.play(sound);
                    }
                }
            }
        }
        name_updated
    }

    pub fn validate_reference(&self, chat_name: &str, highlight: &Message) -> bool {
        match highlight.id {
            None => false,
            Some(id) => match self.name_to_chat.get(chat_name) {
                None => false,
                Some(pos) => match self.chats.get(*pos) {
                    None => false,
                    Some(ch) => match ch.messages.get(id) {
                        None => false,
                        Some(msg) => highlight.time == msg.time,
                    },
                },
            },
        }
    }

    pub fn push_server_message(&mut self, text: &str) {
        let mut msg = Message::new_system(text);
        msg.parse_for_links();
        self.server_messages.push(msg);
        if self.active_chat_tab_name != SERVER_TAB_NAME {
            self.highlights.mark_as_unread(SERVER_TAB_NAME);
        }
    }

    pub fn remove_chat(&mut self, target: String) {
        let normalized = target.to_lowercase();
        if let Some(pos) = self.name_to_chat.remove(&normalized) {
            self.chats.remove(pos);
            for ch in &self.chats[pos..] {
                if let Some(ch) = self.name_to_chat.get_mut(&ch.name.to_lowercase()) {
                    *ch -= 1;
                }
            }
        }
        self.highlights.drop(&normalized);
    }

    pub fn clear_chat(&mut self, target: &str) {
        if let Some(pos) = self.name_to_chat.get(target) {
            if let Some(chat) = self.chats.get_mut(*pos) {
                chat.messages.clear();
            }
        }
        self.highlights.drop(target);
    }

    pub fn filter_chats<F>(
        &self,
        f: F,
    ) -> std::iter::Filter<std::iter::Enumerate<std::slice::Iter<'_, steel_core::chat::Chat>>, F>
    where
        F: Fn(&(usize, &Chat)) -> bool,
    {
        self.chats.iter().enumerate().filter(f)
    }

    pub fn has_chat(&self, target: &str) -> bool {
        self.name_to_chat.contains_key(&target.to_lowercase())
    }
}
