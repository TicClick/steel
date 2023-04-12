use std::collections::BTreeSet;

use eframe::egui::{self, Ui};

use crate::app::AppMessageIn;
use crate::core::chat::{ChatLike, ChatType};

use super::UIState;

#[derive(Default)]
pub struct ChatTabs {
    pub new_channel_input: String,
    pub new_chat_input: String,
}

impl ChatTabs {
    pub fn show(&mut self, ctx: &egui::Context, state: &mut UIState) {
        egui::SidePanel::left("chats").show(ctx, |ui| {
            ui.heading("public channels");
            if state.is_connected() {
                self.show_new_chat_input(state, ui, ChatType::Channel);
            }
            self.show_chats(state, ui, ChatType::Channel);

            ui.separator();

            ui.heading("private messages");
            if state.is_connected() {
                self.show_new_chat_input(state, ui, ChatType::Person);
            }
            self.show_chats(state, ui, ChatType::Person);
        });
    }
}

impl ChatTabs {
    fn show_new_chat_input(&mut self, state: &mut UIState, ui: &mut Ui, mode: ChatType) {
        let input: &mut String = match mode {
            ChatType::Channel => &mut self.new_channel_input,
            ChatType::Person => &mut self.new_chat_input,
        };
        ui.horizontal(|ui| {
            let add_chat = ui.button("+");
            let hint = match mode {
                ChatType::Channel => "channel",
                ChatType::Person => "user",
            };
            let response = ui.add_sized(
                ui.available_size(),
                egui::TextEdit::singleline(input)
                    .hint_text(hint)
                    .interactive(state.is_connected())
                    .id(egui::Id::new(mode.clone())),
            );
            if add_chat.clicked()
                || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                // Whether the channel is valid or not is determined by the server (will send us a message).
                match mode {
                    ChatType::Channel => {
                        state
                            .app_queue_handle
                            .blocking_send(AppMessageIn::UIChannelOpened(input.clone()))
                            .unwrap();
                    }
                    ChatType::Person => {
                        state
                            .app_queue_handle
                            .blocking_send(AppMessageIn::UIPrivateChatOpened(input.clone()))
                            .unwrap();
                    }
                }
                input.clear();
                response.request_focus();
            }
        });
    }

    fn show_chats(&self, state: &mut UIState, ui: &mut Ui, mode: ChatType) {
        let it = state.chats.keys().filter(|s| match mode {
            ChatType::Channel => s.is_channel(),
            ChatType::Person => !s.is_channel(),
        });

        let mut chats_to_clear = BTreeSet::new();

        for channel_name in it {
            let is_active_tab = state.is_active_tab(channel_name);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let mut label = egui::RichText::new(channel_name);
                    if is_active_tab {
                        state.highlights.mark_as_read(channel_name);
                    } else if state.highlights.tab_contains_highlight(channel_name) {
                        label = label.color(state.settings.notifications.highlights.colour.clone());
                    }

                    let chat_tab = ui.selectable_value(
                        &mut state.active_chat_tab_name,
                        channel_name.to_owned(),
                        label,
                    );
                    let mut close_tab = chat_tab.middle_clicked();
                    chat_tab.context_menu(|ui| {
                        if matches!(mode, ChatType::Channel) {
                            if state.settings.chat.autojoin.contains(channel_name) {
                                if ui.button("Remofe from favourites").clicked() {
                                    state.settings.chat.autojoin.remove(channel_name);
                                    // TODO: this should be done elsewhere, in a centralized manner, I'm just being lazy right now
                                    state
                                        .app_queue_handle
                                        .blocking_send(AppMessageIn::UISettingsUpdated(
                                            state.settings.clone(),
                                        ))
                                        .unwrap();
                                    ui.close_menu();
                                }
                            } else if ui.button("Add to favourites").clicked() {
                                state.settings.chat.autojoin.insert(channel_name.to_owned());
                                // TODO: this should be done elsewhere, in a centralized manner, I'm just being lazy right now
                                state
                                    .app_queue_handle
                                    .blocking_send(AppMessageIn::UISettingsUpdated(
                                        state.settings.clone(),
                                    ))
                                    .unwrap();
                                ui.close_menu();
                            }
                        }

                        if ui.button("Clear messages").clicked() {
                            chats_to_clear.insert(channel_name.to_owned());
                            ui.close_menu();
                        }

                        let close_title = match mode {
                            ChatType::Channel => "Leave",
                            ChatType::Person => "Close",
                        };
                        if ui.button(close_title).clicked() {
                            close_tab = true;
                            ui.close_menu();
                        }
                    });

                    if close_tab {
                        state
                            .app_queue_handle
                            .blocking_send(AppMessageIn::UIChatClosed(channel_name.to_owned()))
                            .unwrap();
                    }
                });
            });
        }

        for target in chats_to_clear {
            state.clear_chat(&target);
        }
    }
}
