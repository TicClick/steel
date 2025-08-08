use eframe::egui;
use steel_core::settings::chat::ChatPosition;

use std::collections::HashMap;

use crate::gui::{chat::chat_view::ChatView, state::UIState};

#[derive(Default)]
pub struct ChatViewController {
    views: HashMap<String, ChatView>,
}

impl ChatViewController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn scroll_chat_to(&mut self, state: &UIState, lowercase_name: &str, message_id: usize) {
        if let Some(chat_view) = self.views.get_mut(lowercase_name) {
            chat_view.scroll_to = match state.settings.chat.behaviour.chat_position {
                ChatPosition::Bottom => Some(message_id + 1),
                ChatPosition::Top => Some(message_id),
            };
        }
    }

    pub fn insert_user_mention(&mut self, ctx: &egui::Context, state: &UIState, username: String) {
        if let Some(chat_view) = self.views.get_mut(&state.active_chat_tab_name) {
            chat_view.insert_user_mention(ctx, username);
        }
    }

    pub fn response_widget_id(&self, active_chat_name: &str) -> Option<egui::Id> {
        if let Some(chat_view) = self.views.get(active_chat_name) {
            return chat_view.response_widget_id;
        }
        None
    }

    pub fn return_focus(&self, ctx: &egui::Context, active_chat_name: &str) {
        if let Some(widget_id) = self.response_widget_id(active_chat_name) {
            ctx.memory_mut(|mem| {
                if mem.focused().is_none() {
                    mem.request_focus(widget_id);
                }
            });
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &UIState) {
        if let Some(chat_view) = self.views.get_mut(&state.active_chat_tab_name) {
            chat_view.show(ctx, state);
        }
    }

    pub fn add(&mut self, chat_name: String) {
        self.views
            .insert(chat_name.clone(), ChatView::new(chat_name));
    }

    pub fn remove(&mut self, name: &str) {
        self.views.remove(&name.to_lowercase());
    }
}
