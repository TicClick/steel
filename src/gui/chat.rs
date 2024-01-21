use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::collections::{BTreeMap, BTreeSet};

use steel_core::TextStyle;

use crate::core::chat::{Chat, ChatLike, Message, MessageChunk, MessageType};
use crate::gui::state::UIState;
use crate::gui::DecoratedText;

use crate::gui::command;

const MAX_MESSAGE_LENGTH: usize = 450;

trait WithInnerShadow {
    fn inner_shadow_bottom(&self, pixels: usize);
}

// (Almost) as seen at https://gist.github.com/juancampa/d8dcf7cdab813062f082eac7415abcfc
impl WithInnerShadow for egui::Ui {
    fn inner_shadow_bottom(&self, pixels: usize) {
        let mut shadow_rect = self.available_rect_before_wrap();

        let central_frame_margin = 8.; // egui::Frame::central_panel().inner_margin
        shadow_rect.set_left(shadow_rect.left() - central_frame_margin);
        shadow_rect.set_width(
            shadow_rect.width() + self.spacing().scroll.bar_inner_margin + central_frame_margin,
        );
        shadow_rect.set_bottom(shadow_rect.bottom() + self.spacing().item_spacing.y);

        let colour_ctor = match self.visuals().dark_mode {
            true => |a: u8| egui::Color32::from_rgba_unmultiplied(120, 120, 120, a),
            false => egui::Color32::from_black_alpha,
        };

        let painter = self.painter();
        let mut avail_rect = shadow_rect.translate((0.0, shadow_rect.height() - 1.0).into());
        avail_rect.set_height(1.0);
        for i in 0..pixels {
            let alpha = 1.0 - (i as f32 / pixels as f32);
            let shift = -avail_rect.height() * i as f32;
            let rect = avail_rect.translate((0.0, shift).into());
            painter.rect_filled(rect, 0.0, colour_ctor((alpha * alpha * 80.0).floor() as u8));
        }
    }
}

#[derive(Default)]
pub struct ChatWindow {
    chat_input: String,
    pub response_widget_id: Option<egui::Id>,
    pub scroll_to: Option<usize>,

    chat_row_height: Option<f32>,
    cached_row_heights: BTreeMap<String, Vec<f32>>,

    // FIXME: This is a hack to prevent the context menu from re-sticking to other chat buttons (and therefore messages)
    // when the chat keeps scrolling to bottom. The menu seems to not care about that and stick to whichever is beneath, which is changing.
    context_menu_target: Option<Message>,

    // Draw the hinting shadow at the bottom of the chat in the next frame.
    shadow_next_frame: bool,

    // Chat space width -- longer lines will wrap around the window.
    widget_width: f32,

    command_helper: command::CommandHelper,
}

impl ChatWindow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &UIState) {
        let interactive = state.is_connected() && state.active_chat().is_some();
        if interactive {
            egui::TopBottomPanel::bottom("input").show(ctx, |ui| {
                self.command_helper
                    .maybe_show(ui, state, &mut self.chat_input);

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
                            "messages longer than {} characters are truncated",
                            MAX_MESSAGE_LENGTH
                        ));
                    }
                    self.response_widget_id = Some(response.id);
                    ui.add_space(2.);

                    if let Some(ch) = state.active_chat() {
                        if response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            && !self
                                .command_helper
                                .detect_and_run(state, &mut self.chat_input)
                        {
                            state.core.chat_message_sent(&ch.name, &self.chat_input);
                            self.chat_input.clear();
                            response.request_focus();
                        }
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

        egui::CentralPanel::default().show(ctx, |ui| {
            // Default spacing, which is by default zero for table rows.
            ui.spacing_mut().item_spacing.y = 4.;
            self.widget_width = ui.available_width();

            let chat_row_height = *self
                .chat_row_height
                .get_or_insert_with(|| ui.text_style_height(&egui::TextStyle::Body));

            self.cached_row_heights
                .entry(state.active_chat_tab_name.clone())
                .or_default()
                .resize(state.chat_message_count(), chat_row_height);

            ui.push_id(&state.active_chat_tab_name, |ui| {
                let view_height = ui.available_height();
                let mut builder = TableBuilder::new(ui);
                if let Some(message_id) = self.scroll_to {
                    builder = builder.scroll_to_row(message_id, Some(egui::Align::Center));
                    self.scroll_to = None;
                } else {
                    builder = builder.stick_to_bottom(true);
                }

                let heights = self.cached_row_heights[&state.active_chat_tab_name]
                    .clone()
                    .into_iter();
                let mut last_visible_row = 0;

                builder
                    .max_scroll_height(view_height)
                    .column(Column::remainder())
                    .auto_shrink([false; 2])
                    .body(|body| {
                        if let Some(ch) = state.active_chat() {
                            body.heterogeneous_rows(heights, |mut row| {
                                let row_index = row.index();
                                last_visible_row = row_index;
                                row.col(|ui| {
                                    self.show_regular_chat_single_message(ui, state, ch, row_index);
                                });
                            });
                        } else {
                            match state.active_chat_tab_name.as_str() {
                                super::SERVER_TAB_NAME => {
                                    let server_tab_styles = Some({
                                        let mut st = BTreeSet::new();
                                        st.insert(TextStyle::Monospace);
                                        st
                                    });
                                    body.heterogeneous_rows(heights, |mut row| {
                                        let row_index = row.index();
                                        last_visible_row = row_index;
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
                                super::HIGHLIGHTS_TAB_NAME => {
                                    body.heterogeneous_rows(heights, |mut row| {
                                        let row_index = row.index();
                                        last_visible_row = row_index;
                                        row.col(|ui| {
                                            self.show_highlights_tab_single_message(
                                                ui, state, row_index,
                                            )
                                        });
                                    });
                                }
                                _ => (),
                            }
                        }
                    });

                // FIXME: the shadow is removed as soon as the last row becomes PARTIALLY, NOT FULLY visible.
                if last_visible_row + 1
                    < self
                        .cached_row_heights
                        .get_mut(&state.active_chat_tab_name)
                        .unwrap()
                        .len()
                {
                    if self.shadow_next_frame {
                        ui.inner_shadow_bottom(20);
                    } else {
                        self.shadow_next_frame = true;
                    }
                } else {
                    self.shadow_next_frame = false;
                }
            });
        });
    }

    fn show_regular_chat_single_message(
        &mut self,
        ui: &mut egui::Ui,
        state: &UIState,
        ch: &Chat,
        message_index: usize,
    ) {
        let msg = &ch.messages[message_index];

        #[allow(unused_mut)] // glass
        let mut username_styles = BTreeSet::<TextStyle>::new();
        let mut message_styles = BTreeSet::<TextStyle>::new();

        #[cfg(feature = "glass")]
        {
            if let Some(st) = state.glass.style_username(&ch.name, msg) {
                username_styles.insert(st);
            }
            if let Some(st) = state.glass.style_message(&ch.name, msg) {
                message_styles.insert(st);
            }
        }

        if msg.highlight {
            message_styles.insert(TextStyle::Highlight);
        }
        if matches!(msg.r#type, MessageType::Action) {
            message_styles.insert(TextStyle::Italics);
        }

        let updated_height = ui
            .push_id(format!("{}_row_{}", ch.name, message_index), |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x /= 2.;
                    ui.set_max_width(self.widget_width);
                    show_datetime(ui, state, msg, &None);
                    match msg.r#type {
                        MessageType::Action | MessageType::Text => {
                            self.format_username(ui, state, &ch.name, msg, &Some(username_styles));
                            format_chat_message_text(ui, state, msg, &Some(message_styles))
                        }
                        MessageType::System => format_system_message(ui, msg),
                    }
                })
            })
            .inner
            .inner;
        self.cached_row_heights
            .get_mut(&state.active_chat_tab_name)
            .unwrap()[message_index] = updated_height;
    }

    fn show_highlights_tab_single_message(
        &mut self,
        ui: &mut egui::Ui,
        state: &UIState,
        message_index: usize,
    ) {
        let (chat_name, msg) = &state.highlights.ordered()[message_index];
        let updated_height = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x /= 2.;
                show_datetime(ui, state, msg, &None);
                format_chat_name(ui, state, chat_name, msg);
                self.format_username(ui, state, chat_name, msg, &None);
                format_chat_message_text(ui, state, msg, &None)
            })
            .inner;
        self.cached_row_heights
            .get_mut(&state.active_chat_tab_name)
            .unwrap()[message_index] = updated_height;
    }

    fn show_server_tab_single_message(
        &mut self,
        ui: &mut egui::Ui,
        state: &UIState,
        message_index: usize,
        styles: &Option<BTreeSet<TextStyle>>,
    ) {
        let msg = &state.server_messages[message_index];
        let updated_height = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x /= 2.;
                show_datetime(ui, state, msg, styles);
                format_chat_message_text(ui, state, msg, styles)
            })
            .inner;
        self.cached_row_heights
            .get_mut(&state.active_chat_tab_name)
            .unwrap()[message_index] = updated_height;
    }

    pub fn return_focus(&mut self, ctx: &egui::Context, state: &UIState) {
        if state.is_connected() {
            ctx.memory_mut(|mem| {
                if mem.focus().is_none() {
                    if let Some(id) = self.response_widget_id {
                        mem.request_focus(id);
                    }
                }
            });
        }
    }

    fn format_username(
        &mut self,
        ui: &mut egui::Ui,
        state: &UIState,
        chat_name: &str,
        msg: &Message,
        styles: &Option<BTreeSet<TextStyle>>,
    ) {
        let username_text = if msg.username == state.settings.chat.irc.username {
            egui::RichText::new(&msg.username).color(state.settings.ui.colours().own.clone())
        } else {
            let colour = state
                .settings
                .ui
                .colours()
                .username_colour(&msg.username.to_lowercase());
            egui::RichText::new(&msg.username).color(colour.clone())
        }
        .with_styles(styles, &state.settings);

        #[allow(unused_mut)] // glass
        let mut resp = ui.button(username_text);

        #[cfg(feature = "glass")]
        if let Some(tt) = state.glass.show_user_tooltip(chat_name, msg) {
            resp = resp.on_hover_text_at_pointer(tt);
        }

        if resp.is_pointer_button_down_on() {
            self.context_menu_target = Some(msg.clone());
        }

        if resp.clicked() {
            self.handle_username_click(ui, msg);
        }

        resp.context_menu(|ui| {
            show_username_menu(
                ui,
                state,
                chat_name,
                self.context_menu_target.as_ref().unwrap_or(msg),
            )
        });
    }

    fn handle_username_click(&mut self, ui: &mut egui::Ui, msg: &Message) {
        if let Some(text_edit_id) = self.response_widget_id {
            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), text_edit_id) {
                let pos = match state.ccursor_range() {
                    None => 0,
                    Some(cc) => std::cmp::min(cc.primary.index, cc.secondary.index),
                };

                if let Some(cc) = state.ccursor_range() {
                    let start = std::cmp::min(cc.primary.index, cc.secondary.index);
                    let end = std::cmp::max(cc.primary.index, cc.secondary.index);
                    if start != end {
                        self.chat_input.replace_range(start..end, "");
                    }
                }

                let insertion = if self.chat_input.is_empty() {
                    format!("{}: ", msg.username)
                } else if pos == self.chat_input.chars().count() {
                    if self.chat_input.ends_with(' ') {
                        msg.username.to_owned()
                    } else {
                        format!(" {}", msg.username)
                    }
                } else {
                    msg.username.to_owned()
                };
                self.chat_input.insert_str(pos, &insertion);
                let ccursor = egui::text::CCursor::new(pos + insertion.len());
                state.set_ccursor_range(Some(egui::text::CCursorRange::one(ccursor)));
                state.store(ui.ctx(), text_edit_id);
            }
        }
    }
}

fn show_datetime(
    ui: &mut egui::Ui,
    state: &UIState,
    msg: &Message,
    styles: &Option<BTreeSet<TextStyle>>,
) -> egui::Response {
    let timestamp = egui::RichText::new(msg.formatted_time()).with_styles(styles, &state.settings);
    ui.label(timestamp).on_hover_ui_at_pointer(|ui| {
        ui.vertical(|ui| {
            ui.label(format!("{} (local time zone)", msg.formatted_date_local()));
            ui.label(format!("{} (UTC)", msg.formatted_date_utc()));
        });
    })
}

#[allow(unused_variables)] // glass
fn show_username_menu(ui: &mut egui::Ui, state: &UIState, chat_name: &str, message: &Message) {
    if state.is_connected() && ui.button("💬 Open chat").clicked() {
        state.core.private_chat_opened(&message.username);
        ui.close_menu();
    }

    // TODO: the link should contain ID instead
    if ui.button("🔎 View profile").clicked() {
        ui.ctx().output_mut(|o| {
            o.open_url = Some(egui::output::OpenUrl {
                url: format!("https://osu.ppy.sh/users/{}", message.username),
                new_tab: true,
            });
        });
        ui.close_menu();
    }

    if ui.button("🌐 Translate message").clicked() {
        ui.ctx().output_mut(|o| {
            o.open_url = Some(egui::output::OpenUrl {
                url: format!(
                    "https://translate.google.com/?sl=auto&tl=en&text={}&op=translate",
                    percent_encoding::utf8_percent_encode(
                        &message.text,
                        percent_encoding::NON_ALPHANUMERIC
                    )
                ),
                new_tab: true,
            });
        });
        ui.close_menu();
    }

    ui.separator();

    if ui.button("Copy message").clicked() {
        ui.ctx().output_mut(|o| {
            o.copied_text = message.to_string();
        });
        ui.close_menu();
    }

    if ui.button("Copy username").clicked() {
        ui.ctx().output_mut(|o| {
            o.copied_text = message.username.clone();
        });
        ui.close_menu();
    }

    #[cfg(feature = "glass")]
    state
        .glass
        .show_user_context_menu(ui, &state.core, chat_name, message);
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
                ui.close_menu();
            }
        });
        if switch_requested {
            state
                .core
                .chat_switch_requested(chat_name, message.id.unwrap());
        }
    }
}

fn format_chat_message_text(
    ui: &mut egui::Ui,
    state: &UIState,
    msg: &Message,
    styles: &Option<BTreeSet<TextStyle>>,
) -> f32 {
    let layout = egui::Layout::from_main_dir_and_cross_align(
        egui::Direction::LeftToRight,
        egui::Align::Center,
    )
    .with_main_wrap(true)
    .with_cross_justify(false);

    let resp = ui.with_layout(layout, |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        if let Some(chunks) = &msg.chunks {
            for c in chunks {
                match &c {
                    MessageChunk::Text(s) | MessageChunk::Link { title: s, .. } => {
                        let text_chunk =
                            egui::RichText::new(s).with_styles(styles, &state.settings);
                        if let MessageChunk::Link { location: loc, .. } = c {
                            ui.hyperlink_to(text_chunk, loc.clone()).context_menu(|ui| {
                                if ui.button("Copy URL").clicked() {
                                    ui.ctx().output_mut(|o| {
                                        o.copied_text = loc.to_owned();
                                    });
                                    ui.close_menu();
                                }
                            });
                        } else {
                            ui.label(text_chunk);
                        }
                    }
                }
            }
        }
    });
    resp.response.rect.height()
}
