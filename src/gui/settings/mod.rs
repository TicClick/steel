#![allow(clippy::upper_case_acronyms)]

mod application;
mod chat;
mod logging;
mod notifications;
mod ui;

use std::cmp;

use eframe::egui;

use crate::core::settings;

use crate::gui::state::UIState;

#[derive(Clone, Debug, Default, cmp::PartialEq, cmp::Eq)]
pub enum Tab {
    Application,
    #[default]
    Chat,
    Interface,
    Notifications,
    #[cfg(feature = "glass")]
    Moderation,
    Logging,
}

#[derive(Default)]
pub struct SettingsWindow {
    active_tab: Tab,
    autojoin: chat::AutojoinSection,
    username_input: String,
    username_colour_input: settings::Colour,
    visible_password: bool,

    highlights_input: String,
    notifications_builtin_sound: settings::BuiltInSound,
    notifications_style: settings::NotificationStyle,
    text_row_height: Option<f32>,
}

impl SettingsWindow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, ctx: &eframe::egui::Context, state: &mut UIState, is_open: &mut bool) {
        if let Some(settings::Sound::BuiltIn(sound)) =
            &state.settings.notifications.highlights.sound
        {
            self.notifications_builtin_sound = sound.clone();
        }

        self.notifications_style = state.settings.notifications.notification_style.clone();

        let mut save_clicked = false;
        egui::Window::new("settings").open(is_open).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.selectable_value(&mut self.active_tab, Tab::Application, "application");
                    ui.selectable_value(&mut self.active_tab, Tab::Chat, "chat");
                    ui.selectable_value(&mut self.active_tab, Tab::Interface, "interface");
                    ui.selectable_value(&mut self.active_tab, Tab::Notifications, "notifications");

                    #[cfg(feature = "glass")]
                    ui.selectable_value(&mut self.active_tab, Tab::Moderation, "moderation");

                    ui.selectable_value(&mut self.active_tab, Tab::Logging, "logging");
                });

                ui.separator();

                match self.active_tab {
                    Tab::Application => self.show_application_tab(ctx, ui, state),
                    Tab::Chat => self.show_chat_tab(ctx, ui, state),
                    Tab::Interface => self.show_ui_tab(ui, state),
                    Tab::Notifications => self.show_notifications_tab(ui, state),

                    #[cfg(feature = "glass")]
                    Tab::Moderation => state.glass.show_ui(ui, &state.settings.ui.theme),

                    Tab::Logging => self.show_logging_tab(ui, state),
                }
            });

            ui.add_space(10.);

            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    let reset_button = egui::Button::new("reset");
                    if ui
                        .add(reset_button)
                        .on_hover_text_at_pointer("double click to discard changes")
                        .double_clicked()
                    {
                        state.core.settings_requested();

                        #[cfg(feature = "glass")]
                        state.core.glass_settings_requested();
                    }

                    if ui
                        .button("save settings")
                        .on_hover_text_at_pointer("save active settings to file")
                        .clicked()
                    {
                        state.core.settings_updated(&state.settings.clone());

                        #[cfg(feature = "glass")]
                        state.core.glass_settings_updated(state.glass.settings_as_yaml());

                        save_clicked = true;
                    }
                });
            });
        });
        if save_clicked {
            *is_open = false;
        }
    }
}
