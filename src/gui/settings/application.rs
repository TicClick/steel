use eframe::egui;

use super::SettingsWindow;
use crate::{core::updater, gui::state::UIState};

impl SettingsWindow {
    pub(super) fn show_application_tab(
        &mut self,
        _ctx: &egui::Context,
        ui: &mut eframe::egui::Ui,
        state: &mut UIState,
    ) {
        let autoupdate = state.settings.application.autoupdate.enabled;

        ui.vertical(|ui| {
            ui.heading("general");
            let hint = format!(
                "checked every {} minutes. to apply an update, restart the application",
                crate::core::updater::AUTOUPDATE_INTERVAL_MINUTES
            );
            ui.checkbox(
                &mut state.settings.application.autoupdate.enabled,
                "enable automatic updates",
            )
            .on_hover_text_at_pointer(hint);

            ui.label("update URL");
            let url_response = ui.add(
                egui::TextEdit::multiline(&mut state.settings.application.autoupdate.url)
                    .hint_text("should point to release metadata"),
            );

            // Apply URL changes automatically when the text field loses focus
            if url_response.lost_focus() {
                state
                    .core
                    .update_settings_changed(&state.settings.application.autoupdate);
            }

            ui.horizontal(|ui| {
                if ui
                    .button("revert")
                    .on_hover_text_at_pointer("roll back the URL to its default value")
                    .clicked()
                {
                    state.settings.application.autoupdate.url = updater::default_update_url();
                    state
                        .core
                        .update_settings_changed(&state.settings.application.autoupdate);
                }
            });

            if let Some(test_result) = &state.update_state.url_test_result {
                match test_result {
                    Ok(_) => {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;

                            ui.label("URL validation: ");
                            ui.label(egui::RichText::new("OK").color(egui::Color32::DARK_GREEN));
                        });
                    }
                    Err(why) => {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                ui.label("URL validation: ");
                                ui.label(
                                    egui::RichText::new("FAIL").color(egui::Color32::DARK_RED),
                                );
                            });
                            ui.label(why.to_string());
                        });
                    }
                }
            }
        });

        if autoupdate != state.settings.application.autoupdate.enabled {
            state
                .core
                .update_settings_changed(&state.settings.application.autoupdate);
        }
    }
}
