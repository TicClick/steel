use std::collections::BTreeSet;

use eframe::egui::{self, Ui};

use crate::core::chat::{ChatLike, ChatState, ChatType};

use crate::gui::state::UIState;

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

            let add_chat = !input.is_empty()
                && (add_chat.clicked()
                    || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))));

            if add_chat {
                // Whether the channel is valid or not is determined by the server (will send us a message),
                // but for now let's add it to the interface.
                match mode {
                    ChatType::Channel => {
                        let channel_name = match input.is_channel() {
                            true => input.clone(),
                            false => format!("#{}", input),
                        };
                        state.core.channel_opened(&channel_name);
                        state.core.channel_join_requested(&channel_name);
                    }
                    ChatType::Person => state.core.private_chat_opened(input),
                }
                input.clear();
                response.request_focus();
            }
        });
    }

    fn show_chats(&self, state: &mut UIState, ui: &mut Ui, mode: ChatType) {
        let it: Vec<(String, String, ChatState)> = state
            .filter_chats(|ch| match mode {
                ChatType::Channel => ch.name.is_channel(),
                ChatType::Person => !ch.name.is_channel(),
            })
            .map(|ch| {
                (
                    ch.name.to_lowercase(),
                    ch.name.to_owned(),
                    ch.state.to_owned(),
                )
            })
            .collect();

        let mut chats_to_clear = BTreeSet::new();

        for (normalized_chat_name, chat_name, chat_state) in it {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let mut label = egui::RichText::new(&chat_name);
                    if state
                        .highlights
                        .tab_contains_highlight(&normalized_chat_name)
                    {
                        label = label.color(state.settings.notifications.highlights.colour.clone());
                    }

                    let chat_tab = ui.selectable_value(
                        &mut state.active_chat_tab_name,
                        normalized_chat_name.to_owned(),
                        label,
                    );
                    if chat_tab.clicked() {
                        state.highlights.mark_as_read(&normalized_chat_name);
                    }
                    if matches!(chat_state, ChatState::JoinInProgress) {
                        ui.spinner();
                    }
                    if chat_tab.middle_clicked() {
                        state.core.chat_tab_closed(&normalized_chat_name);
                    }

                    chat_tab.context_menu(|ui| {
                        if matches!(mode, ChatType::Channel) {
                            if state.settings.chat.autojoin.contains(&normalized_chat_name) {
                                if ui.button("Remove from favourites").clicked() {
                                    state.settings.chat.autojoin.remove(&normalized_chat_name);
                                    // TODO: this should be done elsewhere, in a centralized manner, I'm just being lazy right now
                                    state.core.settings_updated(&state.settings);
                                    ui.close_menu();
                                }
                            } else if ui.button("Add to favourites").clicked() {
                                state
                                    .settings
                                    .chat
                                    .autojoin
                                    .insert(normalized_chat_name.to_owned());
                                // TODO: this should be done elsewhere, in a centralized manner, I'm just being lazy right now
                                state.core.settings_updated(&state.settings);
                                ui.close_menu();
                            }
                        }

                        if ui.button("Clear messages").clicked() {
                            chats_to_clear.insert(normalized_chat_name.to_owned());
                            ui.close_menu();
                        }

                        let close_title = match mode {
                            ChatType::Channel => "Leave",
                            ChatType::Person => "Close",
                        };
                        if ui.button(close_title).clicked() {
                            state.core.chat_tab_closed(&normalized_chat_name);
                            ui.close_menu();
                        }
                    });
                });
            });
        }

        for target in chats_to_clear {
            state.clear_chat(&target);
        }
    }
}
