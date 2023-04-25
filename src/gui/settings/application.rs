use eframe::egui;

use super::SettingsWindow;
use crate::gui::state::UIState;

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
            ui.checkbox(&mut state.settings.application.autoupdate.enabled, "enable automatic updates")
                .on_hover_text_at_pointer(hint);

            ui.heading("plugins");
            if !state.settings.application.plugins.enabled {
                ui.label(
                    "plugins are third-party modules (.dll, .so) which add extra functions. \
                to activate a plugin, place its file into the application's folder and restart it.\n\
                \n\
                beware that plugins can THEORETICALLY do anything in the application, or on your PC, that you can, so \
                only add them if you know and trust their authors.",
                );
            }

            ui.checkbox(
                &mut state.settings.application.plugins.enabled,
                "enable plugins",
            )
            .on_hover_text_at_pointer("requires application restart");
        });

        if autoupdate != state.settings.application.autoupdate.enabled {
            state
                .updater
                .enable_autoupdate(state.settings.application.autoupdate.enabled);
        }
    }
}
