use eframe::egui;

pub fn menu_item_copy_url(ui: &mut egui::Ui, url: &str) {
    if ui.button("Copy URL").clicked() {
        ui.ctx().output_mut(|o| {
            o.copied_text = url.to_owned();
        });
        ui.close_menu();
    }
}
