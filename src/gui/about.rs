use eframe::egui;
use steel_core::{VersionString, DEFAULT_DATETIME_FORMAT, DEFAULT_DATE_FORMAT};

use crate::core::settings::{BuiltInSound, Sound};
use crate::core::updater::{State, UpdateState};

use crate::gui::state::UIState;

fn icon_as_texture(ctx: &eframe::egui::Context) -> egui::TextureHandle {
    match crate::png_to_rgba(include_bytes!("../../media/icons/about.png")) {
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
                        self.show_plugins(ui, state);
                        self.show_credits(ui);
                        self.show_update_section(ui, state);
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
            ui.label(format!("{} by TicClick (", crate::VERSION));
            ui.hyperlink_to("source code", "https://github.com/TicClick/steel");
            ui.label("). not affiliated with peppy or ppy Pty Ltd. have fun!");
        });
    }

    fn show_plugins(&self, ui: &mut egui::Ui, state: &UIState) {
        ui.heading("plugins");
        let mut versions = Vec::new();
        for (name, version) in state.plugin_manager.installed() {
            versions.push(format!("- {} {}", name, version));
        }
        ui.label(if versions.is_empty() {
            "no plugins loaded".to_owned()
        } else {
            versions.join("\n")
        });
    }

    fn show_update_section(&self, ui: &mut egui::Ui, state: &UIState) {
        ui.heading("update");

        let UpdateState {
            state: last_action,
            when,
        } = state.updater.state();
        match last_action {
            State::Idle => {
                if ui.button("check for updates").clicked() {
                    state.updater.check_version();
                }
            }
            State::UpdateError(text) => {
                ui.label(format!("failed to fetch updates: {text}"));
                if ui.button("check for updates").clicked() {
                    state.updater.check_version();
                }
            }
            State::FetchingMetadata => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("checking for updates...");
                });
            }
            State::MetadataReady(m) => {
                if crate::VERSION.semver() >= m.tag_name.semver() {
                    let label = format!("no updates, {} is the latest version", m.tag_name);
                    ui.label(label);
                    if ui.button("check again").clicked() {
                        state.updater.check_version();
                    }
                } else {
                    let label = format!(
                        "new release: {} from {}",
                        m.tag_name,
                        m.published_at.format(DEFAULT_DATE_FORMAT),
                    );
                    ui.label(label);
                    ui.horizontal(|ui| {
                        if ui.button("check again").clicked() {
                            state.updater.check_version();
                        }
                        if ui
                            .button(format!(
                                "update {} → {} ({} MB)",
                                crate::VERSION,
                                m.tag_name,
                                m.size() >> 20
                            ))
                            .clicked()
                        {
                            state.updater.download_new_version();
                        }
                    });
                }
            }
            State::FetchingRelease => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("downloading...");
                });
            }
            State::ReleaseReady(m) => {
                ui.label(format!(
                    "{} downloaded, restart the app whenever you wish",
                    m.tag_name
                ));
            }
        }

        if let Some(when) = when {
            let label = format!("- last action: {}", when.format(DEFAULT_DATETIME_FORMAT));
            ui.label(label);
        }
        let autoupdate_status = format!(
            "- automatic updates: {}",
            match state.settings.application.autoupdate.enabled {
                true => "enabled",
                false => "disabled",
            }
        );
        ui.label(autoupdate_status);
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
