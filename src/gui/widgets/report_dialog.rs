use eframe::egui;
use steel_core::chat::ChatLike;

use crate::gui::state::UIState;

pub fn show_report_dialog(ctx: &egui::Context, ui_state: &mut UIState) {
    let mut should_close = false;

    if let Some(dialog) = &mut ui_state.report_dialog {
        let mut is_open = true;
        let just_opened = dialog.just_opened;

        egui::Window::new("report to moderators")
            .open(&mut is_open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.label("your issue with ");
                        ui.label(egui::RichText::new(format!("{}:", dialog.username)).strong());
                    });

                    let response = ui.add(
                        egui::TextEdit::multiline(&mut dialog.reason)
                            .id_salt("report-dialog-reason-input")
                            .desired_rows(2)
                            .char_limit(400)
                            .return_key(Some(egui::KeyboardShortcut {
                                modifiers: egui::Modifiers::NONE,
                                logical_key: egui::Key::Enter,
                            }))
                            .hint_text("max 400 characters"),
                    );

                    if just_opened {
                        response.request_focus();
                    }

                    let is_enter_pressed =
                        response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                    ui.horizontal(|ui| {
                        if ui.button("send report").clicked() || is_enter_pressed {
                            ui_state.core.chat_message_sent(
                                &dialog.chat_name,
                                dialog.chat_name.chat_type(),
                                &format!(
                                    "!report {} {}",
                                    dialog.username.to_lowercase().replace(" ", "_"),
                                    dialog.reason
                                ),
                            );
                            should_close = true;
                        }

                        if ui.button("cancel").clicked() {
                            should_close = true;
                        }
                    });
                });
            });

        if !is_open || should_close {
            ui_state.report_dialog = None;
        } else if just_opened {
            dialog.just_opened = false;
        }
    }
}
