use eframe::egui;

use crate::gui::context_menu::url::menu_item_copy_url;

pub struct RegularLink<'link> {
    text: &'link egui::RichText,
    location: &'link str,
}

impl<'link> RegularLink<'link> {
    pub fn new(text: &'link egui::RichText, location: &'link str) -> Self {
        Self { text, location }
    }
}

impl egui::Widget for RegularLink<'_> {
    fn ui(self, ui: &mut egui::Ui) -> eframe::egui::Response {
        let resp = ui.hyperlink_to(self.text.clone(), self.location);
        resp.context_menu(|ui| menu_item_copy_url(ui, self.location));
        resp
    }
}
