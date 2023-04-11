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
            let response = ui.add_sized(
                ui.available_size(),
                egui::TextEdit::singleline(input)
                    .hint_text("<Enter> add")
                    .interactive(state.is_connected())
                    .id(egui::Id::new(mode.clone())),
            );
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
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

                    ui.selectable_value(
                        &mut state.active_chat_tab_name,
                        channel_name.to_owned(),
                        label,
                    );
                    if ui.button("x").clicked() {
                        state
                            .app_queue_handle
                            .blocking_send(AppMessageIn::UIChatClosed(channel_name.to_owned()))
                            .unwrap();
                    }
                });
            });
        }
    }
}
