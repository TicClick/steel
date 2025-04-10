use eframe::egui;

use crate::gui::state::UIState;
use steel_core::chat::ChatLike;

use super::regular_link::RegularLink;

pub struct ChatLink<'link> {
    chat_name: &'link str,
    display_text: &'link egui::RichText,
    location: &'link str,

    ui_state: &'link UIState,
}

impl<'link> ChatLink<'link> {
    pub fn new(
        chat_name: &'link str,
        display_text: &'link egui::RichText,
        location: &'link str,
        ui_state: &'link UIState,
    ) -> Self {
        Self {
            chat_name,
            display_text,
            location,
            ui_state,
        }
    }
}

impl egui::Widget for ChatLink<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        match self.ui_state.settings.chat.behaviour.handle_osu_chat_links {
            false => ui.add(RegularLink::new(self.display_text, self.location)),
            true => {
                let title = match self.location.is_channel() {
                    true => format!("Open {}", self.location),
                    false => format!("Chat with {}", self.location),
                };
                let resp = ui
                    .link(self.display_text.clone())
                    .on_hover_text_at_pointer(title);

                if resp.clicked() {
                    match self.ui_state.has_chat(self.chat_name) {
                        true => self
                            .ui_state
                            .core
                            .chat_switch_requested(self.chat_name, None),
                        false => self.ui_state.core.chat_opened(self.chat_name),
                    }
                }
                resp
            }
        }
    }
}
