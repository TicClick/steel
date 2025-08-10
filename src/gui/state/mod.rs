use std::collections::HashSet;

use steel_core::chat::{Chat, ChatLike, ChatState, ConnectionStatus, Message, MessageType};
use steel_core::ipc::updater::UpdateState;
use steel_core::ipc::{client::CoreClient, server::AppMessageIn};

use eframe::egui;
use tokio::sync::mpsc::UnboundedSender;

use crate::core::settings::Settings;
use crate::gui::HIGHLIGHTS_TAB_NAME;

use super::HIGHLIGHTS_SEPARATOR;
use crate::gui::widgets::connection_indicator::ConnectionIndicator;

#[derive(Debug)]
pub enum UIMessageIn {
    SettingsChanged(Box<Settings>),
    ConnectionStatusChanged(ConnectionStatus),
    ConnectionActivity,
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
    pub active_chat_tab_name: String,

    pub core: CoreClient,

    pub update_state: UpdateState,
    pub sound_player: crate::core::sound::SoundPlayer,
    pub original_exe_path: Option<std::path::PathBuf>,

    #[cfg(feature = "glass")]
    pub glass: glass::Glass,

    pub highlights: HashSet<String>,

    pub connection_indicator: ConnectionIndicator,
    notification_start_time: Option<std::time::Instant>,
}

impl UIState {
    pub fn new(
        app_queue_handle: UnboundedSender<AppMessageIn>,
        settings: Settings,
        original_exe_path: Option<std::path::PathBuf>,
    ) -> Self {
        let irc_settings = settings.chat.irc.clone();

        Self {
            connection: ConnectionStatus::default(),
            settings,
            chats: Vec::new(),
            active_chat_tab_name: String::new(),
            core: CoreClient::new(app_queue_handle),
            update_state: UpdateState::default(),
            sound_player: crate::core::sound::SoundPlayer::new(),
            original_exe_path,

            #[cfg(feature = "glass")]
            glass: glass::Glass::default(),

            highlights: HashSet::new(),

            connection_indicator: ConnectionIndicator::new(
                false,
                irc_settings.server,
                irc_settings.ping_timeout,
            ),
            notification_start_time: None,
        }
    }

    pub fn chats(&self) -> &Vec<Chat> {
        &self.chats
    }

    pub fn chats_mut(&mut self) -> &mut Vec<Chat> {
        &mut self.chats
    }

    pub fn name_to_chat(&self, name: &str) -> Option<usize> {
        for (idx, chat) in self.chats.iter().enumerate() {
            if chat.normalized_name == name {
                return Some(idx);
            }
        }
        None
    }

    pub fn update_settings(&mut self, settings: &Settings) {
        self.settings = settings.clone();
        self.update_highlights(
            &self
                .settings
                .notifications
                .highlights
                .words
                .join(HIGHLIGHTS_SEPARATOR),
        );
    }

    pub fn update_highlights(&mut self, words: &str) {
        self.highlights = words
            .trim()
            .split(HIGHLIGHTS_SEPARATOR)
            .filter(|s| !s.is_empty())
            .map(|el| el.to_lowercase())
            .collect();
    }

    pub fn active_chat(&self) -> Option<&Chat> {
        self.find_chat(&self.active_chat_tab_name)
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.connection, ConnectionStatus::Connected)
    }

    pub fn add_new_chat(&mut self, name: String, switch_to_chat: bool) -> Option<&Chat> {
        self.chats.push(Chat::new(&name));
        if let Some(chat) = self.chats.last() {
            if switch_to_chat {
                self.active_chat_tab_name = chat.normalized_name.clone();
            }
        }
        self.chats.last()
    }

    pub fn chat_count(&self) -> usize {
        self.chats.len()
    }

    pub fn is_active_tab(&self, target: &str) -> bool {
        self.active_chat_tab_name == target
    }

    pub fn set_chat_state(&mut self, target: &str, state: ChatState) {
        let normalized = target.to_lowercase();
        if let Some(ch) = self.find_chat_mut(&normalized) {
            ch.set_state(state)
        }
    }

    pub fn push_chat_message(&mut self, target: &str, mut message: Message, ctx: &egui::Context) {
        let normalized = target.to_lowercase();
        let is_tab_active = self.is_active_tab(&normalized);
        let is_system_message = matches!(message.r#type, MessageType::System);

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

        if message.r#type != MessageType::System {
            message.detect_highlights(&self.highlights, current_username);
        }

        let mut should_rename_chat = false;
        if let Some(chat) = self.find_chat_mut(&normalized) {
            message.id = Some(chat.messages.len());
            if chat.name != target && !is_system_message {
                chat.name = target.to_string();
                should_rename_chat = true;
            }
            chat.push(message.clone(), is_tab_active);
        }

        if should_rename_chat {
            self.core.update_window_title();
        }

        if message.highlight {
            self.maybe_notify(ctx, &message, &normalized);
            message.set_original_chat(target);
            if let Some(highlights) = self.find_chat_mut(HIGHLIGHTS_TAB_NAME) {
                highlights.push(message, false);
            }
        }
    }

    pub fn maybe_notify(
        &mut self,
        ctx: &egui::Context,
        message: &Message,
        normalized_chat_name: &str,
    ) {
        let window_unfocused = !ctx.input(|i| i.viewport().focused.unwrap_or(false));
        let should_notify = {
            let should_flash_for_highlight =
                self.settings.notifications.notification_events.highlights;
            let should_flash_for_private_message = normalized_chat_name.is_person()
                && self
                    .settings
                    .notifications
                    .notification_events
                    .private_messages;

            should_flash_for_highlight || should_flash_for_private_message
        };

        if should_notify {
            if window_unfocused {
                if cfg!(target_os = "linux") {
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                        eframe::egui::UserAttentionType::Informational,
                    ));
                } else {
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                        eframe::egui::UserAttentionType::Critical,
                    ));
                    if matches!(
                        self.settings.notifications.notification_style,
                        steel_core::settings::NotificationStyle::Moderate
                    ) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                            eframe::egui::UserAttentionType::Informational,
                        ));
                    };
                    self.notification_start_time = Some(std::time::Instant::now());
                }
            }

            if let Some(sound) = &self.settings.notifications.highlights.sound {
                let should_play_sound = match self.settings.notifications.sound_only_when_unfocused
                {
                    true => window_unfocused && message.highlight,
                    false => message.highlight,
                };
                if should_play_sound {
                    self.sound_player.play(sound);
                }
            }
        }
    }

    pub fn validate_reference(&self, chat_name: &str, message_id: usize) -> bool {
        self.find_chat(chat_name)
            .is_some_and(|c| c.messages.get(message_id).is_some())
    }

    pub fn remove_chat(&mut self, target: String) {
        let normalized = target.to_lowercase();
        let is_active_chat = self.is_active_tab(&normalized);

        if let Some(pos) = self.name_to_chat(&normalized) {
            self.chats.remove(pos);
        }

        if is_active_chat {
            self.switch_to_first_chat();
        }
    }

    pub fn switch_to_first_chat(&mut self) {
        if let Some(first_chat) = self.chats.first() {
            self.active_chat_tab_name = first_chat.normalized_name.clone();
        } else {
            self.active_chat_tab_name.clear();
        }
    }

    pub fn clear_chat(&mut self, target: &str) {
        if let Some(chat) = self.find_chat_mut(target) {
            chat.clear();
        }
    }

    pub fn has_chat(&self, target: &str) -> bool {
        self.find_chat(&target.to_lowercase()).is_some()
    }

    pub fn find_chat(&self, target: &str) -> Option<&Chat> {
        self.chats.iter().find(|ch| ch.normalized_name == target)
    }

    pub fn find_chat_mut(&mut self, target: &str) -> Option<&mut Chat> {
        self.chats
            .iter_mut()
            .find(|ch| ch.normalized_name == target)
    }

    #[cfg(feature = "glass")]
    pub fn update_glass_settings(&mut self, settings: glass::config::GlassSettings) {
        self.glass.set_settings(settings);
    }

    pub fn check_flash_timeout(&mut self, ctx: &eframe::egui::Context) {
        if self.settings.notifications.enable_notification_timeout
            && matches!(
                self.settings.notifications.notification_style,
                steel_core::settings::NotificationStyle::Intensive
            )
        {
            if let Some(start_time) = self.notification_start_time {
                let elapsed = start_time.elapsed().as_secs();
                if elapsed >= self.settings.notifications.notification_timeout_seconds as u64 {
                    // Stop the attention request by sending Informational (less intrusive)
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                        eframe::egui::UserAttentionType::Informational,
                    ));
                    self.notification_start_time = None;
                }
            }
        }
    }
}
