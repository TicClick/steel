use eframe::egui;

use crate::app::AppMessageIn;
use crate::core::chat::{Chat, Message, MessageChunk, MessageType};

use super::UIState;

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
        egui::TopBottomPanel::bottom("input").show(ctx, |ui| {
            if !state.is_connected() {
                ui.centered_and_justified(|ui| ui.label("(chat not available in offline mode)"));
                return;
            }

            let text_field =
                egui::TextEdit::singleline(&mut self.chat_input).hint_text("new message");

            // Don't indent the widget. Hacky, but we don't have access to ui.placer, which controls the layout.
            let mut pos = ui.available_rect_before_wrap();
            pos.set_left(pos.left() - ui.spacing().item_spacing.x);
            pos.set_right(pos.right() + ui.spacing().item_spacing.x);

            let response = ui.put(pos, text_field);
            self.response_widget_id = Some(response.id);

            if let Some(ch) = state.active_chat() {
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    state
                        .app_queue_handle
                        .blocking_send(AppMessageIn::UIChatMessageSent {
                            target: ch.name.clone(),
                            text: self.chat_input.clone(),
                        })
                        .unwrap();
                    self.chat_input.clear();
                    response.request_focus();
                }
            }
        });

        // TODO: use show_rows() instead
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if let Some(ch) = state.active_chat() {
                        for i in 0..ch.messages.len() {
                            self.display_chat_message(ui, state, ch, i);
                        }
                    }
                });
        });
    }

    pub fn return_focus(&mut self, ctx: &egui::Context) {
        ctx.memory_mut(|mem| {
            if mem.focus().is_none() {
                if let Some(id) = self.response_widget_id {
                    mem.request_focus(id);
                }
            }
        });
    }

    fn display_chat_message(
        &self,
        ui: &mut egui::Ui,
        state: &UIState,
        chat: &Chat,
        message_id: usize,
    ) {
        let msg = &chat.messages[message_id];
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x /= 2.;
            ui.label(msg.formatted_time());

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
                .context_menu(|ui| self.show_username_menu(ui, state, chat, msg));

            let is_highlight = state
                .highlights
                .message_contains_highlight(chat, message_id);
            let is_action = matches!(msg.r#type, MessageType::Action);

            let layout = egui::Layout::from_main_dir_and_cross_align(
                egui::Direction::LeftToRight,
                egui::Align::Center,
            )
            .with_main_wrap(true)
            .with_cross_justify(false);

            ui.with_layout(layout, |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for c in state.get_chunks(&chat.name, message_id) {
                    match &c {
                        MessageChunk::Text(s) | MessageChunk::Link { title: s, .. } => {
                            let mut text_chunk = egui::RichText::new(s);
                            if is_highlight {
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
                                            o.copied_text = loc;
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
            });
        });
    }

    fn show_username_menu(
        &self,
        ui: &mut egui::Ui,
        state: &UIState,
        _chat: &Chat,
        message: &Message,
    ) {
        if state.is_connected() && ui.button("💬 Open chat").clicked() {
            state
                .app_queue_handle
                .blocking_send(AppMessageIn::UIPrivateChatOpened(message.username.clone()))
                .unwrap();
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

        ui.menu_button("📄 Copy", |ui| {
            if ui.button("Message").clicked() {
                ui.ctx().output_mut(|o| {
                    o.copied_text = message.to_string();
                });
                ui.close_menu();
            }

            if ui.button("Username").clicked() {
                ui.ctx().output_mut(|o| {
                    o.copied_text = message.username.clone();
                });
                ui.close_menu();
            }
        });
    }
}
