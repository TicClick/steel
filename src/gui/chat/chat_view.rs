use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::collections::BTreeMap;
use steel_core::settings::chat::ChatPosition;

use steel_core::TextStyle;

use crate::core::chat::{Chat, ChatLike, Message, MessageType};
use crate::gui::state::UIState;
use crate::gui::widgets::chat::message::message_text::ChatMessageText;
use crate::gui::widgets::chat::message::timestamp::TimestampLabel;
use crate::gui::widgets::chat::message::username::{choose_colour, Username};
use crate::gui::widgets::chat::shadow::InnerShadow;
use crate::gui::widgets::chat::unread_marker::UnreadMarker;
use crate::gui::CENTRAL_PANEL_INNER_MARGIN_Y;

use crate::gui::command::{self, CommandHelper};
use crate::gui::HIGHLIGHTS_TAB_NAME;
use crate::gui::SERVER_TAB_NAME;

const MAX_MESSAGE_LENGTH: usize = 450;

pub struct ChatView<'chat> {
    chat: &'chat Chat,
    chat_input: String,
    pub response_widget_id: Option<egui::Id>,
    pub scroll_to: Option<usize>,

    chat_row_height: Option<f32>,
    cached_row_heights: BTreeMap<String, Vec<f32>>,

    // Chat space width -- longer lines will wrap around the window.
    widget_width: f32,

    command_helper: command::CommandHelper,

    // Whether the context menu was open during the previous frame.
    user_context_menu_open: bool,
}

impl<'chat> ChatView<'chat> {
    pub fn new(chat: &'chat Chat) -> Self {
        Self {
            chat,
            chat_input: String::default(),
            response_widget_id: None,
            scroll_to: None,
            chat_row_height: None,
            cached_row_heights: BTreeMap::default(),
            widget_width: 0.0,
            command_helper: CommandHelper::default(),
            user_context_menu_open: false,
        }
    }

    fn maybe_show_unread_marker(
        &mut self,
        ui: &mut egui::Ui,
        state: &UIState,
        channel_name: &str,
        message_index: usize,
        chat_row_height: f32,
    ) -> bool {
        if state.active_chat_tab_name == channel_name {
            if let Some(unread_idx) = state.read_tracker.get_last_read_position(channel_name) {
                if unread_idx == message_index {
                    ui.add(
                        UnreadMarker::new()
                            .ui_height(chat_row_height)
                            .color(state.settings.ui.colours().highlight.clone().into()),
                    );
                    return true;
                }
            }
        }
        false
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &UIState) {
        let interactive = state.is_connected();
        if interactive {
            egui::TopBottomPanel::bottom("input")
                .frame(
                    egui::Frame::central_panel(&ctx.style()).inner_margin(egui::Margin {
                        left: 8,
                        right: 8,
                        top: 0,
                        bottom: 2,
                    }),
                )
                .show(ctx, |ui| {
                    ui.vertical_centered_justified(|ui| {
                        let message_length_exceeded = self.chat_input.len() >= 450;

                        // Special tabs (server messages and highlights) are 1) fake and 2) read-only
                        let mut text_field = egui::TextEdit::singleline(&mut self.chat_input)
                            .char_limit(MAX_MESSAGE_LENGTH)
                            .id_source("chat-input")
                            .hint_text("new message");
                        if message_length_exceeded {
                            text_field = text_field.text_color(egui::Color32::RED);
                        }

                        ui.add_space(8.);
                        let mut response = ui.add(text_field);
                        if message_length_exceeded {
                            response = response.on_hover_text_at_pointer(format!(
                                "messages longer than {MAX_MESSAGE_LENGTH} characters are truncated"
                            ));
                        }
                        self.response_widget_id = Some(response.id);
                        ui.add_space(2.);

                        if response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            && !{
                                let result = self.command_helper.detect_and_run(
                                    self.chat,
                                    &state.core,
                                    &mut self.chat_input,
                                );
                                if result {
                                    self.return_focus(ctx, state);
                                }
                                result
                            }
                        {
                            let trimmed_message = self.chat_input.trim();
                            if !trimmed_message.is_empty() {
                                state
                                    .core
                                    .chat_message_sent(&self.chat.name, trimmed_message);
                            }
                            self.chat_input.clear();
                            response.request_focus();
                        }
                    });
                });
        } else {
            self.response_widget_id = None;
        }

        // Format the chat view as a table with variable row widths (replacement for `ScrollView::show_rows()`,
        // which only understands uniform rows and glitches pretty hard when run in a `show_rows()` + `stick_to_bottom()` combination.
        //
        // Each of the individual display functions (chat/server message or highlight) report the height
        // of a rendered text piece ("galley"), which may be wrapped and therefore occupy several non-wrapped rows.
        //
        // The values are saved for the next drawing cycle, when TableBuilder calculates a proper virtual table.
        // Source of wisdom: https://github.com/emilk/egui/blob/c86bfb6e67abf208dccd7e006ccd9c3675edcc2f/crates/egui_demo_lib/src/demo/table_demo.rs

        egui::CentralPanel::default()
            .frame(
                egui::Frame::central_panel(&ctx.style())
                    .inner_margin(egui::Margin::symmetric(8, CENTRAL_PANEL_INNER_MARGIN_Y)),
            )
            .show(ctx, |ui| {
                if self
                    .command_helper
                    .has_applicable_commands(&self.chat_input)
                {
                    egui::Window::new("chat-command-hint-layer")
                        .title_bar(false)
                        .resizable(false)
                        .pivot(egui::Align2::LEFT_BOTTOM)
                        .fixed_pos(ui.available_rect_before_wrap().left_bottom())
                        .show(ctx, |ui| {
                            self.command_helper.show(
                                ui,
                                self.chat,
                                &state.core,
                                &mut self.chat_input,
                                &self.response_widget_id,
                            );
                        });
                }

                // Disable scrolling to avoid resetting context menu.
                let stick_chat_to_bottom = !self.user_context_menu_open;
                self.user_context_menu_open = false;

                // Chat row spacing, which is by default zero for table rows.
                ui.spacing_mut().item_spacing.y = 2.;
                self.widget_width = ui.available_width();

                let chat_row_height = *self
                    .chat_row_height
                    .get_or_insert_with(|| ui.text_style_height(&egui::TextStyle::Body));

                // Add a fake row with the unread marker to the scroll view hosting the table with chat messages.
                let add_fake_row = matches!(
                    state.settings.chat.behaviour.chat_position,
                    ChatPosition::Bottom
                );
                let chat_row_count = match add_fake_row {
                    true => self.chat.messages.len() + 1,
                    false => self.chat.messages.len(),
                };

                self.cached_row_heights
                    .entry(self.chat.normalized_name.clone())
                    .or_default()
                    .resize(chat_row_count, chat_row_height);

                ui.push_id(&self.chat.normalized_name, |ui| {
                    let view_height = ui.available_height();
                    let view_width = ui.available_width();

                    let mut builder = TableBuilder::new(ui);
                    if let Some(message_id) = self.scroll_to {
                        builder = builder.scroll_to_row(message_id, Some(egui::Align::Center));
                        self.scroll_to = None;
                    } else {
                        builder = builder.stick_to_bottom(stick_chat_to_bottom);
                    }

                    let heights = self.cached_row_heights[&self.chat.normalized_name]
                        .clone()
                        .into_iter();

                    let scroll_area_output = builder
                        .max_scroll_height(view_height)
                        .column(Column::remainder())
                        .auto_shrink([false; 2])
                        .body(|body| match self.chat.normalized_name.as_str() {
                            SERVER_TAB_NAME => {
                                let server_tab_styles = Some(vec![TextStyle::Monospace]);
                                body.heterogeneous_rows(heights, |mut row| {
                                    let row_index = row.index();
                                    row.col(|ui| {
                                        self.show_server_tab_single_message(
                                            ui,
                                            state,
                                            row_index,
                                            &server_tab_styles,
                                        )
                                    });
                                });
                            }

                            HIGHLIGHTS_TAB_NAME => {
                                body.heterogeneous_rows(heights, |mut row| {
                                    let row_index = row.index();
                                    row.col(|ui| {
                                        self.show_highlights_tab_single_message(
                                            ui, state, row_index,
                                        )
                                    });
                                });
                            }

                            _ => {
                                if state.filter.active {
                                    let mut filtered_payload = Vec::new();
                                    let mut filtered_heights = Vec::new();
                                    let mut original_indices = Vec::new();

                                    let heights: Vec<f32> = heights.collect();
                                    for (idx, m) in self.chat.messages.iter().enumerate() {
                                        if state.filter.matches(m) {
                                            filtered_payload.push(m);
                                            filtered_heights.push(heights[idx]);
                                            original_indices.push(idx);
                                        }
                                    }

                                    body.heterogeneous_rows(
                                        filtered_heights.into_iter(),
                                        |mut row| {
                                            let row_index = row.index();
                                            row.col(|ui| {
                                                self.user_context_menu_open |= self
                                                    .show_regular_chat_single_message(
                                                        ui,
                                                        state,
                                                        self.chat,
                                                        &self.chat.messages
                                                            [original_indices[row_index]],
                                                        row_index,
                                                        false,
                                                        0.0,
                                                    );
                                            });
                                        },
                                    );
                                } else {
                                    body.heterogeneous_rows(heights, |mut row| {
                                        let row_index = row.index();
                                        if row.index() == 0 && add_fake_row {
                                            let sz = view_height - chat_row_height - 4.0;
                                            row.col(|ui| {
                                                ui.allocate_space(egui::Vec2 {
                                                    x: view_width,
                                                    y: sz,
                                                });
                                            });
                                            self.cached_row_heights
                                                .get_mut(&self.chat.normalized_name)
                                                .unwrap()[0] = sz;
                                        } else {
                                            let message_idx = match add_fake_row {
                                                true => row_index - 1,
                                                false => row_index,
                                            };
                                            let message = &self.chat.messages[message_idx];

                                            row.col(|ui| {
                                                let marker_shown = self.maybe_show_unread_marker(
                                                    ui,
                                                    state,
                                                    &self.chat.normalized_name,
                                                    message_idx,
                                                    chat_row_height,
                                                );

                                                let marker_height = match marker_shown {
                                                    true => chat_row_height,
                                                    false => 0.0,
                                                };

                                                self.user_context_menu_open |= self
                                                    .show_regular_chat_single_message(
                                                        ui,
                                                        state,
                                                        self.chat,
                                                        message,
                                                        row_index,
                                                        true,
                                                        marker_height,
                                                    );
                                            });
                                        }
                                    });
                                }
                            }
                        });

                    // Decide if a shadow should be drawn.
                    let scroll_view_bottom_y = view_height + scroll_area_output.state.offset.y;
                    let offscreen_area_height =
                        scroll_area_output.content_size.y - scroll_view_bottom_y;

                    // Side comment: it would be nice to get the scroll view position rounded down, so that sub-pixel jitters
                    //   don't disable autoscrolling (apparently this happens when offscreen_area_height >= eps).
                    if offscreen_area_height > 1. {
                        ui.add(InnerShadow::new(20));
                    }
                });
            });
    }

    #[allow(clippy::too_many_arguments)]
    fn show_regular_chat_single_message(
        &mut self,
        ui: &mut egui::Ui,
        state: &UIState,
        chat: &Chat,
        msg: &Message,
        message_index: usize,
        cache_heights: bool,
        extra_height: f32,
    ) -> bool {
        let mut username_styles = Vec::new();
        let mut message_styles = Vec::new();

        #[cfg(feature = "glass")]
        {
            if let Some(st) = state
                .glass
                .style_username(&chat.name, msg, &state.settings.ui.theme)
            {
                username_styles.push(st);
            }
            if let Some(st) = state.glass.style_message(&chat.name, msg) {
                message_styles.push(st);
            }
        }

        if msg.highlight {
            message_styles.push(TextStyle::Highlight(
                state.settings.ui.colours().highlight.clone().into(),
            ));
        }

        if matches!(msg.r#type, MessageType::Action) {
            message_styles.push(TextStyle::Italics);
        }

        username_styles.push(TextStyle::Coloured(choose_colour(
            &msg.username,
            &state.settings,
        )));

        let mut context_menu_active = false;
        let updated_height = ui
            .push_id(format!("{}_row_{}", chat.name, message_index), |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x /= 2.;
                    ui.set_max_width(self.widget_width);

                    ui.add(TimestampLabel::new(&msg.time, &None));

                    match msg.r#type {
                        MessageType::Action | MessageType::Text => {
                            let response = ui.add(Username::new(
                                msg,
                                &chat.name,
                                &Some(username_styles),
                                &state.core,
                                state.is_connected(),
                                #[cfg(feature = "glass")]
                                &state.glass,
                            ));

                            context_menu_active |= response.context_menu_opened();

                            ui.add(ChatMessageText::new(
                                msg.chunks.as_ref().unwrap(),
                                &Some(message_styles),
                                &state.settings.chat.behaviour,
                                &state.core,
                            ))
                            .rect
                            .height()
                        }
                        MessageType::System => format_system_message(ui, msg),
                    }
                })
            })
            .inner
            .inner;

        if cache_heights {
            self.cached_row_heights
                .get_mut(&chat.normalized_name)
                .unwrap()[message_index] = updated_height + extra_height;
        }

        context_menu_active
    }

    fn show_highlights_tab_single_message(
        &mut self,
        ui: &mut egui::Ui,
        state: &UIState,
        message_index: usize,
    ) {
        let (chat_name, msg) = &state.read_tracker.ordered_highlights()[message_index];
        let updated_height = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x /= 2.;

                ui.add(TimestampLabel::new(&msg.time, &None));

                format_chat_name(ui, state, chat_name, msg);

                ui.add(Username::new(
                    msg,
                    chat_name,
                    &None,
                    &state.core,
                    state.is_connected(),
                    #[cfg(feature = "glass")]
                    &state.glass,
                ));

                ui.add(ChatMessageText::new(
                    msg.chunks.as_ref().unwrap(),
                    &None,
                    &state.settings.chat.behaviour,
                    &state.core,
                ))
                .rect
                .height()
            })
            .inner;
        self.cached_row_heights
            .get_mut(HIGHLIGHTS_TAB_NAME)
            .unwrap()[message_index] = updated_height;
    }

    fn show_server_tab_single_message(
        &mut self,
        ui: &mut egui::Ui,
        state: &UIState,
        message_index: usize,
        styles: &Option<Vec<TextStyle>>,
    ) {
        let msg = &state.server_messages[message_index];
        let updated_height = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x /= 2.;
                ui.add(TimestampLabel::new(&msg.time, styles));
                ui.add(ChatMessageText::new(
                    msg.chunks.as_ref().unwrap(),
                    styles,
                    &state.settings.chat.behaviour,
                    &state.core,
                ))
            })
            .response
            .rect
            .height();

        self.cached_row_heights.get_mut(SERVER_TAB_NAME).unwrap()[message_index] = updated_height;
    }

    pub fn return_focus(&mut self, ctx: &egui::Context, state: &UIState) {
        if state.is_connected() {
            ctx.memory_mut(|mem| {
                if mem.focused().is_none() {
                    if let Some(id) = self.response_widget_id {
                        mem.request_focus(id);
                    }
                }
            });
        }
    }

    pub fn insert_user_mention(&mut self, ctx: &egui::Context, username: String) {
        if let Some(text_edit_id) = self.response_widget_id {
            if let Some(mut state) = egui::TextEdit::load_state(ctx, text_edit_id) {
                let pos = match state.cursor.char_range() {
                    None => 0,
                    Some(cc) => std::cmp::min(cc.primary.index, cc.secondary.index),
                };

                if let Some(cc) = state.cursor.char_range() {
                    let start = std::cmp::min(cc.primary.index, cc.secondary.index);
                    let end = std::cmp::max(cc.primary.index, cc.secondary.index);
                    if start != end {
                        self.chat_input.replace_range(start..end, "");
                    }
                }

                let insertion = if self.chat_input.is_empty() {
                    format!("{username}: ")
                } else if pos == self.chat_input.chars().count() {
                    if self.chat_input.ends_with(' ') {
                        username.to_owned()
                    } else {
                        format!(" {username}")
                    }
                } else {
                    username.to_owned()
                };
                self.chat_input.insert_str(pos, &insertion);
                let ccursor = egui::text::CCursor::new(pos + insertion.len());
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                state.store(ctx, text_edit_id);
            }
        }
    }
}

fn format_system_message(ui: &mut egui::Ui, msg: &Message) -> f32 {
    ui.add_enabled(false, egui::Button::new(&msg.text))
        .rect
        .height()
}

fn format_chat_name(ui: &mut egui::Ui, state: &UIState, chat_name: &str, message: &Message) {
    let chat_button = ui.button(match chat_name.is_channel() {
        true => chat_name,
        false => "(PM)",
    });

    if state.validate_reference(chat_name, message) {
        let mut switch_requested = chat_button.clicked();
        chat_button.context_menu(|ui| {
            if ui.button("Go to message").clicked() {
                switch_requested = true;
                ui.close();
            }
        });
        if switch_requested {
            state.core.chat_switch_requested(chat_name, message.id);
        }
    }
}
