use eframe::egui;
use steel_core::ipc::client::CoreClient;

const ICON_OPEN_CHAT_LOG: &str = "📜 Open chat log";
const OPEN_CHAT_LOG: &str = "Open chat log";

pub fn menu_item_open_chat_log(
    ui: &mut egui::Ui,
    core_client: &CoreClient,
    show_icon: bool,
    target: &str,
) {
    let text = match show_icon {
        true => ICON_OPEN_CHAT_LOG,
        false => OPEN_CHAT_LOG,
    };

    if ui.add(egui::Button::new(text)).clicked() {
        core_client.open_chat_log(target);
        ui.close();
    }
}
