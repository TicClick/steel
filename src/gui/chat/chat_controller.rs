use eframe::egui::{self, Widget};
use steel_core::{
    chat::ConnectionStatus,
    settings::{chat::ChatPosition, ChatBackend},
};

use std::{collections::HashMap, time::Instant};

use crate::gui::{chat::chat_view::ChatView, state::UIState};
use crate::{
    core::http::{oauth_flow::OAuthFlowManager, token_storage::PersistedTokenState},
    gui::chat::login_screen::LoginScreen,
};

pub struct ChatViewController {
    views: HashMap<String, ChatView>,
    oauth_flow: OAuthFlowManager,
    cached_token_state: Option<PersistedTokenState>,
    last_token_check: Instant,
}

impl ChatViewController {
    pub fn new() -> Self {
        Self::default()
    }

    fn ensure_token_state_loaded(&mut self) {
        if self.cached_token_state.is_none() || self.last_token_check.elapsed().as_secs() > 2 {
            self.cached_token_state = crate::core::http::token_storage::load_token_state().ok();
            self.last_token_check = Instant::now();
        }
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

    pub fn enable_filter(&mut self, state: &UIState) {
        if let Some(chat_view) = self.views.get_mut(&state.active_chat_tab_name) {
            chat_view.enable_filter();
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
                // Only request focus if nothing else is focused. This prevents stealing focus from other inputs
                // (filters, settings, chat tabs, etc).
                if mem.focused().is_none() {
                    mem.request_focus(widget_id);
                }
            });
        }
    }

    // FIXME(TicClick): rework this into a screen that supports BOTH modes.
    pub fn show(&mut self, ctx: &egui::Context, state: &UIState) {
        match self.views.get_mut(&state.active_chat_tab_name) {
            Some(chat_view) => chat_view.show(ctx, state),
            None => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    if matches!(state.settings.chat.backend, ChatBackend::API) {
                        match state.connection {
                            ConnectionStatus::Disconnected { auth_failed, .. } => {
                                if auth_failed {
                                    self.cached_token_state = None;
                                }
                                self.ensure_token_state_loaded();
                                LoginScreen::new(
                                    state,
                                    &self.oauth_flow,
                                    self.cached_token_state.as_ref(),
                                )
                                .ui(ui);
                            }
                            ConnectionStatus::Scheduled(when) => {
                                let now = chrono::Local::now();
                                let progress_pct = (when - now).as_seconds_f32();
                                ui.horizontal(|ui| {
                                    ui.label(format!(
                                        "Waiting {progress_pct:.1} s before reconnecting..."
                                    ));
                                    ui.add(egui::Spinner::new());
                                });
                            }
                            ConnectionStatus::InProgress => {
                                ui.label("Logging in...");
                                ui.add(egui::Spinner::new());
                            }
                            ConnectionStatus::Connected => {
                                // some kind of "welcome, you are connected" screen?
                            }
                        }
                    }
                });
            }
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

impl Default for ChatViewController {
    fn default() -> Self {
        Self {
            views: HashMap::new(),
            oauth_flow: OAuthFlowManager::new(),
            cached_token_state: None,
            last_token_check: Instant::now(),
        }
    }
}
