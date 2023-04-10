use eframe::egui;

use crate::core::updater::UpdateState;

use super::UIState;
use crate::VersionString;

#[derive(Default)]
pub struct About {}

impl About {
    pub fn show(&mut self, ctx: &eframe::egui::Context, state: &UIState, is_open: &mut bool) {
        egui::Window::new("about").open(is_open).show(ctx, |ui| {
            ui.vertical(|ui| {
                self.show_initial_section(ui);
                self.show_credits(ui);
                self.show_update_section(ui, state);
            });
        });
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

    fn show_update_section(&self, ui: &mut egui::Ui, state: &UIState) {
        ui.heading("update");
        match state.updater.state() {
            UpdateState::Idle => {
                if ui.button("check for updates").clicked() {
                    state.updater.check_version();
                }
            }
            UpdateState::UpdateError(text) => {
                ui.label(
                    egui::RichText::new(format!(
                        "failed to fetch updates: {} (runtime.log may have details)",
                        text
                    ))
                    .color(egui::Color32::RED),
                );
                if ui.button("check for updates").clicked() {
                    state.updater.check_version();
                }
            }
            UpdateState::FetchingMetadata => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("checking for updates...");
                });
            }
            UpdateState::MetadataReady(m) => {
                if crate::VERSION.semver() < m.tag_name.semver() {
                    ui.label(
                        egui::RichText::new(format!(
                            "no updates, {} is the latest version",
                            m.tag_name
                        ))
                        .color(egui::Color32::GREEN),
                    );
                    if ui.button("check again").clicked() {
                        state.updater.check_version();
                    }
                } else {
                    ui.label(
                        egui::RichText::new(format!(
                            "last release: {} from {}",
                            m.tag_name,
                            m.published_at.format("%Y-%m-%d")
                        ))
                        .color(egui::Color32::YELLOW),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("check again").clicked() {
                            state.updater.check_version();
                        }
                        if ui
                            .button(format!("update {} → {}", crate::VERSION, m.tag_name))
                            .clicked()
                        {
                            state.updater.download_new_version();
                        }
                    });
                }
            }
            UpdateState::FetchingRelease => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("downloading...");
                });
            }
            UpdateState::ReleaseReady(m) => {
                ui.label(
                    egui::RichText::new(format!(
                        "{} downloaded -- restart the app whenever you wish",
                        m.tag_name
                    ))
                    .color(egui::Color32::GREEN),
                );
            }
        }
    }

    fn show_credits(&self, ui: &mut egui::Ui) {
        ui.heading("credits");
        ui.collapsing("libraries", |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("- interface: ");
                ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                ui.label(" by Emil Ernerfeldt, MIT License\n");

                ui.label("- fonts: ");
                ui.hyperlink_to("Google Noto", "https://fonts.google.com/noto");
                ui.label(", SIL Open Font License\n");

                ui.label("- a lot of ");
                ui.hyperlink_to(
                    "cool packages",
                    "https://github.com/TicClick/steel/blob/master/Cargo.toml",
                );
                ui.label("\n");
            });
        });
    }
}
