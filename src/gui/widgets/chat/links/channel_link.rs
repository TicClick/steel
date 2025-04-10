use eframe::egui;

use crate::gui::state::UIState;

pub struct ChannelLink<'link> {
    display_text: &'link egui::RichText,
    location: &'link str,

    ui_state: &'link UIState,
}

impl<'link> ChannelLink<'link> {
    pub fn new(
        display_text: &'link egui::RichText,
        location: &'link str,
        ui_state: &'link UIState,
    ) -> Self {
        Self {
            display_text,
            location,
            ui_state,
        }
    }
}

impl egui::Widget for ChannelLink<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let on_hover_text = format!("Open {}", self.location);
        let resp = ui
            .link(self.display_text.clone())
            .on_hover_text_at_pointer(on_hover_text);

        if resp.clicked() {
            match self.ui_state.has_chat(self.location) {
                true => self
                    .ui_state
                    .core
                    .chat_switch_requested(self.location, None),
                false => self.ui_state.core.chat_opened(self.location),
            }
        }

        resp
    }
}
