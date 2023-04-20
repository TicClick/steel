use eframe::egui::{Color32, RichText, Ui};

pub mod about;
pub mod chat;
pub mod chat_tabs;
pub mod highlights;
pub mod menu;
pub mod settings;
pub mod state;
pub mod window;

const HIGHLIGHTS_TAB_NAME: &str = "highlights";
const SERVER_TAB_NAME: &str = "server";

pub fn validate_username(input: &str) -> Result<(), &'static str> {
    match input.contains(|ch: char| !ch.is_ascii_alphanumeric() && !"-_ []".contains(ch)) {
        true => Err("invalid username"),
        false => Ok(()),
    }
}

pub fn validate_channel_name(input: &str) -> Result<(), &'static str> {
    match input.contains(|ch: char| !ch.is_ascii_alphanumeric() && ch != '#') {
        true => Err("invalid channel name"),
        false => Ok(()),
    }
}

pub fn chat_validation_error(ui: &mut Ui, error_text: &str) {
    ui.label(RichText::new(error_text).color(Color32::RED))
        .on_hover_text_at_pointer(
            "usernames and channel names cannot contain punctuation or non-alphanumeric characters",
        );
}
