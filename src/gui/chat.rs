use eframe::egui;

use crate::core::chat::{Chat, ChatLike, Message, MessageChunk, MessageType};

use crate::gui::state::UIState;

#[derive(Default)]
pub struct ChatWindow {
    chat_input: String,
    pub response_widget_id: Option<egui::Id>,
}

impl ChatWindow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &UIState) {
        // Special tabs (server messages and highlights) are 1) fake and 2) read-only
        if state.active_chat().is_some() {
            egui::TopBottomPanel::bottom("input").show(ctx, |ui| {
                let text_field = egui::TextEdit::singleline(&mut self.chat_input)
                    .hint_text("new message")
                    .frame(false)
                    .interactive(state.is_connected());
                let response = ui
                    .centered_and_justified(|ui| {
                        let response = ui.add(text_field);
                        if !state.is_connected() {
                            response.on_hover_text_at_pointer("you are offline")
                        } else {
                            response
                        }
                    })
                    .inner;
                self.response_widget_id = Some(response.id);

                if let Some(ch) = state.active_chat() {
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        state.core.chat_message_sent(&ch.name, &self.chat_input);
                        self.chat_input.clear();
                        response.request_focus();
                    }
                }
            });
        }

        // TODO: use show_rows() instead
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if let Some(ch) = state.active_chat() {
                        self.show_chat_messages(ui, state, ch);
                    } else {
                        match state.active_chat_tab_name.as_str() {
                            super::SERVER_TAB_NAME => self.show_server_messages(ui, state),
                            super::HIGHLIGHTS_TAB_NAME => self.show_highlights(ui, state),
                            _ => (),
                        }
                    }
                });
        });
    }

    fn show_highlights(&self, ui: &mut egui::Ui, state: &UIState) {
        for (chat_name, msg) in state.highlights.ordered() {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x /= 2.;
                show_datetime(ui, msg);
                format_chat_name(ui, state, chat_name, msg.id);
                format_username(ui, state, msg);
                format_chat_message_text(ui, state, msg, false);
            });
        }
    }

    fn show_server_messages(&self, ui: &mut egui::Ui, state: &UIState) {
        for msg in state.server_messages.iter() {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x /= 2.;
                show_datetime(ui, msg);
                ui.label(egui::RichText::new(&msg.text).monospace())
                    .context_menu(|ui| {
                        if ui.button("Copy message").clicked() {
                            ui.ctx().output_mut(|o| {
                                o.copied_text = msg.text.to_owned();
                            });
                            ui.close_menu();
                        }
                    });
            });
        }
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

    fn show_chat_messages(&self, ui: &mut egui::Ui, state: &UIState, chat: &Chat) {
        for msg in chat.messages.iter() {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x /= 2.;
                show_datetime(ui, msg);
                match msg.r#type {
                    MessageType::Action | MessageType::Text => format_chat_message(ui, state, msg),
                    MessageType::System => format_system_message(ui, msg),
                }
            });
        }
    }
}

fn show_datetime(ui: &mut egui::Ui, msg: &Message) {
    ui.label(msg.formatted_time()).on_hover_ui_at_pointer(|ui| {
        ui.vertical(|ui| {
            ui.label(format!("{} (local time zone)", msg.formatted_date_local()));
            ui.label(format!("{} (UTC)", msg.formatted_date_utc()));
        });
    });
}

fn show_username_menu(ui: &mut egui::Ui, state: &UIState, message: &Message) {
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
}

fn format_system_message(ui: &mut egui::Ui, msg: &Message) {
    ui.add_enabled(false, egui::Button::new(&msg.text));
}

fn format_chat_message(ui: &mut egui::Ui, state: &UIState, msg: &Message) {
    format_username(ui, state, msg);
    format_chat_message_text(ui, state, msg, msg.highlight);
}

fn format_chat_name(
    ui: &mut egui::Ui,
    state: &UIState,
    chat_name: &String,
    message_id: Option<usize>,
) {
    let chat_button = ui.button(match chat_name.is_channel() {
        true => chat_name,
        false => "(PM)",
    });

    if let Some(id) = message_id {
        let mut switch_requested = chat_button.clicked();
        chat_button.context_menu(|ui| {
            if ui.button("Go to message").clicked() {
                switch_requested = true;
                ui.close_menu();
            }
        });
        if switch_requested {
            state.core.chat_switch_requested(chat_name, id);
        }
    }
}

fn format_username(ui: &mut egui::Ui, state: &UIState, msg: &Message) {
    let username_text = if msg.username == state.settings.chat.irc.username {
        egui::RichText::new(&msg.username).color(state.settings.ui.colours.own.clone())
    } else {
        let mut label = egui::RichText::new(&msg.username);
        if let Some(c) = state
            .settings
            .ui
            .colours
            .users
            .get(&msg.username.to_lowercase())
        {
            label = label.color(c.clone())
        }
        label
    };

    ui.button(username_text)
        .context_menu(|ui| show_username_menu(ui, state, msg));
}

fn format_chat_message_text(
    ui: &mut egui::Ui,
    state: &UIState,
    msg: &Message,
    mark_as_highlight: bool,
) {
    let is_action = matches!(msg.r#type, MessageType::Action);

    let layout = egui::Layout::from_main_dir_and_cross_align(
        egui::Direction::LeftToRight,
        egui::Align::Center,
    )
    .with_main_wrap(true)
    .with_cross_justify(false);

    ui.with_layout(layout, |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        if let Some(chunks) = &msg.chunks {
            for c in chunks {
                match &c {
                    MessageChunk::Text(s) | MessageChunk::Link { title: s, .. } => {
                        let mut text_chunk = egui::RichText::new(s);
                        if mark_as_highlight {
                            text_chunk = text_chunk
                                .color(state.settings.notifications.highlights.colour.clone());
                        }
                        if is_action {
                            text_chunk = text_chunk.italics();
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
}
