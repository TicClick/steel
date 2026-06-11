use eframe::egui;
use steel_core::chat::{ChatLike, ChatType};
use steel_core::ipc::server::SettingsPatch;

use crate::gui::state::UIState;

const ICON_ADD_TO_FAVOURITES: &str = "❤️ Add to favourites";
const ADD_TO_FAVOURITES: &str = "Add to favourites";

const ICON_REMOVE_FROM_FAVOURITES: &str = "❤️ Remove from favourites";
const REMOVE_FROM_FAVOURITES: &str = "Remove from favourites";

const ICON_CLEAR_CHAT_TAB: &str = "🧼 Clear chat";
const CLEAR_CHAT_TAB: &str = "Clear chat";

const ICON_LEAVE_CHANNEL: &str = "✖ Leave";
const LEAVE_CHANNEL: &str = "Leave";

const ICON_CLOSE_CHAT: &str = "✖ Close";
const CLOSE_CHAT: &str = "Close";

const ICON_DETACH_CHAT: &str = "🗗 Open in a separate window";
const DETACH_CHAT: &str = "Open in a separate window";

const ICON_REATTACH_CHAT: &str = "🗖 Return to the main window";
const REATTACH_CHAT: &str = "Return to the main window";

pub fn menu_item_add_to_favourites(
    ui: &mut egui::Ui,
    state: &mut UIState,
    show_icon: bool,
    target: &str,
) {
    let text = match show_icon {
        true => ICON_ADD_TO_FAVOURITES,
        false => ADD_TO_FAVOURITES,
    };

    if ui.button(text).clicked() {
        state
            .core
            .settings_patched(SettingsPatch::AutojoinAdded(target.to_owned()));
        ui.close();
    }
}

pub fn menu_item_remove_from_favourites(
    ui: &mut egui::Ui,
    state: &mut UIState,
    show_icon: bool,
    target: &str,
) {
    let text = match show_icon {
        true => ICON_REMOVE_FROM_FAVOURITES,
        false => REMOVE_FROM_FAVOURITES,
    };

    if ui.button(text).clicked() {
        state
            .core
            .settings_patched(SettingsPatch::AutojoinRemoved(target.to_owned()));
        ui.close();
    }
}

pub fn menu_item_clear_chat_tab(ui: &mut egui::Ui, state: &UIState, show_icon: bool, target: &str) {
    let text = match show_icon {
        true => ICON_CLEAR_CHAT_TAB,
        false => CLEAR_CHAT_TAB,
    };

    if ui.button(text).clicked() {
        state.core.chat_tab_cleared(target, target.chat_type());
        ui.close();
    }
}

pub fn menu_item_detach_chat(
    ui: &mut egui::Ui,
    state: &mut UIState,
    show_icon: bool,
    target: &str,
) {
    let text = match show_icon {
        true => ICON_DETACH_CHAT,
        false => DETACH_CHAT,
    };

    if ui.button(text).clicked() {
        state.detach_chat(target);
        ui.close();
    }
}

pub fn menu_item_reattach_chat(
    ui: &mut egui::Ui,
    state: &mut UIState,
    show_icon: bool,
    target: &str,
) {
    let text = match show_icon {
        true => ICON_REATTACH_CHAT,
        false => REATTACH_CHAT,
    };

    if ui.button(text).clicked() {
        state.reattach_chat(target);
        state
            .core
            .chat_switch_requested(target, target.chat_type(), None);
        ui.close();
    }
}

pub fn menu_item_close_chat(
    ui: &mut egui::Ui,
    state: &mut UIState,
    show_icon: bool,
    target: &str,
    mode: &ChatType,
) {
    let text = match mode {
        ChatType::Channel => match show_icon {
            true => ICON_LEAVE_CHANNEL,
            false => LEAVE_CHANNEL,
        },
        ChatType::Person => match show_icon {
            true => ICON_CLOSE_CHAT,
            false => CLOSE_CHAT,
        },
        ChatType::System => return,
    };

    if ui.button(text).clicked() {
        state.core.chat_tab_closed(target, target.chat_type());
        ui.close();
    }
}
