use eframe::egui;

pub fn menu_item_copy_url(ui: &mut egui::Ui, url: &str) {
    if ui.button("Copy URL").clicked() {
        ui.ctx().copy_text(url.to_owned());
        ui.close();
    }
}
