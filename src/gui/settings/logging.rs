use eframe::egui::{self, RichText};
use steel_core::{chat, settings::logging::ChatLoggingFormats};

use super::SettingsWindow;
use crate::core::logging::format_message_for_logging;
use crate::gui::state::UIState;

impl SettingsWindow {
    pub(super) fn show_logging_tab(&mut self, ui: &mut eframe::egui::Ui, state: &mut UIState) {
        ui.vertical(|ui| {
            ui.heading("application logging");
            ui.horizontal(|ui| {
                ui.label("level");
                egui::ComboBox::from_id_salt("app-logging-level")
                    .selected_text(state.settings.logging.application.level.as_str())
                    .show_ui(ui, |ui| {
                        for level in [
                            log::LevelFilter::Debug,
                            log::LevelFilter::Info,
                            log::LevelFilter::Warn,
                            log::LevelFilter::Error,
                        ] {
                            ui.selectable_value(
                                &mut state.settings.logging.application.level,
                                level,
                                level.to_string(),
                            );
                        }
                    });
            });

            ui.heading("chat logging");

            ui.checkbox(
                &mut state.settings.logging.chat.enabled,
                "enable chat logging",
            );
            ui.checkbox(
                &mut state.settings.logging.chat.log_system_events,
                "log system events",
            );

            ui.heading("logs directory");
            ui.horizontal(|ui| {
                ui.label("location (will be created)");
                ui.text_edit_singleline(&mut state.settings.logging.chat.directory)
                    .on_hover_text_at_pointer("both relative and absolute paths are supported");

                if ui
                    .button("open")
                    .on_hover_text_at_pointer(
                        "open the directory. if it doesn't exist yet, nothing will happen",
                    )
                    .clicked()
                    && std::path::Path::new(&state.settings.logging.chat.directory).exists()
                {
                    state
                        .core
                        .open_fs_path(&state.settings.logging.chat.directory);
                }
            });

            ui.heading("formats");

            ui.collapsing("help", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;

                    ui.label("date formats: ");
                    ui.hyperlink("https://strftime.net");
                    ui.label("\n");

                    ui.label("placeholders:\n");
                    ui.label("- ");
                    ui.label(RichText::new("{username}").color(ui.style().visuals.warn_fg_color));
                    ui.label(" - author of the message\n");

                    ui.label("- ");
                    ui.label(RichText::new("{text}").color(ui.style().visuals.warn_fg_color));
                    ui.label(" - message text\n");

                    ui.label("- ");
                    ui.label(RichText::new("{date}").color(ui.style().visuals.warn_fg_color));
                    ui.label(" - message date and/or time");
                });
            });

            ui.horizontal(|ui| {
                ui.label("regular message");
                ui.text_edit_singleline(&mut state.settings.logging.chat.format.user_message);
            });
            ui.horizontal(|ui| {
                ui.label("user action");
                ui.text_edit_singleline(&mut state.settings.logging.chat.format.user_action);
            });
            ui.horizontal(|ui| {
                ui.label("system message");
                ui.text_edit_singleline(&mut state.settings.logging.chat.format.system_message);
            });
            ui.horizontal(|ui| {
                ui.label("date and time");
                ui.text_edit_singleline(&mut state.settings.logging.chat.format.date);
            });

            ui.horizontal_wrapped(|ui| {
                let mut example_chat_log =
                    make_example_chat_log(&state.settings.logging.chat.format);
                ui.add_enabled(false, egui::TextEdit::multiline(&mut example_chat_log));
            });
        });
    }
}

fn make_example_chat_log(formats: &ChatLoggingFormats) -> String {
    let chat_log = vec![
        (
            chat::Message::new_system("You have joined #sprawl"),
            &formats.system_message,
        ),
        (
            chat::Message::new_text("WilliamGibson", "I think I left my cyberdeck on"),
            &formats.user_message,
        ),
        (
            chat::Message::new_action("WilliamGibson", "runs away"),
            &formats.user_action,
        ),
    ];
    chat_log
        .iter()
        .map(|(message, line_format)| {
            format_message_for_logging(&formats.date, line_format, message)
        })
        .collect::<Vec<String>>()
        .join("\n")
}
