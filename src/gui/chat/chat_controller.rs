use eframe::egui;
use steel_core::{
    chat::ConnectionStatus,
    settings::{
        chat::{ChatPosition, OAuthMode},
        ChatBackend,
    },
};

use std::collections::HashMap;

use crate::core::http::{
    oauth_flow::{OAuthFlowManager, OAuthFlowParams},
    token_storage::load_token_state,
};
use crate::gui::{chat::chat_view::ChatView, state::UIState};

#[derive(Default)]
pub struct ChatViewController {
    views: HashMap<String, ChatView>,
    oauth_flow: OAuthFlowManager,
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

    pub fn show(&mut self, ctx: &egui::Context, state: &UIState) {
        match self.views.get_mut(&state.active_chat_tab_name) {
            Some(chat_view) => chat_view.show(ctx, state),
            None => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    if !(matches!(state.settings.chat.backend, ChatBackend::API)
                        && matches!(state.connection, ConnectionStatus::Disconnected { .. }))
                    {
                        return;
                    }

                    self.show_oauth_login_ui(ui, state);
                });
            }
        }
    }

    fn show_oauth_login_ui(&mut self, ui: &mut egui::Ui, state: &UIState) {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.heading("osu! API Authentication");
            ui.add_space(20.0);

            match load_token_state() {
                Ok(token_state) => {
                    if token_state.is_access_token_valid() {
                        ui.label("Token status: valid");
                        let expires_at = token_state.access_expires_at;
                        ui.label(format!(
                            "Expires at: {}",
                            expires_at.format("%Y-%m-%d %H:%M:%S UTC")
                        ));
                        ui.add_space(10.0);

                        if ui.button("Connect").clicked() {
                            state.core.connect_requested();
                        }
                    } else if token_state.refresh_token.is_some()
                        && token_state.is_refresh_token_valid()
                    {
                        ui.label("Token status: expired (refresh token available)");
                        ui.add_space(10.0);

                        if ui.button("Connect").clicked() {
                            state.core.connect_requested();
                        }
                    } else {
                        ui.label("Token status: expired");
                        ui.add_space(10.0);
                        self.show_login_button(ui, state);
                    }
                }
                Err(_) => {
                    ui.label("Token status: not logged in");
                    ui.add_space(10.0);
                    self.show_login_button(ui, state);
                }
            }
        });
    }

    fn show_login_button(&mut self, ui: &mut egui::Ui, state: &UIState) {
        let api_settings = &state.settings.chat.api;
        let oauth_mode = &api_settings.oauth_mode;

        let mode_description = match oauth_mode {
            OAuthMode::Default => format!("Using jump server: {}", api_settings.jump_server_url),
            OAuthMode::SelfHosted => format!(
                "Using self-hosted OAuth (client_id: {})",
                api_settings.client_id
            ),
        };
        ui.label(mode_description);
        ui.add_space(10.0);

        if ui.button("Login with osu!").clicked() {
            match self.start_oauth_flow(state) {
                Err(e) => state.core.push_ui_error(Box::new(e), false),
                Ok(auth_url) => ui.ctx().open_url(egui::OpenUrl {
                    url: auth_url,
                    new_tab: true,
                }),
            }
        }

        ui.add_space(10.0);
        ui.label(
            egui::RichText::new(format!("Local callback port: {}", api_settings.local_port))
                .small()
                .weak(),
        );
    }

    fn start_oauth_flow(
        &mut self,
        state: &UIState,
    ) -> Result<String, crate::core::http::oauth_listener::OAuthListenerError> {
        let api_settings = &state.settings.chat.api;

        let params = OAuthFlowParams {
            local_port: api_settings.local_port,
            oauth_mode: api_settings.oauth_mode.clone(),
            client_id: api_settings.client_id.clone(),
            client_secret: api_settings.client_secret.clone(),
            redirect_uri: api_settings.redirect_uri.clone(),
            jump_server_url: api_settings.jump_server_url.clone(),
        };

        self.oauth_flow
            .start_oauth_flow(params, state.core.app_queue_handle().clone())
    }

    pub fn add(&mut self, chat_name: String) {
        self.views
            .insert(chat_name.clone(), ChatView::new(chat_name));
    }

    pub fn remove(&mut self, name: &str) {
        self.views.remove(&name.to_lowercase());
    }
}
