use std::error::Error;

use eframe::egui;
use steel_core::ipc::client::CoreClient;

#[derive(Debug)]
pub struct ErrorWrapper {
    pub error: Box<dyn Error>,
    pub is_fatal: bool,
}

pub struct ErrorPopup {
    core: CoreClient,
    errors: Vec<ErrorWrapper>,
}

pub enum ModalAction {
    Close,
    Exit,
    Restart,
}

impl ErrorPopup {
    pub fn new(core: CoreClient) -> Self {
        Self {
            core,
            errors: Vec::new(),
        }
    }

    pub fn push_error(&mut self, error: Box<dyn Error>, is_fatal: bool) {
        self.errors.push(ErrorWrapper { error, is_fatal });
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        if let Some(most_recent_error) = self.errors.last() {
            let response = egui::Modal::new("errors-modal".into()).show(ctx, |ui| {
                ui.set_max_width((ctx.screen_rect().width() * 0.5).min(450.0));
                ui.heading("Application error");
                ui.label(format!("{}", most_recent_error.error));
                if let Some(source_error) = most_recent_error.error.source() {
                    ui.label(format!("Caused by the following error: {:?}", source_error));
                }

                ui.horizontal(|ui| match most_recent_error.is_fatal {
                    true => {
                        ui.add_space(ui.available_width() - 60.0);
                        if ui.button("restart").clicked() {
                            Some(ModalAction::Restart)
                        } else if ui
                            .add(egui::Button::new("exit").fill(egui::Color32::DARK_RED))
                            .clicked()
                        {
                            Some(ModalAction::Exit)
                        } else {
                            None
                        }
                    }
                    false => {
                        ui.add_space(ui.available_width() - 60.0);
                        if ui.button("close").clicked() {
                            Some(ModalAction::Close)
                        } else {
                            None
                        }
                    }
                })
                .inner
            });

            let topmost_modal_dismissed =
                response.should_close() && !response.backdrop_response.clicked();

            match response.inner {
                None => {
                    if topmost_modal_dismissed && !most_recent_error.is_fatal {
                        self.errors.pop();
                    }
                }
                Some(action) => match action {
                    ModalAction::Close => {
                        self.errors.pop();
                    }
                    ModalAction::Restart => {
                        self.core.restart_requested(None);
                    }
                    ModalAction::Exit => {
                        self.core.exit_requested(None, 1);
                    }
                },
            }
        }
    }
}
