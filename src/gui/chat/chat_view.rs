use eframe::egui;
use egui_extras::{Column, TableBuilder};
use steel_core::chat::Chat;
use steel_core::settings::chat::ChatPosition;

use steel_core::TextStyle;

use crate::core::chat::MessageType;
use crate::gui::state::UIState;
use crate::gui::widgets::chat::message::username::choose_colour;
use crate::gui::widgets::chat::message::ChatViewRow;
use crate::gui::widgets::chat::shadow::InnerShadow;
use crate::gui::{CENTRAL_PANEL_INNER_MARGIN_X, CENTRAL_PANEL_INNER_MARGIN_Y};

use crate::gui::command::{self, CommandHelper};

const MAX_MESSAGE_LENGTH: usize = 450;

pub struct ChatView {
    chat_name: String,
    chat_input: String,
    pub response_widget_id: Option<egui::Id>,
    pub scroll_to: Option<usize>,
    cached_row_heights: Vec<f32>,
    command_helper: command::CommandHelper,

    // Whether the context menu was open during the previous frame.
    user_context_menu_open: bool,
}

impl ChatView {
    pub fn new(chat_name: String) -> Self {
        Self {
            chat_name,
            chat_input: String::default(),
            response_widget_id: None,
            scroll_to: None,
            cached_row_heights: Vec::default(),
            command_helper: CommandHelper::default(),
            user_context_menu_open: false,
        }
    }

    fn egui_id(&self, prefix: &str) -> String {
        format!("{}-{}", prefix, self.chat_name)
    }

    fn show_chat_input(&mut self, ctx: &egui::Context, state: &UIState, chat: &Chat) {
        let text_field_id = self.egui_id("chat-input");
        egui::TopBottomPanel::bottom(self.egui_id("input-panel"))
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
                    let message_length_exceeded = self.chat_input.len() >= MAX_MESSAGE_LENGTH;

                    // Special tabs (server messages and highlights) are 1) fake and 2) read-only
                    let mut text_field = egui::TextEdit::singleline(&mut self.chat_input)
                        .char_limit(MAX_MESSAGE_LENGTH)
                        .id_source(text_field_id)
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
                                chat,
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
                            state.core.chat_message_sent(&chat.name, trimmed_message);
                        }
                        self.chat_input.clear();
                        response.request_focus();
                    }
                });
            });
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &UIState) {
        let chat = match state.find_chat(&self.chat_name) {
            Some(chat) => chat,
            None => return, // Chat not found, nothing to show
        };

        match state.is_connected() {
            true => self.show_chat_input(ctx, state, chat),
            false => self.response_widget_id = None,
        }

        let add_filler_space = matches!(
            state.settings.chat.behaviour.chat_position,
            ChatPosition::Bottom
        );

        let chat_row_height = 18.0; // ui.text_style_height(&egui::TextStyle::Body) + 2x spacing
        let chat_view_size = ctx.available_rect().size()
            - egui::vec2(
                (2 * CENTRAL_PANEL_INNER_MARGIN_X).into(),
                (2 * CENTRAL_PANEL_INNER_MARGIN_Y).into(),
            );

        let mut rows: Vec<ChatViewRow> = Vec::new();

        if add_filler_space {
            rows.push(ChatViewRow::filler(
                chat,
                chat_view_size.x,
                chat_view_size.y - chat_row_height - 4.0,
            ));
        }

        let mut unread_marker_active = false;
        for (idx, message) in chat.messages.iter().enumerate() {
            if state.settings.chat.behaviour.track_unread_messages
                && state.active_chat_tab_name == chat.normalized_name
                && chat.prev_unread_pointer == idx
            {
                rows.push(ChatViewRow::unread_marker(
                    chat,
                    chat_row_height,
                    state.settings.ui.colours().highlight.clone().into(),
                ));
                unread_marker_active = true;
            }

            let mut username_styles = Vec::new();
            let mut message_styles = Vec::new();

            #[cfg(feature = "glass")]
            {
                if let Some(st) = state.glass.style_username(
                    &chat.normalized_name,
                    message,
                    &state.settings.ui.theme,
                ) {
                    username_styles.push(st);
                }
                if let Some(st) = state.glass.style_message(&chat.normalized_name, message) {
                    message_styles.push(st);
                }
            }

            if message.highlight {
                message_styles.push(TextStyle::Highlight(
                    state.settings.ui.colours().highlight.clone().into(),
                ));
            }

            if matches!(message.r#type, MessageType::Action) {
                message_styles.push(TextStyle::Italics);
            }

            username_styles.push(TextStyle::Coloured(choose_colour(
                &message.username,
                &state.settings,
            )));

            rows.push(ChatViewRow::message(
                chat,
                message,
                Some(message_styles),
                Some(username_styles),
                &state.core,
                &state.settings,
                #[cfg(feature = "glass")]
                &state.glass,
            ));
        }

        self.cached_row_heights.resize(rows.len(), chat_row_height);
        let heights = self.cached_row_heights.clone();

        let command_helper_window_id = self.egui_id("command-helper");
        let chat_view_id = self.egui_id("chat-view");

        egui::CentralPanel::default()
            .frame(
                egui::Frame::central_panel(&ctx.style()).inner_margin(egui::Margin::symmetric(
                    CENTRAL_PANEL_INNER_MARGIN_X,
                    CENTRAL_PANEL_INNER_MARGIN_Y,
                )),
            )
            .show(ctx, |ui| {
                if self
                    .command_helper
                    .has_applicable_commands(&self.chat_input)
                {
                    egui::Window::new(command_helper_window_id)
                        .title_bar(false)
                        .resizable(false)
                        .pivot(egui::Align2::LEFT_BOTTOM)
                        .fixed_pos(ui.available_rect_before_wrap().left_bottom())
                        .show(ctx, |ui| {
                            self.command_helper.show(
                                ui,
                                chat,
                                &state.core,
                                &mut self.chat_input,
                                &self.response_widget_id,
                            );
                        });
                }

                // Chat row spacing, which is by default zero for table rows.
                ui.spacing_mut().item_spacing.y = 2.;

                ui.push_id(&chat_view_id, |ui| {
                    let view_height = ui.available_height();

                    let mut builder = TableBuilder::new(ui)
                        .stick_to_bottom(!self.user_context_menu_open) // Disable scrolling to avoid resetting context menu.
                        .max_scroll_height(chat_view_size.y)
                        .column(Column::remainder())
                        .auto_shrink([false; 2]);

                    if let Some(message_id) = self.scroll_to {
                        let is_message_past_unread_marker =
                            unread_marker_active && chat.prev_unread_pointer <= message_id;
                        let adjusted_row_id = <bool as Into<usize>>::into(add_filler_space)
                            + message_id
                            + <bool as Into<usize>>::into(is_message_past_unread_marker);

                        let should_stick_to_bottom = message_id == chat.messages.len();
                        builder = builder
                            .scroll_to_row(adjusted_row_id, Some(egui::Align::Center))
                            .stick_to_bottom(should_stick_to_bottom);

                        self.scroll_to = None;
                    }

                    // Format the chat view as a table with variable row widths (replacement for `ScrollView::show_rows()`,
                    // which only understands uniform rows and glitches pretty hard when run in a `show_rows()` + `stick_to_bottom()` combination).
                    //
                    // Row heights are saved for the next drawing cycle, when TableBuilder calculates a proper virtual table.
                    // Source of wisdom: https://github.com/emilk/egui/blob/c86bfb6e67abf208dccd7e006ccd9c3675edcc2f/crates/egui_demo_lib/src/demo/table_demo.rs

                    let heights = heights.into_iter();
                    let scroll_area_output = builder.body(|body| {
                        body.heterogeneous_rows(heights, |mut row| {
                            let chat_row_widget = &mut rows[row.index()];
                            let row_idx = row.index();

                            row.col(|ui| {
                                ui.set_max_width(chat_view_size.x); // Re-trigger text wrapping on window size change.
                                let chat_row_height = ui.add(chat_row_widget).rect.height();
                                if row_idx < self.cached_row_heights.len() {
                                    self.cached_row_heights[row_idx] = chat_row_height;
                                }
                            });
                        });
                    });

                    self.user_context_menu_open = rows.iter().any(|row| row.is_user_menu_opened());

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
