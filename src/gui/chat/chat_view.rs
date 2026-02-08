use eframe::egui;
use egui_extras::{Column, TableBuilder};
use steel_core::chat::{Chat, ChatLike};
use steel_core::settings::chat::ChatPosition;

#[cfg(feature = "puffin")]
use puffin;
use steel_core::TextStyle;

use crate::core::chat::MessageType;
use crate::gui::state::UIState;
use crate::gui::widgets::chat::message::username::choose_colour;
use crate::gui::widgets::chat::message::ChatViewRow;
use crate::gui::widgets::chat::shadow::InnerShadow;
use crate::gui::{CENTRAL_PANEL_INNER_MARGIN_X, CENTRAL_PANEL_INNER_MARGIN_Y};

use super::filter::ChatFilter;
use crate::gui::command::{self, CommandHelper};

const MAX_MESSAGE_LENGTH: usize = 450;

enum RowMetadata {
    Filler,
    UnreadMarker,
    Message { message_idx: usize },
}

pub struct ChatView {
    chat_name: String,
    chat_input: String,
    pub response_widget_id: Option<egui::Id>,
    pub scroll_to: Option<usize>,
    scroll_to_attempts: u8,
    cached_row_heights: Vec<f32>,
    command_helper: command::CommandHelper,

    filter: ChatFilter,

    user_context_menu_open: bool,
}

impl ChatView {
    pub fn new(chat_name: String) -> Self {
        Self {
            chat_name,
            chat_input: String::default(),
            response_widget_id: None,
            scroll_to: None,
            scroll_to_attempts: 0,
            cached_row_heights: Vec::default(),
            command_helper: CommandHelper::default(),
            filter: ChatFilter::new(),
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

                    // Check if filter input fields have focus to avoid intercepting Enter
                    let filter_has_focus = ctx.memory(|mem| {
                        if let Some(focused_id) = mem.focused() {
                            let username_filter_id =
                                egui::Id::new(format!("username-filter-input-{}", self.chat_name));
                            let message_filter_id =
                                egui::Id::new(format!("message-filter-input-{}", self.chat_name));
                            focused_id == username_filter_id || focused_id == message_filter_id
                        } else {
                            false
                        }
                    });

                    if response.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        && !filter_has_focus
                        && !{
                            let result = self.command_helper.detect_and_run(
                                chat,
                                &state.core,
                                &mut self.chat_input,
                            );
                            if result {
                                response.request_focus();
                            }
                            result
                        }
                    {
                        let trimmed_message = self.chat_input.trim();
                        if !trimmed_message.is_empty() {
                            state.core.chat_message_sent(
                                &chat.name,
                                chat.name.chat_type(),
                                trimmed_message,
                            );
                        }
                        self.chat_input.clear();
                        response.request_focus();
                    }
                });
            });
    }

    fn show_filter(&mut self, ctx: &egui::Context, state: &UIState, chat: &Chat) {
        let (activated_now, scroll_to) = self.filter.handle_input(ctx);

        if let Some(message_idx) = scroll_to {
            self.scroll_to = Some(message_idx);
        }

        if let Some(message_idx) = self.filter.show_ui(ctx, state, chat, activated_now) {
            self.scroll_to = Some(message_idx);
        }
    }

    pub fn enable_filter(&mut self) {
        self.filter.enable();
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &UIState) {
        #[cfg(feature = "puffin")]
        puffin::profile_function!();

        let chat = match state.find_chat(&self.chat_name) {
            Some(chat) => chat,
            None => return,
        };

        self.show_filter(ctx, state, chat);

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

        let mut row_metadata: Vec<RowMetadata> = Vec::new();

        if add_filler_space {
            row_metadata.push(RowMetadata::Filler);
        }

        let mut unread_marker_active = false;
        for idx in 0..chat.messages.len() {
            #[cfg(feature = "puffin")]
            puffin::profile_scope!("create_chat_row_metadata");
            if state.settings.chat.behaviour.track_unread_messages
                && state.active_chat_tab_name == chat.normalized_name
                && chat.prev_unread_pointer == idx
            {
                row_metadata.push(RowMetadata::UnreadMarker);
                unread_marker_active = true;
            }

            row_metadata.push(RowMetadata::Message { message_idx: idx });
        }

        self.cached_row_heights
            .resize(row_metadata.len(), chat_row_height);
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

                    let should_stick_to_bottom = !(self.user_context_menu_open
                        || self.filter.is_active()
                        || self.scroll_to.is_some());
                    let mut builder = TableBuilder::new(ui)
                        .stick_to_bottom(should_stick_to_bottom) // Disable scrolling when filter is active or context menu is open
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

                        // Keep scroll_to active for multiple frames to ensure it takes effect
                        self.scroll_to_attempts += 1;
                        if self.scroll_to_attempts >= 3 {
                            self.scroll_to = None;
                            self.scroll_to_attempts = 0;
                        }
                    }

                    // Format the chat view as a table with variable row widths (replacement for `ScrollView::show_rows()`,
                    // which only understands uniform rows and glitches pretty hard when run in a `show_rows()` + `stick_to_bottom()` combination).
                    //
                    // Row heights are saved for the next drawing cycle, when TableBuilder calculates a proper virtual table.
                    // Source of wisdom: https://github.com/emilk/egui/blob/c86bfb6e67abf208dccd7e006ccd9c3675edcc2f/crates/egui_demo_lib/src/demo/table_demo.rs

                    let heights = heights.into_iter();
                    let mut user_context_menu_open = false;
                    let scroll_area_output = builder.body(|body| {
                        #[cfg(feature = "puffin")]
                        puffin::profile_scope!("render_table_body");
                        body.heterogeneous_rows(heights, |mut row| {
                            let row_idx = row.index();

                            row.col(|ui| {
                                #[cfg(feature = "puffin")]
                                puffin::profile_scope!("render_row");
                                ui.set_max_width(chat_view_size.x); // Re-trigger text wrapping on window size change.

                                // Create ChatViewRow on-demand for visible rows only
                                let mut chat_row_widget = match &row_metadata[row_idx] {
                                    RowMetadata::Filler => ChatViewRow::filler(
                                        chat,
                                        chat_view_size.x,
                                        chat_view_size.y - chat_row_height - 4.0,
                                    ),
                                    RowMetadata::UnreadMarker => ChatViewRow::unread_marker(
                                        chat,
                                        chat_row_height,
                                        state.settings.ui.colours().highlight.clone().into(),
                                    ),
                                    RowMetadata::Message { message_idx } => {
                                        let message = &chat.messages[*message_idx];

                                        let mut username_styles: Vec<TextStyle> = Vec::new();
                                        let mut message_styles = Vec::new();

                                        // Add default username color first, so it can be overridden by glass styles
                                        username_styles.push(TextStyle::Coloured(choose_colour(
                                            &message.username,
                                            &state.own_username,
                                            &state.settings,
                                        )));

                                        #[cfg(feature = "glass")]
                                        {
                                            if let Some(st) = state.glass.style_username(
                                                &chat.normalized_name,
                                                message,
                                                &state.settings.ui.theme,
                                            ) {
                                                username_styles.push(st);
                                            }
                                            if let Some(st) = state
                                                .glass
                                                .style_message(&chat.normalized_name, message)
                                            {
                                                message_styles.push(st);
                                            }
                                        }

                                        if matches!(message.r#type, MessageType::Action) {
                                            message_styles.push(TextStyle::Italics);
                                        }

                                        let search_result_color = self.filter.get_highlight_color(
                                            *message_idx,
                                            state.settings.ui.colours(),
                                        );

                                        ChatViewRow::message(
                                            chat,
                                            message,
                                            Some(message_styles),
                                            Some(username_styles),
                                            &state.core,
                                            &state.settings,
                                            #[cfg(feature = "glass")]
                                            &state.glass,
                                            message.highlight,
                                            search_result_color,
                                        )
                                    }
                                };

                                let chat_row_height = ui.add(&mut chat_row_widget).rect.height();
                                if row_idx < self.cached_row_heights.len() {
                                    self.cached_row_heights[row_idx] = chat_row_height;
                                }

                                if chat_row_widget.is_user_menu_opened() {
                                    user_context_menu_open = true;
                                }
                            });
                        });
                    });

                    self.user_context_menu_open = user_context_menu_open;

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

    pub fn insert_user_mention(&mut self, ctx: &egui::Context, username: String) {
        if let Some(text_edit_id) = self.response_widget_id {
            if let Some(mut state) = egui::TextEdit::load_state(ctx, text_edit_id) {
                let char_pos = match state.cursor.char_range() {
                    None => 0,
                    Some(cc) => std::cmp::min(cc.primary.index, cc.secondary.index),
                };

                let mut chars: Vec<char> = self.chat_input.chars().collect();

                // Delete selected text if any
                if let Some(cc) = state.cursor.char_range() {
                    let start = std::cmp::min(cc.primary.index, cc.secondary.index);
                    let end = std::cmp::max(cc.primary.index, cc.secondary.index);
                    if start != end {
                        chars.drain(start..end);
                    }
                }

                let insertion = if chars.is_empty() {
                    format!("{username}: ")
                } else if char_pos == chars.len() {
                    if chars.last() == Some(&' ') {
                        username.clone()
                    } else {
                        format!(" {username}")
                    }
                } else {
                    username.clone()
                };

                let before: String = chars.iter().take(char_pos).collect();
                let after: String = chars.iter().skip(char_pos).collect();
                self.chat_input = format!("{before}{insertion}{after}");

                let new_char_pos = char_pos + insertion.chars().count();
                let ccursor = egui::text::CCursor::new(new_char_pos);
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                state.store(ctx, text_edit_id);
            }
        }
    }
}
