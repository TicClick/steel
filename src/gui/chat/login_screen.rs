use eframe::egui::{self, Widget};
use steel_core::settings::OAuthMode;

use crate::{
    core::http::{
        oauth_flow::{OAuthFlowManager, OAuthFlowParams},
        token_storage::load_token_state,
    },
    gui::state::UIState,
};

pub struct LoginScreen<'a> {
    state: &'a UIState,
    oauth_flow: &'a OAuthFlowManager,
}

impl<'a> LoginScreen<'a> {
    pub fn new(state: &'a UIState, oauth_flow: &'a OAuthFlowManager) -> Self {
        Self { state, oauth_flow }
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
}

impl Widget for LoginScreen<'_> {
    fn ui(mut self, ui: &mut egui::Ui) -> egui::Response {
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
                            self.state.core.connect_requested();
                        }
                    } else if token_state.refresh_token.is_some()
                        && token_state.is_refresh_token_valid()
                    {
                        ui.label("Token status: expired (refresh token available)");
                        ui.add_space(10.0);

                        if ui.button("Connect").clicked() {
                            self.state.core.connect_requested();
                        }
                    } else {
                        ui.label("Token status: expired");
                        ui.add_space(10.0);
                        self.show_login_button(ui, self.state);
                    }
                }
                Err(_) => {
                    ui.label("Token status: not logged in");
                    ui.add_space(10.0);
                    self.show_login_button(ui, self.state);
                }
            }
        })
        .response
    }
}
