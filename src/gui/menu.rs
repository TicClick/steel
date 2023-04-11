use eframe::egui;

use crate::core::settings;
use crate::{app::AppMessageIn, core::irc::ConnectionStatus};

use super::UIState;

#[derive(Default)]
pub struct Menu {
    pub show_settings: bool,
    pub show_about: bool,
}

impl Menu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dialogs_visible(&self) -> bool {
        self.show_settings || self.show_about
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &mut UIState) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                if let Some(theme) = ctx.style().visuals.light_dark_small_toggle_button(ui) {
                    let old_theme = state.settings.ui.theme.clone();
                    state.settings.ui.theme = if theme.dark_mode {
                        settings::ThemeMode::Dark
                    } else {
                        settings::ThemeMode::Light
                    };
                    if state.settings.ui.theme != old_theme {
                        state
                            .app_queue_handle
                            .blocking_send(AppMessageIn::UISettingsUpdated(state.settings.clone()))
                            .unwrap();
                    }
                }

                if ui.button("settings").clicked() {
                    self.show_settings = !self.show_settings;
                }

                let (action, enabled) = match state.connection {
                    ConnectionStatus::Disconnected => ("connect", true),
                    ConnectionStatus::InProgress => ("connecting...", false),
                    ConnectionStatus::Connected => ("disconnect", true),
                };
                if ui.add_enabled(enabled, egui::Button::new(action)).clicked() {
                    match state.connection {
                        ConnectionStatus::Disconnected => {
                            state
                                .app_queue_handle
                                .blocking_send(AppMessageIn::UIConnectRequested)
                                .unwrap();
                        }
                        ConnectionStatus::InProgress => (),
                        ConnectionStatus::Connected => {
                            state
                                .app_queue_handle
                                .blocking_send(AppMessageIn::UIDisconnectRequested)
                                .unwrap();
                        }
                    }
                }

                if ui.button("about").clicked() {
                    self.show_about = !self.show_about;
                }
            });
        });
    }
}
