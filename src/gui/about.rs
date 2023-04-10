use eframe::egui;

use super::UIState;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
pub struct About {}

impl About {
    pub fn show(&mut self, ctx: &eframe::egui::Context, _state: &UIState, is_open: &mut bool) {
        egui::Window::new("about").open(is_open).show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.heading("steel");
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label(format!("v{} by TicClick (", VERSION));
                    ui.hyperlink_to("source code", "https://github.com/TicClick/steel");
                    ui.label("). not affiliated with peppy or ppy Pty Ltd. have fun!");
                });

                ui.collapsing("credits", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.label("- UI library: ");
                        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                        ui.label(" by Emil Ernerfeldt, MIT License\n");

                        ui.label("- fonts: ");
                        ui.hyperlink_to("Google Noto", "https://fonts.google.com/noto");
                        ui.label(", SIL Open Font License");
                    });
                });
            });
        });
    }
}
