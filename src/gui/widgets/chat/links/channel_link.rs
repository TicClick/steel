use eframe::egui;
use steel_core::ipc::client::CoreClient;

pub struct ChannelLink<'link, 'app> {
    display_text: &'link egui::RichText,
    location: &'link str,

    core_client: &'app CoreClient,
}

impl<'link, 'app> ChannelLink<'link, 'app> {
    pub fn new(
        display_text: &'link egui::RichText,
        location: &'link str,
        core_client: &'app CoreClient
    ) -> Self {
        Self {
            display_text,
            location,
            core_client,
        }
    }
}

impl egui::Widget for ChannelLink<'_, '_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let on_hover_text = format!("Open {}", self.location);
        let resp = ui
            .link(self.display_text.clone())
            .on_hover_text_at_pointer(on_hover_text);

        if resp.clicked() {
            self.core_client.chat_opened(self.location)
        }

        resp
    }
}
