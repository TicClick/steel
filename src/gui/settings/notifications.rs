use eframe::egui;

use super::SettingsWindow;
use crate::gui::{state::UIState, HIGHLIGHTS_SEPARATOR};
use steel_core::settings::{BuiltInSound, NotificationStyle, Sound};

impl SettingsWindow {
    pub(super) fn show_notifications_tab(
        &mut self,
        ui: &mut eframe::egui::Ui,
        state: &mut UIState,
    ) {
        ui.vertical(|ui| {
            ui.heading("highlights");
            ui.label("keywords");
            if self.highlights_input.is_empty() {
                self.highlights_input = state
                    .settings
                    .notifications
                    .highlights
                    .words
                    .join(HIGHLIGHTS_SEPARATOR);
            }
            let hl = egui::TextEdit::multiline(&mut self.highlights_input).hint_text(
                "words or phrases, separated by comma and space. example: one, 2 3 4, five",
            );
            if ui
                .add(hl)
                .on_hover_text_at_pointer(
                    "list of words or phrases which will trigger notifications:\n\
                - must be separated by comma and space (example: one, 2 3 4, five)\n\
                - exact case does not matter\n\
                - full match required (\"ha\" will not highlight a message with \"haha\")",
                )
                .changed()
            {
                state.update_highlights(&self.highlights_input);
            }

            ui.heading("sound");

            ui.radio_value(
                &mut state.settings.notifications.highlights.sound,
                None,
                "don't play anything",
            );

            let builtin_sound_chosen = matches!(
                state.settings.notifications.highlights.sound,
                Some(Sound::BuiltIn(_))
            );
            ui.horizontal(|ui| {
                let mut response = ui.radio(builtin_sound_chosen, "built-in");
                let inner = egui::ComboBox::from_id_salt("sound")
                    .selected_text(self.notifications_builtin_sound.to_string())
                    .show_ui(ui, |ui| {
                        let mut options = vec![BuiltInSound::Bell, BuiltInSound::DoubleBell];

                        // \o /
                        if format!(
                            "{:x}",
                            md5::compute(state.settings.chat.irc.username.as_bytes())
                        ) == "cdb6d5ffca1edf2659aa721c19ccec1b"
                        {
                            options.push(BuiltInSound::PartyHorn);
                        }

                        options.extend([BuiltInSound::Ping, BuiltInSound::TwoTone]);

                        let dropdown_height = options.len() as f32
                            * (ui.text_style_height(&egui::TextStyle::Body)
                                + 2. * ui.spacing().item_spacing.y);

                        egui::ScrollArea::vertical()
                            .auto_shrink([true, false])
                            .max_height(dropdown_height)
                            .show(ui, |ui| {
                                let mut clicked = false;
                                for o in options {
                                    clicked |= ui
                                        .selectable_value(
                                            &mut self.notifications_builtin_sound,
                                            o.clone(),
                                            o.to_string(),
                                        )
                                        .clicked();
                                }
                                clicked
                            })
                            .inner
                    });

                if response.clicked() || inner.inner.unwrap_or(false) {
                    state.settings.notifications.highlights.sound =
                        Some(Sound::BuiltIn(self.notifications_builtin_sound.clone()));
                    response.mark_changed();
                }

                let test_button = egui::Button::new("🔈");
                let button_clicked = match state.sound_player.functional() {
                    true => ui.add(test_button).clicked(),
                    false => {
                        let error_text = match state.sound_player.initialization_error() {
                            None => "unknown initialization error".into(),
                            Some(e) => e.to_string(),
                        };
                        ui.add_enabled(false, test_button)
                            .on_disabled_hover_text(error_text);
                        false
                    }
                };

                if button_clicked {
                    state
                        .sound_player
                        .play(&Sound::BuiltIn(self.notifications_builtin_sound.clone()));
                }
            });

            let checkbox_sound_when_unfocused = egui::Checkbox::new(
                &mut state.settings.notifications.sound_only_when_unfocused,
                "play sounds only when client is not focused"
            );

            ui.add_enabled(builtin_sound_chosen, checkbox_sound_when_unfocused)
                .on_hover_text_at_pointer("when enabled, notification sounds will only play when the application is not in focus");

            // TODO: implement custom sound picker
            // There is no centralized egui-based file dialog solution. nfd2 pulls up GTK3, tinyfiledialogs seems to crash when used naively.
            // Need to either implement it myself, or check potential leads from https://github.com/emilk/egui/issues/270

            ui.heading("visuals");

            ui.horizontal(|ui| {
                ui.label("notification style");
                egui::ComboBox::from_id_salt("notification_style")
                    .selected_text(self.notifications_style.to_string())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.notifications_style,
                            NotificationStyle::Moderate,
                            NotificationStyle::Moderate.to_string(),
                        );

                        if cfg!(not(target_os = "linux")) {
                            ui.selectable_value(
                                &mut self.notifications_style,
                                NotificationStyle::Intensive,
                                NotificationStyle::Intensive.to_string(),
                            );
                        }
                    });

                if self.notifications_style != state.settings.notifications.notification_style {
                    state.settings.notifications.notification_style = self.notifications_style.clone();
                }
            });

            ui.label("notify on");
            ui.indent("notify-checkboxes", |ui| {
                ui.checkbox(&mut state.settings.notifications.notification_events.highlights, "highlights");
                ui.checkbox(&mut state.settings.notifications.notification_events.private_messages, "private messages");
            });

            let is_timeout_enabled = matches!(self.notifications_style, NotificationStyle::Intensive) && cfg!(not(target_os = "linux"));
            let response = ui.add_enabled_ui(is_timeout_enabled, |ui| {
                ui.checkbox(&mut state.settings.notifications.enable_notification_timeout, "stop notification after timeout");

                ui.add_enabled_ui(state.settings.notifications.enable_notification_timeout, |ui| {
                    ui.indent("notification-timeout-slider", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("timeout duration");
                            let mut timeout = state.settings.notifications.notification_timeout_seconds as f32;
                            let slider = egui::Slider::new(&mut timeout, 1.0..=60.0).suffix(" seconds").integer();
                            if ui.add(slider).changed() {
                                state.settings.notifications.notification_timeout_seconds = timeout as u32;
                            }
                        });
                    });
                });
            })
                .response
                .on_disabled_hover_text(
                    if cfg!(target_os = "linux") {
                        "this setting is unavailable on Linux"
                    } else {
                        "this setting is inapplicable for selected notifcation style"
                    }
                );
        });
    }
}
