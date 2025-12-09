use eframe::egui;
use steel_core::chat::{ChatLike, ChatType};

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
        state.settings.chat.autojoin.push(target.to_owned());
        // TODO: this should be done elsewhere, in a centralized manner, I'm just being lazy right now
        state.core.settings_updated(&state.settings);
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
        state.settings.chat.autojoin.retain(|s| s != target);
        // TODO: this should be done elsewhere, in a centralized manner, I'm just being lazy right now
        state.core.settings_updated(&state.settings);
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
