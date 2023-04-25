use eframe::egui;
use steel_core::{VersionString, DEFAULT_DATETIME_FORMAT, DEFAULT_DATE_FORMAT};

use super::state::UIState;
use crate::core::updater::{State, UpdateState};

#[derive(Default)]
pub struct UpdateWindow {}

impl UpdateWindow {
    pub fn show(&mut self, ctx: &egui::Context, state: &mut UIState, is_open: &mut bool) {
        egui::Window::new("update")
            .open(is_open)
            .default_size((250., 200.))
            .show(ctx, |ui| {
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
                        if ui.button("restart now").clicked() {
                            crate::core::os::restart();
                        }
                    }
                }

                ui.separator();
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
            });
    }
}
