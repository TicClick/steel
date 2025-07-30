use eframe::egui::{self, Widget};
use steel_core::{VersionString, DEFAULT_DATETIME_FORMAT, DEFAULT_DATE_FORMAT};

use super::state::UIState;
use steel_core::ipc::updater::{State, UpdateState};

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
                    force_update,
                    ..
                } = &state.update_state;
                match last_action {
                    State::Idle => {
                        if ui.button("check for updates").clicked() {
                            state.core.check_application_updates();
                        }
                    }
                    State::UpdateError(text) => {
                        ui.label(format!("failed to fetch updates: {text}"));
                        if ui.button("check for updates").clicked() {
                            state.core.check_application_updates();
                        }
                    }
                    State::FetchingMetadata => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("checking for updates...");
                        });
                    }
                    State::MetadataReady(m) => {
                        if crate::VERSION.semver() >= m.tag_name.semver() && !force_update {
                            let label = format!("no updates, {} is the latest version", m.tag_name);
                            ui.label(label);
                            if ui.button("check again").clicked() {
                                state.core.check_application_updates();
                            }
                        } else {
                            let label = format!(
                                "next release: {} from {}",
                                m.tag_name,
                                m.published_at.format(DEFAULT_DATE_FORMAT),
                            );
                            ui.label(label);
                            ui.horizontal(|ui| {
                                if ui.button("check again").clicked() {
                                    state.core.check_application_updates();
                                }
                                if ui
                                    .button(format!(
                                        "{}update {} → {} ({} MB)",
                                        if *force_update { "force " } else { "" },
                                        crate::VERSION,
                                        m.tag_name,
                                        m.size() >> 20
                                    ))
                                    .clicked()
                                {
                                    state.core.download_application_update();
                                }
                            });
                        }
                    }
                    State::FetchingRelease(ready_bytes, total_bytes) => {
                        ui.vertical(|ui| {
                            if let Some(total_bytes) = total_bytes {
                                let pct = *ready_bytes as f32 / *total_bytes as f32;
                                egui::ProgressBar::new(*ready_bytes as f32 / *total_bytes as f32)
                                    .text(format!(
                                        "{} MB -- {}%",
                                        total_bytes >> 20,
                                        (pct * 100.0) as usize
                                    ))
                                    .ui(ui);
                            } else {
                                egui::Spinner::new().ui(ui);
                            }
                        });
                        if ui.button("abort").clicked() {
                            state.core.abort_application_update();
                        }
                    }
                    State::ReleaseReady(m) => {
                        ui.label(format!(
                            "{} downloaded, restart the app whenever you wish",
                            m.tag_name
                        ));
                        if ui.button("restart now").clicked() {
                            crate::core::os::restart(state.original_exe_path.clone());
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
