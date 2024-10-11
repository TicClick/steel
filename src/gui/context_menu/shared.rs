use std::path::Path;

use eframe::egui;

use crate::{core::logging::chat_log_path, gui::state::UIState};

const ICON_OPEN_CHAT_LOG: &str = "📜 Open chat log";
const OPEN_CHAT_LOG: &str = "Open chat log";

pub fn menu_item_open_chat_log(ui: &mut egui::Ui, state: &UIState, show_icon: bool, target: &str) {
    let chat_path = chat_log_path(
        &Path::new(&state.settings.logging.chat.directory).to_path_buf(),
        target,
    );

    if ui
        .add(egui::Button::new(if show_icon {
            ICON_OPEN_CHAT_LOG
        } else {
            OPEN_CHAT_LOG
        }))
        .clicked()
    {
        state.core.open_fs_path(chat_path.to_str().unwrap());
        ui.close_menu();
    }
}
