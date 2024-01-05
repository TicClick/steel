use eframe::egui::{Color32, RichText, Ui};
use std::collections::BTreeSet;

pub mod about;
pub mod chat;
pub mod chat_tabs;
pub mod highlights;
pub mod menu;
pub mod settings;
pub mod state;
pub mod update_window;
pub mod window;

const HIGHLIGHTS_TAB_NAME: &str = "$highlights";
const SERVER_TAB_NAME: &str = "$server";
const HIGHLIGHTS_SEPARATOR: &str = ", ";

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum TextStyle {
    Bold,
    Italics,
    Underline,
    Strikethrough,
    Monospace,

    Highlight,
}

pub trait DecoratedText {
    fn with_styles(
        self,
        decorations: &Option<BTreeSet<TextStyle>>,
        settings: &steel_core::settings::Settings,
    ) -> RichText;
}

impl DecoratedText for RichText {
    fn with_styles(
        mut self,
        decorations: &Option<BTreeSet<TextStyle>>,
        settings: &steel_core::settings::Settings,
    ) -> RichText {
        match decorations {
            None => self,
            Some(decorations) => {
                for d in decorations {
                    match d {
                        TextStyle::Bold => self = self.strong(),
                        TextStyle::Italics => self = self.italics(),
                        TextStyle::Monospace => self = self.monospace(),
                        TextStyle::Underline => self = self.underline(),
                        TextStyle::Strikethrough => self = self.strikethrough(),

                        TextStyle::Highlight => {
                            self = self.color(settings.ui.colours().highlight.clone())
                        }
                    }
                }
                self
            }
        }
    }
}

impl DecoratedText for String {
    fn with_styles(
        self,
        decorations: &Option<BTreeSet<TextStyle>>,
        settings: &steel_core::settings::Settings,
    ) -> RichText {
        RichText::new(self).with_styles(decorations, settings)
    }
}

impl DecoratedText for &str {
    fn with_styles(
        self,
        decorations: &Option<BTreeSet<TextStyle>>,
        settings: &steel_core::settings::Settings,
    ) -> RichText {
        RichText::new(self).with_styles(decorations, settings)
    }
}

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

pub fn png_to_rgba(bytes: &[u8]) -> Result<(Vec<u8>, (u32, u32)), png::DecodingError> {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    match reader.next_frame(&mut buf) {
        Ok(_) => Ok((buf, reader.info().size())),
        Err(e) => Err(e),
    }
}
