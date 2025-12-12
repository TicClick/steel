use eframe::egui;

use steel_core::{chat::ChatLike, ipc::client::CoreClient, settings::chat::ChatBehaviour};

use super::regular_link::RegularLink;

pub struct ChatLink<'link, 'app> {
    chat_name: &'link str,
    display_text: &'link egui::RichText,
    location: &'link str,

    behaviour: &'app ChatBehaviour,
    core_client: &'app CoreClient,
}

impl<'link, 'app> ChatLink<'link, 'app> {
    pub fn new(
        chat_name: &'link str,
        display_text: &'link egui::RichText,
        location: &'link str,
        behaviour: &'app ChatBehaviour,
        core_client: &'app CoreClient,
    ) -> Self {
        Self {
            chat_name,
            display_text,
            location,
            behaviour,
            core_client,
        }
    }
}

impl egui::Widget for ChatLink<'_, '_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        match self.behaviour.handle_osu_chat_links {
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
                    self.core_client
                        .chat_opened(self.chat_name, self.chat_name.chat_type())
                }
                resp
            }
        }
    }
}
