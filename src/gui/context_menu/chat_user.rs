use eframe::egui;
use steel_core::{chat::Message, ipc::client::CoreClient};

const ICON_OPEN_CHAT: &str = "💬 Open chat";
const OPEN_CHAT: &str = "Open chat";

const ICON_OPEN_CHAT_USER_PROFILE: &str = "🔎 View profile";
const OPEN_CHAT_USER_PROFILE: &str = "View profile";

const ICON_TRANSLATE_MESSAGE: &str = "🌐 Translate message";
const TRANSLATE_MESSAGE: &str = "Translate message";

const ICON_COPY_USERNAME: &str = "📋 Copy username";
const COPY_USERNAME: &str = "Copy username";

const ICON_COPY_MESSAGE: &str = "📋 Copy message";
const COPY_MESSAGE: &str = "Copy message";

pub fn menu_item_open_chat(
    ui: &mut egui::Ui,
    core_client: &CoreClient,
    show_icon: bool,
    target: &str,
) {
    let text = match show_icon {
        true => ICON_OPEN_CHAT,
        false => OPEN_CHAT,
    };

    if ui.button(text).clicked() {
        core_client.chat_opened(target);
        ui.close();
    }
}

pub fn menu_item_open_chat_user_profile(ui: &mut egui::Ui, show_icon: bool, target: &str) {
    let text = match show_icon {
        true => ICON_OPEN_CHAT_USER_PROFILE,
        false => OPEN_CHAT_USER_PROFILE,
    };

    if ui.button(text).clicked() {
        ui.ctx().open_url(egui::output::OpenUrl::new_tab(format!(
            "https://osu.ppy.sh/users/@{}",
            target
        )));
        ui.close();
    }
}

pub fn menu_item_translate_message(ui: &mut egui::Ui, show_icon: bool, message_text: &str) {
    let text = match show_icon {
        true => ICON_TRANSLATE_MESSAGE,
        false => TRANSLATE_MESSAGE,
    };

    if ui.button(text).clicked() {
        ui.ctx().open_url(egui::output::OpenUrl::new_tab(format!(
            "https://translate.google.com/?sl=auto&tl=en&text={}&op=translate",
            percent_encoding::utf8_percent_encode(message_text, percent_encoding::NON_ALPHANUMERIC)
        )));
        ui.close();
    }
}

pub fn menu_item_copy_message(ui: &mut egui::Ui, show_icon: bool, message: &Message) {
    let text = match show_icon {
        true => ICON_COPY_MESSAGE,
        false => COPY_MESSAGE,
    };

    if ui.button(text).clicked() {
        ui.ctx().copy_text(message.to_string());
        ui.close();
    }
}

pub fn menu_item_copy_username(ui: &mut egui::Ui, show_icon: bool, message: &Message) {
    let text = match show_icon {
        true => ICON_COPY_USERNAME,
        false => COPY_USERNAME,
    };

    if ui.button(text).clicked() {
        ui.ctx().copy_text(message.username.clone());
        ui.close();
    }
}
