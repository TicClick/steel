use eframe::egui::{self, RichText};
use steel_core::chat;

use super::SettingsWindow;
use crate::{
    core::{
        self,
        logging::{
            format_message_for_logging, to_log_action_line_format, to_log_system_line_format,
        },
    },
    gui::state::UIState,
};

impl SettingsWindow {
    pub(super) fn show_logging_tab(&mut self, ui: &mut eframe::egui::Ui, state: &mut UIState) {
        ui.vertical(|ui| {
            ui.heading("chat logging");
            ui.checkbox(
                &mut state.settings.journal.chat_events.enabled,
                "enable chat logging",
            );
            ui.checkbox(
                &mut state.settings.journal.chat_events.log_system_events,
                "log system events",
            );

            ui.horizontal(|ui| {
                ui.label("directory with logs");
                ui.text_edit_singleline(&mut state.settings.journal.chat_events.directory)
                    .on_hover_text_at_pointer("location of all the log files");

                if ui.button("open").clicked()
                    && std::path::Path::new(&state.settings.journal.chat_events.directory).exists()
                {
                    core::os::open_external_directory(
                        &state.settings.journal.chat_events.directory,
                    );
                }
            });

            ui.label("format of a single line");
            ui.text_edit_multiline(&mut state.settings.journal.chat_events.format);

            ui.horizontal_wrapped(|ui| {
                let mut example_chat_log =
                    make_example_chat_log(&state.settings.journal.chat_events.format);
                ui.add_enabled(false, egui::TextEdit::multiline(&mut example_chat_log));
                // ui.label(RichText::new("←").color(ui.visuals().warn_fg_color));
            });

            ui.collapsing("click to show help", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;

                    ui.label("allowed placeholders:\n");

                    ui.label("- ");
                    ui.label(RichText::new("{username}").color(ui.style().visuals.warn_fg_color));
                    ui.label(" - author of the message\n");

                    ui.label("- ");
                    ui.label(RichText::new("{text}").color(ui.style().visuals.warn_fg_color));
                    ui.label(" - message text\n");

                    ui.label("- ");
                    ui.label(RichText::new("{date:").color(ui.style().visuals.warn_fg_color));
                    ui.label(RichText::new("dateformat").color(ui.style().visuals.error_fg_color));
                    ui.label(RichText::new("}").color(ui.style().visuals.warn_fg_color));
                    ui.label(" - message date/time, where ");
                    ui.label(RichText::new("dateformat").color(ui.style().visuals.error_fg_color));
                    ui.label(" is replaced by a format string. example: ");

                    ui.label(RichText::new("{date:").color(ui.style().visuals.warn_fg_color));
                    ui.label(
                        RichText::new("%Y-%m-%d %H:%M:%S").color(ui.style().visuals.error_fg_color),
                    );
                    ui.label(RichText::new("}").color(ui.style().visuals.warn_fg_color));
                    ui.label(" (");
                    ui.hyperlink_to("click for more examples", "https://strftime.net");
                    ui.label(")");
                });
            });
        });

        // TODO(logging): Add a setting for logging system events.
    }
}

fn make_example_chat_log(message_format: &str) -> String {
    let action_message_format = to_log_action_line_format(message_format);
    let system_message_format = to_log_system_line_format(message_format);
    let chat_log = vec![
        (
            chat::Message::new_system("You have joined #sprawl"),
            system_message_format.as_str(),
        ),
        (
            chat::Message::new_text("WilliamGibson", "I think I left my cyberdeck on"),
            message_format,
        ),
        (
            chat::Message::new_action("WilliamGibson", "runs away"),
            action_message_format.as_str(),
        ),
    ];
    chat_log
        .iter()
        .map(|(message, line_format)| format_message_for_logging(line_format, message))
        .collect::<Vec<String>>()
        .join("\n")
}
