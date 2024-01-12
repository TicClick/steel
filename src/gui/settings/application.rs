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
            ui.checkbox(
                &mut state.settings.application.autoupdate.enabled,
                "enable automatic updates",
            )
            .on_hover_text_at_pointer(hint);
        });

        if autoupdate != state.settings.application.autoupdate.enabled {
            state
                .updater
                .enable_autoupdate(state.settings.application.autoupdate.enabled);
        }
    }
}
