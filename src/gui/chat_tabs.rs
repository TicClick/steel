use eframe::egui::{self, Frame, Id, Margin, Ui};
use egui_dnd::DragDropItem;
use steel_core::chat::TabState;
use steel_core::settings::{Colour, Settings};

use crate::core::chat::{ChatLike, ChatState, ChatType};

use crate::gui::state::UIState;

use super::context_menu::chat::{
    menu_item_add_to_favourites, menu_item_clear_chat_tab, menu_item_close_chat,
    menu_item_remove_from_favourites,
};
use super::context_menu::chat_user::menu_item_open_chat_user_profile;
use super::context_menu::shared::menu_item_open_chat_log;

const MIN_CHAT_TABS_SCROLLVIEW_HEIGHT: f32 = 180.;

// Stabilize indices of the elements in the drag-and-drop zone with channels.
// Courtesy of lucasmerlin @ https://github.com/lucasmerlin/hello_egui/blob/main/crates/egui_dnd/examples/index_as_id.rs

struct ChatTabData {
    original_index: usize,
    normalized_name: String,
    name: String,
    chat_state: ChatState,
    tab_state: TabState,
}

struct EnumeratedItem<T> {
    item: T,
    index: usize,
}

impl<T> DragDropItem for EnumeratedItem<T> {
    fn id(&self) -> Id {
        Id::new(self.index)
    }
}

#[derive(Default)]
pub struct ChatTabs {
    pub new_channel_input: String,
    pub new_chat_input: String,
}

impl ChatTabs {
    pub fn show(&mut self, ctx: &egui::Context, state: &mut UIState) {
        let frame_maker = || Frame::new().inner_margin(Margin::symmetric(2, 2));

        egui::SidePanel::left("chats").show(ctx, |ui| {
            egui::TopBottomPanel::top("public-channels-panel")
                .resizable(true)
                .show_separator_line(false)
                .frame(frame_maker())
                .show_inside(ui, |ui| {
                    ui.heading("public channels");
                    if state.is_connected() {
                        self.show_new_chat_input(state, ui, ChatType::Channel);
                    }
                    self.show_chats(state, ui, ChatType::Channel);
                });

            egui::TopBottomPanel::top("private-chats-panel")
                .resizable(true)
                .show_separator_line(false)
                .frame(frame_maker())
                .show_inside(ui, |ui| {
                    ui.heading("private messages");
                    if state.is_connected() {
                        self.show_new_chat_input(state, ui, ChatType::Person);
                    }
                    self.show_chats(state, ui, ChatType::Person);
                });

            egui::TopBottomPanel::top("system-chats-panel")
                .resizable(false)
                .show_separator_line(false)
                .frame(frame_maker())
                .show_inside(ui, |ui| {
                    ui.heading("system");
                    self.show_chats(state, ui, ChatType::System);
                });
        });
    }
}

fn pick_tab_colour(settings: &Settings, tab_state: &TabState) -> Colour {
    let colour = match tab_state {
        TabState::Read => &settings.ui.colours().read_tabs,
        TabState::Unread => &settings.ui.colours().unread_tabs,
        TabState::Highlight => &settings.ui.colours().highlight,
    };
    colour.clone()
}

fn tab_context_menu(ui: &mut Ui, state: &mut UIState, normalized_chat_name: &str, mode: &ChatType) {
    let is_favourite_chat = state
        .settings
        .chat
        .autojoin
        .iter()
        .any(|s| s == normalized_chat_name);

    match is_favourite_chat {
        true => menu_item_remove_from_favourites(ui, state, false, normalized_chat_name),
        false => menu_item_add_to_favourites(ui, state, false, normalized_chat_name),
    }

    if !normalized_chat_name.is_channel() {
        menu_item_open_chat_user_profile(ui, false, normalized_chat_name);
    }
    menu_item_open_chat_log(ui, &state.core, false, normalized_chat_name);

    ui.separator();

    menu_item_clear_chat_tab(ui, state, false, normalized_chat_name);

    ui.separator();

    menu_item_close_chat(ui, state, false, normalized_chat_name, mode);
}

impl ChatTabs {
    fn show_new_chat_input(&mut self, state: &mut UIState, ui: &mut Ui, mode: ChatType) {
        let input: &mut String = match mode {
            ChatType::Channel => &mut self.new_channel_input,
            ChatType::Person => &mut self.new_chat_input,
            ChatType::System => unreachable!(),
        };

        let validation_result = match mode {
            ChatType::Channel => super::validate_channel_name(input),
            ChatType::Person => super::validate_username(input),
            ChatType::System => unreachable!(),
        };

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let add_chat = ui.button("+").on_hover_text_at_pointer(
                    "<Enter> = add\n\
                    Middle click = close",
                );
                let hint = match mode {
                    ChatType::Channel => "channel",
                    ChatType::Person => "user",
                    ChatType::System => unreachable!(),
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
                        || (response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))));

                if add_chat && validation_result.is_ok() {
                    // Whether the channel is valid or not is determined by the server (will send us a message),
                    // but for now let's add it to the interface.
                    let target = if matches!(mode, ChatType::Channel) && !input.is_channel() {
                        format!("#{input}")
                    } else {
                        input.to_owned()
                    };
                    state.core.chat_opened(&target);
                    input.clear();
                    response.request_focus();
                }
            });

            if let Err(reason) = validation_result {
                super::chat_validation_error(ui, reason);
            }
        });
    }

    fn show_chats(&mut self, state: &mut UIState, ui: &mut Ui, mode: ChatType) {
        let channel = state.core.clone();
        let active_chat_name = state.active_chat_tab_name.to_lowercase();

        // Use `relevant_chats` for reordering, and then shift all chats based on the shift within `relevant_chats`.
        let relevant_chats = state
            .chats()
            .iter()
            .enumerate()
            .filter(|ch| ch.1.category == mode)
            .map(|(i, chat)| ChatTabData {
                original_index: i,
                normalized_name: chat.normalized_name.clone(),
                name: chat.name.clone(),
                chat_state: chat.state.clone(),
                tab_state: chat.tab_state(),
            })
            .collect::<Vec<ChatTabData>>();
        let active_element_bg = ui.style().visuals.selection.bg_fill;

        let result = egui::ScrollArea::vertical()
            .id_salt(format!("{mode}-tabs"))
            .auto_shrink([false, true])
            .min_scrolled_height(MIN_CHAT_TABS_SCROLLVIEW_HEIGHT)
            .show(ui, |ui| {
                let items = relevant_chats.iter().map(|item| EnumeratedItem {
                    index: item.original_index,
                    item,
                });

                let response = egui_dnd::dnd(ui, format!("{mode}-tabs-drag-and-drop")).show(
                    items,
                    |ui, item, handle, _drag_state| {
                        handle.ui(ui, |ui| {
                            let normalized_chat_name = &item.item.normalized_name;

                            let background_colour = match *normalized_chat_name == active_chat_name
                            {
                                true => active_element_bg,
                                false => egui::Color32::TRANSPARENT,
                            };
                            let text_colour =
                                pick_tab_colour(&state.settings, &item.item.tab_state);

                            let label =
                                egui::RichText::new(item.item.name.clone()).color(text_colour);

                            let chat_tab = ui
                                .horizontal(|ui| {
                                    let button =
                                        ui.add(egui::Button::new(label).fill(background_colour));
                                    if matches!(item.item.chat_state, ChatState::JoinInProgress) {
                                        ui.spinner();
                                    }
                                    button
                                })
                                .inner;

                            if chat_tab.clicked() {
                                channel.chat_switch_requested(normalized_chat_name, None);
                            }

                            if mode != ChatType::System {
                                if chat_tab.middle_clicked() {
                                    channel.chat_tab_closed(normalized_chat_name);
                                }
                                chat_tab.context_menu(|ui| {
                                    tab_context_menu(ui, state, normalized_chat_name, &mode)
                                });
                            }
                        });
                    },
                );

                response.final_update()
            });

        if let Some(res) = result.inner {
            let original_idx_from = relevant_chats[res.from].original_index;
            let original_idx_to = match res.to == relevant_chats.len() {
                true => state.chats().len(),
                false => relevant_chats[res.to].original_index,
            };
            egui_dnd::utils::shift_vec(original_idx_from, original_idx_to, state.chats_mut());
        }
    }
}
