use eframe::egui;
use std::cell::Cell;

use egui::{Color32, Widget};
use steel_core::{
    chat::{Chat, ChatLike, Message, MessageType},
    ipc::client::CoreClient,
    settings::Settings,
    TextStyle,
};

use crate::gui::{
    widgets::chat::{
        message::{message_text::ChatMessageText, timestamp::TimestampLabel, username::Username},
        unread_marker::UnreadMarker,
    },
    CENTRAL_PANEL_INNER_MARGIN_X, HIGHLIGHTS_TAB_NAME, SERVER_TAB_NAME,
};

pub mod message_text;
pub mod timestamp;
pub mod username;

pub enum ChatViewRow<'chat, 'msg> {
    Filler {
        chat: &'chat Chat,
        view_width: f32,
        view_height: f32,
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
        is_user_menu_opened: Cell<bool>,
        #[cfg(feature = "glass")]
        glass: &'msg glass::Glass,
        has_highlight: bool,
        search_result_color: Option<Color32>,
    },
}

impl<'chat, 'msg> ChatViewRow<'chat, 'msg> {
    pub fn filler(chat: &'chat Chat, view_width: f32, view_height: f32) -> Self {
        Self::Filler {
            chat,
            view_width,
            view_height,
        }
    }

    pub fn unread_marker(chat: &'chat Chat, chat_row_height: f32, color: Color32) -> Self {
        Self::UnreadMarker {
            chat,
            chat_row_height,
            color,
        }
    }

    pub fn message(
        chat: &'chat Chat,
        message: &'msg Message,
        message_styles: Option<Vec<TextStyle>>,
        username_styles: Option<Vec<TextStyle>>,
        core: &'msg CoreClient,
        settings: &'msg Settings,
        #[cfg(feature = "glass")] glass: &'msg glass::Glass,
        has_highlight: bool,
        search_result_color: Option<Color32>,
    ) -> Self {
        Self::Message {
            chat,
            message,
            message_styles,
            username_styles,
            core,
            settings,
            is_user_menu_opened: Cell::new(false),
            #[cfg(feature = "glass")]
            glass,
            has_highlight,
            search_result_color,
        }
    }

    pub fn is_user_menu_opened(&self) -> bool {
        match self {
            ChatViewRow::Message {
                is_user_menu_opened,
                ..
            } => is_user_menu_opened.get(),
            _ => false,
        }
    }
}

impl Widget for &mut ChatViewRow<'_, '_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        match self {
            ChatViewRow::Filler {
                view_width,
                view_height,
                ..
            } => ui.allocate_response(
                egui::Vec2 {
                    x: *view_width,
                    y: *view_height,
                },
                egui::Sense::hover(),
            ),

            ChatViewRow::UnreadMarker {
                chat_row_height,
                color,
                ..
            } => ui.add(
                UnreadMarker::new()
                    .ui_height(*chat_row_height)
                    .color(*color),
            ),

            ChatViewRow::Message {
                chat,
                message,
                message_styles,
                username_styles,
                core,
                settings,
                is_user_menu_opened,
                #[cfg(feature = "glass")]
                glass,
                has_highlight,
                search_result_color,
            } => {
                let resp = match chat.normalized_name.as_str() {
                    SERVER_TAB_NAME => {
                        let styles = vec![TextStyle::Monospace];
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x /= 2.;
                            ui.add(TimestampLabel::new(&message.time, Some(&styles)));
                            ui.add(ChatMessageText::new(
                                message.chunks.as_ref(),
                                Some(&styles),
                                &settings.chat.behaviour,
                                core,
                            ))
                        })
                        .response
                    }

                    _ => {
                        let has_highlight = *has_highlight;
                        let search_result_color = *search_result_color;

                        let mut inner_ui = |ui: &mut egui::Ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing.x /= 2.;

                                ui.add(TimestampLabel::new(&message.time, None));

                                if chat.normalized_name.as_str() == HIGHLIGHTS_TAB_NAME {
                                    insert_original_chat_reference(ui, core, message);
                                }

                                match message.r#type {
                                    MessageType::Action | MessageType::Text => {
                                        let response = ui.add(Username::new(
                                            message,
                                            &chat.name,
                                            username_styles.as_ref(),
                                            core,
                                            settings,
                                            #[cfg(feature = "glass")]
                                            glass,
                                        ));

                                        *is_user_menu_opened.get_mut() |=
                                            response.context_menu_opened();

                                        ui.add(ChatMessageText::new(
                                            message.chunks.as_ref(),
                                            message_styles.as_ref(),
                                            &settings.chat.behaviour,
                                            core,
                                        ))
                                    }

                                    MessageType::System => {
                                        ui.add_enabled(false, egui::Button::new(&message.text))
                                    }
                                }
                            })
                            .inner
                        };

                        // Allocate the full width and add horizontal padding for all messages,
                        // because the background block on highlighted messages looks terrible otherwise.
                        let full_width = ui.available_width();

                        let bg_color = if let Some(search_color) = search_result_color {
                            let search_color: Color32 = search_color;
                            Color32::from_rgba_unmultiplied(
                                search_color.r(),
                                search_color.g(),
                                search_color.b(),
                                settings.ui.highlight_bg_opacity,
                            )
                        } else if has_highlight {
                            let highlight_color: Color32 =
                                settings.ui.colours().highlight.clone().into();
                            Color32::from_rgba_unmultiplied(
                                highlight_color.r(),
                                highlight_color.g(),
                                highlight_color.b(),
                                settings.ui.highlight_bg_opacity,
                            )
                        } else {
                            Color32::TRANSPARENT
                        };

                        egui::Frame::new()
                            .fill(bg_color)
                            .inner_margin(egui::Margin {
                                left: 4,
                                right: 4,
                                top: 0,
                                bottom: 0,
                            })
                            .show(ui, |ui| {
                                ui.set_min_width(
                                    full_width - 2.0 * CENTRAL_PANEL_INNER_MARGIN_X as f32,
                                );
                                inner_ui(ui)
                            })
                            .inner
                    }
                };

                resp
            }
        }
    }
}

fn insert_original_chat_reference(ui: &mut egui::Ui, core_client: &CoreClient, message: &Message) {
    let original_chat = match &message.original_chat {
        Some(chat_name) => chat_name,
        None => return,
    };

    let chat_button = ui.button(match original_chat.is_channel() {
        true => original_chat,
        false => "(PM)",
    });

    let mut switch_requested = chat_button.clicked();
    chat_button.context_menu(|ui| {
        if ui.button("Go to message").clicked() {
            switch_requested = true;
            ui.close();
        }
    });
    if switch_requested {
        core_client.chat_switch_requested(original_chat, original_chat.chat_type(), message.id);
    }
}
