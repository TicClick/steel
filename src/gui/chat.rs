use eframe::egui;
use egui_extras::{Column, TableBuilder};

use crate::core::chat::{Chat, ChatLike, Message, MessageChunk, MessageType};
use crate::gui::state::UIState;

#[derive(Default)]
pub struct ChatWindow {
    chat_input: String,
    pub response_widget_id: Option<egui::Id>,
    pub scroll_to: Option<usize>,

    chat_row_height: Option<f32>,
    cached_row_heights: Vec<f32>,

    // FIXME: This is a hack to prevent the context menu from re-sticking to other chat buttons (and therefore messages)
    // when the chat keeps scrolling to bottom. The menu seems to not care about that and stick to whichever is beneath, which is changing.
    context_menu_target: Option<Message>,
}

impl ChatWindow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &UIState) {
        let interactive = state.is_connected() && state.active_chat().is_some();
        if interactive {
            egui::TopBottomPanel::bottom("input").show(ctx, |ui| {
                // Special tabs (server messages and highlights) are 1) fake and 2) read-only
                let text_field = egui::TextEdit::singleline(&mut self.chat_input)
                    .id_source("chat-input")
                    .hint_text("new message");
                let response = ui.centered_and_justified(|ui| ui.add(text_field)).inner;
                self.response_widget_id = Some(response.id);

                if let Some(ch) = state.active_chat() {
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        state.core.chat_message_sent(&ch.name, &self.chat_input);
                        self.chat_input.clear();
                        response.request_focus();
                    }
                }
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

            let chat_row_height = *self
                .chat_row_height
                .get_or_insert_with(|| ui.text_style_height(&egui::TextStyle::Body));
            self.cached_row_heights
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

                let heights = self.cached_row_heights.clone().into_iter();
                builder
                    .max_scroll_height(view_height)
                    .column(Column::remainder())
                    .auto_shrink([false; 2])
                    .body(|body| {
                        if let Some(ch) = state.active_chat() {
                            body.heterogeneous_rows(heights, |row_index, mut row| {
                                row.col(|ui| {
                                    self.show_regular_chat_single_message(ui, state, ch, row_index)
                                });
                            });
                        } else {
                            match state.active_chat_tab_name.as_str() {
                                super::SERVER_TAB_NAME => {
                                    body.heterogeneous_rows(heights, |row_index, mut row| {
                                        row.col(|ui| {
                                            self.show_server_tab_single_message(
                                                ui, state, row_index,
                                            )
                                        });
                                    });
                                }
                                super::HIGHLIGHTS_TAB_NAME => {
                                    body.heterogeneous_rows(heights, |row_index, mut row| {
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
        let updated_height = ui
            .horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x /= 2.;
                ui.style_mut().wrap = Some(true);
                show_datetime(ui, msg);
                match msg.r#type {
                    MessageType::Action | MessageType::Text => {
                        format_username(ui, state, &ch.name, msg, &mut self.context_menu_target);
                        format_chat_message_text(ui, state, msg, msg.highlight, false)
                    }
                    MessageType::System => format_system_message(ui, msg),
                }
            })
            .inner;
        self.cached_row_heights[message_index] = updated_height;
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
                show_datetime(ui, msg);
                format_chat_name(ui, state, chat_name, msg);
                format_username(ui, state, chat_name, msg, &mut self.context_menu_target);
                format_chat_message_text(ui, state, msg, false, false)
            })
            .inner;
        self.cached_row_heights[message_index] = updated_height;
    }

    fn show_server_tab_single_message(
        &mut self,
        ui: &mut egui::Ui,
        state: &UIState,
        message_index: usize,
    ) {
        let msg = &state.server_messages[message_index];
        let updated_height = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x /= 2.;
                show_datetime(ui, msg);
                format_chat_message_text(ui, state, msg, false, true)
            })
            .inner;
        self.cached_row_heights[message_index] = updated_height;
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
}

fn show_datetime(ui: &mut egui::Ui, msg: &Message) -> egui::Response {
    ui.label(msg.formatted_time()).on_hover_ui_at_pointer(|ui| {
        ui.vertical(|ui| {
            ui.label(format!("{} (local time zone)", msg.formatted_date_local()));
            ui.label(format!("{} (UTC)", msg.formatted_date_utc()));
        });
    })
}

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

    if state.plugin_manager.has_plugins() {
        state
            .plugin_manager
            .show_user_context_menu(ui, &state.core, chat_name, message);
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

fn format_username(
    ui: &mut egui::Ui,
    state: &UIState,
    chat_name: &str,
    msg: &Message,
    context_menu_target: &mut Option<Message>,
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
    };

    let resp = ui.button(username_text);
    if resp.hovered() {
        *context_menu_target = Some(msg.clone());
    }
    resp.context_menu(|ui| {
        show_username_menu(
            ui,
            state,
            chat_name,
            context_menu_target.as_ref().unwrap_or(msg),
        )
    });
}

fn format_chat_message_text(
    ui: &mut egui::Ui,
    state: &UIState,
    msg: &Message,
    mark_as_highlight: bool,
    monospace: bool,
) -> f32 {
    let is_action = matches!(msg.r#type, MessageType::Action);

    let layout = egui::Layout::from_main_dir_and_cross_align(
        egui::Direction::LeftToRight,
        egui::Align::Center,
    )
    .with_main_wrap(true)
    .with_cross_justify(false);

    let highlight_colour = state.settings.ui.colours().highlight.clone();

    let resp = ui.with_layout(layout, |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        if let Some(chunks) = &msg.chunks {
            for c in chunks {
                match &c {
                    MessageChunk::Text(s) | MessageChunk::Link { title: s, .. } => {
                        let mut text_chunk = egui::RichText::new(s);
                        if mark_as_highlight {
                            text_chunk = text_chunk.color(highlight_colour.clone());
                        }
                        if is_action {
                            text_chunk = text_chunk.italics();
                        } else if monospace {
                            text_chunk = text_chunk.monospace();
                        }

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
