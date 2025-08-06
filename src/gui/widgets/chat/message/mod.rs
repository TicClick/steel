use eframe::egui;

use egui::{
    Widget, Color32
};
use steel_core::{chat::{Chat, Message, MessageType}, ipc::client::CoreClient, settings::Settings, TextStyle};

use crate::gui::{widgets::chat::{message::{message_text::ChatMessageText, timestamp::TimestampLabel, username::Username}, unread_marker::UnreadMarker}, HIGHLIGHTS_TAB_NAME, SERVER_TAB_NAME};

pub mod message_text;
pub mod timestamp;
pub mod username;

pub enum ChatViewRow<'chat, 'msg> {
    Filler {
        chat: &'chat Chat,
        view_width: f32
    },
    UnreadMarker {
        chat: &'chat Chat,
        chat_row_height: f32,
        color: Color32,
    },
    Message {
        chat: &'chat Chat,
        message: &'msg Message,
        message_styles: Option<Vec<TextStyle>>,
        username_styles: Option<Vec<TextStyle>>,
        core: &'msg CoreClient,
        settings: &'msg Settings,
    }
}

impl<'chat, 'msg> ChatViewRow<'chat, 'msg> {
    pub fn filler(chat: &'chat Chat, view_width: f32) -> Self {
        Self::Filler { chat, view_width }
    }

    pub fn unread_marker(chat: &'chat Chat, chat_row_height: f32, color: Color32) -> Self {
        Self::UnreadMarker { chat, chat_row_height, color }
    }

    pub fn message(
        chat: &'chat Chat,
        message: &'msg Message,
        message_styles: Option<Vec<TextStyle>>,
        username_styles: Option<Vec<TextStyle>>,
        core: &'msg CoreClient,
        settings: &'msg Settings,
    ) -> Self {
        Self::Message { chat, message, message_styles, username_styles, core, settings }
    }
}

impl Widget for &mut ChatViewRow<'_, '_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        match self {
            ChatViewRow::Filler { view_width, .. } => {
                ui.allocate_response(egui::Vec2 {
                    x: *view_width,
                    y: 0.0,
                }, egui::Sense::hover())
            }

            ChatViewRow::UnreadMarker { chat_row_height, color, .. } => {
                ui.add(
                UnreadMarker::new()
                    .ui_height(*chat_row_height)
                    .color(*color)
                )
            }

            ChatViewRow::Message {
                chat,
                message,
                message_styles,
                username_styles,
                core,
                settings,
            } => {
                match chat.normalized_name.as_str() {
                    SERVER_TAB_NAME => {
                        let styles = vec![TextStyle::Monospace];
                        ui
                            .horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x /= 2.;
                                ui.add(TimestampLabel::new(&message.time, Some(&styles)));
                                ui.add(ChatMessageText::new(
                                    message.chunks.as_ref().unwrap(),
                                    Some(&styles),
                                    &settings.chat.behaviour,
                                    &core,
                                ))
                            })
                            .response
                    }

                    HIGHLIGHTS_TAB_NAME => {
                        ui.response()
                    }

                    _ => {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x /= 2.;

                            // ui.set_max_width(self.widget_width);

                            ui.add(TimestampLabel::new(&message.time, None));

                            match message.r#type {
                                MessageType::Action | MessageType::Text => {
                                    let _response = ui.add(Username::new(
                                        message,
                                        &chat.name,
                                        username_styles.as_ref(),
                                        &core,
                                        true, // state.is_connected()
                                        #[cfg(feature = "glass")]
                                        &state.glass,
                                    ));

                                    // context_menu_active |= response.context_menu_opened();

                                    ui.add(ChatMessageText::new(
                                        message.chunks.as_ref().unwrap(),
                                        message_styles.as_ref(),
                                        &settings.chat.behaviour,
                                        &core,
                                    ))
                                }

                                MessageType::System => {
                                    ui.add_enabled(false, egui::Button::new(&message.text))
                                }
                            }
                        }).inner
                    }
                }
            }
        }
    }
}
