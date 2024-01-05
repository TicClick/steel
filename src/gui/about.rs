use eframe::egui;

use crate::core::settings::{BuiltInSound, Sound};
use crate::gui::state::UIState;

fn icon_as_texture(ctx: &eframe::egui::Context) -> egui::TextureHandle {
    match crate::gui::png_to_rgba(include_bytes!("../../media/icons/about.png")) {
        Ok((data, (width, height))) => {
            let image =
                egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &data);
            ctx.load_texture("about-icon", image, egui::TextureOptions::default())
        }
        Err(_) => panic!("failed to load the large app icon"),
    }
}

#[derive(Default)]
pub struct About {
    texture: Option<egui::TextureHandle>,
    rotation: f32,
}

impl About {
    pub fn show(&mut self, ctx: &eframe::egui::Context, state: &mut UIState, is_open: &mut bool) {
        egui::Window::new("about")
            .open(is_open)
            .default_size((420., 200.))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    self.show_app_icon(ctx, ui, state);
                    ui.vertical(|ui| {
                        self.show_initial_section(ui);

                        #[cfg(feature = "glass")]
                        state.glass.show_about(ui);

                        self.show_credits(ui);
                    });
                });
            });
    }

    fn show_app_icon(
        &mut self,
        ctx: &eframe::egui::Context,
        ui: &mut egui::Ui,
        state: &mut UIState,
    ) {
        let texture = self.texture.get_or_insert_with(|| icon_as_texture(ctx));
        let img = egui::Image::new(texture.id(), texture.size_vec2() / 2.0)
            .sense(egui::Sense::click())
            .rotate(self.rotation, egui::Vec2::splat(0.5));
        let resp = ui.add(img);
        if resp.clicked() || resp.secondary_clicked() {
            state.sound_player.play(&Sound::BuiltIn(BuiltInSound::Tick));
            self.rotation += match resp.clicked() {
                true => 0.02,
                false => -0.02,
            }
        }
    }

    fn show_initial_section(&self, ui: &mut egui::Ui) {
        ui.heading("steel");
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label(format!("v{} by TicClick (", crate::VERSION));
            ui.hyperlink_to("source code", "https://github.com/TicClick/steel");
            ui.label("). not affiliated with peppy or ppy Pty Ltd. have fun!");
        });
    }

    fn show_credits(&self, ui: &mut egui::Ui) {
        ui.collapsing("credits", |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("- interface: ");
                ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                ui.label("\n");

                ui.label("- fonts: ");
                ui.hyperlink_to("Google Noto", "https://fonts.google.com/noto");
                ui.label("\n");

                ui.label("- cool packages: ");
                ui.hyperlink_to(
                    "different vendors",
                    "https://github.com/TicClick/steel/blob/master/Cargo.toml",
                );
                ui.label("\n");

                ui.label("- app icon: ");
                ui.hyperlink_to("Freepik", "https://www.flaticon.com/free-icon/rust_5883364");
                ui.label(", sounds: ");
                ui.hyperlink_to(
                    "various artists",
                    "https://github.com/TicClick/steel/blob/master/media/sounds/ATTRIBUTION",
                );
            });
        });
    }
}
