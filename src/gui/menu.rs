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

                let (action, enabled, colour) = match state.connection {
                    ConnectionStatus::Disconnected { .. } => {
                        ("connect".to_owned(), true, egui::Color32::GREEN)
                    }
                    ConnectionStatus::InProgress => {
                        ("connecting...".to_owned(), false, egui::Color32::YELLOW)
                    }
                    ConnectionStatus::Scheduled(when) => {
                        let action = format!(
                            "reconnecting ({}s)",
                            (when - chrono::Local::now()).num_seconds()
                        );
                        (action, false, egui::Color32::YELLOW)
                    }
                    ConnectionStatus::Connected => {
                        ("disconnect".to_owned(), true, egui::Color32::RED)
                    }
                };
                if ui
                    .add_enabled(
                        enabled,
                        egui::Button::new(egui::RichText::new(action).color(colour)),
                    )
                    .clicked()
                {
                    match state.connection {
                        ConnectionStatus::Disconnected { .. } => {
                            state
                                .app_queue_handle
                                .blocking_send(AppMessageIn::UIConnectRequested)
                                .unwrap();
                        }
                        ConnectionStatus::InProgress | ConnectionStatus::Scheduled(_) => (),
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
