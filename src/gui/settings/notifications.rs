use eframe::egui;

use super::SettingsWindow;
use crate::gui::{state::UIState, HIGHLIGHTS_SEPARATOR};
use steel_core::settings::{BuiltInSound, Sound};

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

            ui.heading("notification sound");

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
                        let mut c = ui
                            .selectable_value(
                                &mut self.notifications_builtin_sound,
                                BuiltInSound::Bell,
                                BuiltInSound::Bell.to_string(),
                            )
                            .clicked();
                        c = c
                            || ui
                                .selectable_value(
                                    &mut self.notifications_builtin_sound,
                                    BuiltInSound::DoubleBell,
                                    BuiltInSound::DoubleBell.to_string(),
                                )
                                .clicked();

                        // \o /
                        if format!(
                            "{:x}",
                            md5::compute(state.settings.chat.irc.username.as_bytes())
                        ) == "cdb6d5ffca1edf2659aa721c19ccec1b"
                        {
                            c = c
                                || ui
                                    .selectable_value(
                                        &mut self.notifications_builtin_sound,
                                        BuiltInSound::PartyHorn,
                                        BuiltInSound::PartyHorn.to_string(),
                                    )
                                    .clicked();
                        }
                        c = c
                            || ui
                                .selectable_value(
                                    &mut self.notifications_builtin_sound,
                                    BuiltInSound::Ping,
                                    BuiltInSound::Ping.to_string(),
                                )
                                .clicked();
                        c = c
                            || ui
                                .selectable_value(
                                    &mut self.notifications_builtin_sound,
                                    BuiltInSound::TwoTone,
                                    BuiltInSound::TwoTone.to_string(),
                                )
                                .clicked();

                        c
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

            // TODO: implement custom sound picker
            // There is no centralized egui-based file dialog solution. nfd2 pulls up GTK3, tinyfiledialogs seems to crash when used naively.
            // Need to either implement it myself, or check potential leads from https://github.com/emilk/egui/issues/270
        });
    }
}
