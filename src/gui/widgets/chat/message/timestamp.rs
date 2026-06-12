use chrono::{DateTime, Local};
use eframe::egui::{self, Widget};
use steel_core::chat::Message;
use steel_core::{TextStyle, DEFAULT_DATETIME_FORMAT};

use crate::gui::DecoratedText;

pub trait FormattedTimestamp {
    fn formatted_date_local(&self) -> String;
    fn formatted_date_utc(&self) -> String;
}

impl FormattedTimestamp for DateTime<Local> {
    fn formatted_date_local(&self) -> String {
        self.format(DEFAULT_DATETIME_FORMAT).to_string()
    }

    fn formatted_date_utc(&self) -> String {
        self.naive_utc().format(DEFAULT_DATETIME_FORMAT).to_string()
    }
}

pub struct TimestampLabel<'msg> {
    message: &'msg Message,
    styles: Option<&'msg Vec<TextStyle>>,
}

impl<'msg> TimestampLabel<'msg> {
    pub fn new(message: &'msg Message, styles: Option<&'msg Vec<TextStyle>>) -> Self {
        Self { message, styles }
    }
}

impl Widget for TimestampLabel<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let timestamp =
            egui::RichText::new(&self.message.time_formatted).with_styles(self.styles);
        ui.label(timestamp).on_hover_ui_at_pointer(|ui| {
            ui.vertical(|ui| {
                ui.label(format!(
                    "{} (local time zone)",
                    self.message.time.formatted_date_local()
                ));
                ui.label(format!("{} (UTC)", self.message.time.formatted_date_utc()));
            });
        })
    }
}
