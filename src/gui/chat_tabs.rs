use std::collections::BTreeSet;

use eframe::egui::{self, Color32, Frame, Margin, Ui};
use steel_core::settings::Colour;

use crate::core::chat::{ChatLike, ChatState, ChatType};

use crate::gui::highlights::UnreadType;
use crate::gui::state::UIState;

use super::context_menu::shared::menu_item_open_chat_log;

const MIN_CHAT_TABS_SCROLLVIEW_HEIGHT: f32 = 180.;

#[derive(Default)]
pub struct ChatTabs {
    pub new_channel_input: String,
    pub new_chat_input: String,
}

impl ChatTabs {
    pub fn show(&mut self, ctx: &egui::Context, state: &mut UIState) {
        let frame_maker = || Frame::none().inner_margin(Margin::symmetric(0., 2.));

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
                    self.show_chats(state,  ui, ChatType::Channel);
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
                    self.show_system_tabs(state, ui);
                });
        });
    }
}

fn pick_tab_colour(state: &UIState, normalized_chat_name: &str) -> Colour {
    let colour = match state.highlights.unread_type(normalized_chat_name) {
        None => &state.settings.ui.colours().read_tabs,
        Some(unread) => match unread {
            UnreadType::Highlight => &state.settings.ui.colours().highlight,
            UnreadType::Regular => &state.settings.ui.colours().unread_tabs,
        },
    };
    colour.clone()
}

fn tab_context_menu(
    ui: &mut Ui,
    state: &mut UIState,
    normalized_chat_name: &str,
    mode: &ChatType,
    chats_to_clear: &mut BTreeSet<String>,
) {
    if state
        .settings
        .chat
        .autojoin
        .iter()
        .any(|s| s == normalized_chat_name)
    {
        if ui.button("Remove from favourites").clicked() {
            state
                .settings
                .chat
                .autojoin
                .retain(|s| s != normalized_chat_name);
            // TODO: this should be done elsewhere, in a centralized manner, I'm just being lazy right now
            state.core.settings_updated(&state.settings);
            ui.close_menu();
        }
    } else if ui.button("Add to favourites").clicked() {
        state
            .settings
            .chat
            .autojoin
            .push(normalized_chat_name.to_owned());
        // TODO: this should be done elsewhere, in a centralized manner, I'm just being lazy right now
        state.core.settings_updated(&state.settings);
        ui.close_menu();
    }

    menu_item_open_chat_log(ui, state, false, normalized_chat_name);

    if ui.button("Clear messages").clicked() {
        chats_to_clear.insert(normalized_chat_name.to_owned());
        ui.close_menu();
    }

    ui.separator();

    let close_title = match mode {
        ChatType::Channel => "Leave",
        ChatType::Person => "Close",
    };
    if ui.button(close_title).clicked() {
        state.core.chat_tab_closed(normalized_chat_name);
        ui.close_menu();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Location {
    row: usize,
}

impl ChatTabs {
    fn show_new_chat_input(&mut self, state: &mut UIState, ui: &mut Ui, mode: ChatType) {
        let input: &mut String = match mode {
            ChatType::Channel => &mut self.new_channel_input,
            ChatType::Person => &mut self.new_chat_input,
        };

        let validation_result = match mode {
            ChatType::Channel => super::validate_channel_name(input),
            ChatType::Person => super::validate_username(input),
        };

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let add_chat = ui.button("+").on_hover_text_at_pointer(
                    "<Enter> = add\n\
                    <Shift> + drag = reorder\n\
                    Middle click = close",
                );
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
                        || (response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))));

                if add_chat && validation_result.is_ok() {
                    // Whether the channel is valid or not is determined by the server (will send us a message),
                    // but for now let's add it to the interface.
                    let target = if matches!(mode, ChatType::Channel) && !input.is_channel() {
                        format!("#{}", input)
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

    fn show_chats(
        &mut self,
        state: &mut UIState,
        ui: &mut Ui,
        mode: ChatType,
    ) {
        let it: Vec<(usize, String, String, ChatState)> = state
            .filter_chats(|(_, ch)| match mode {
                ChatType::Channel => ch.name.is_channel(),
                ChatType::Person => !ch.name.is_channel(),
            })
            .map(|(i, ch)| {
                (
                    i,
                    ch.name.to_lowercase(),
                    ch.name.to_owned(),
                    ch.state.to_owned(),
                )
            })
            .collect();

        let mut chats_to_clear = BTreeSet::<String>::new();
        let mut from = None;
        let mut to = None;

        egui::ScrollArea::vertical()
            .id_source(format!("{mode}-tabs"))
            .auto_shrink([false, true])
            .min_scrolled_height(MIN_CHAT_TABS_SCROLLVIEW_HEIGHT)
            .show(ui, |ui| {
                let frame = egui::Frame::default().inner_margin(4.0);
                let (_, dropped_payload) = ui.dnd_drop_zone::<Location, ()>(frame, |ui| {
                    for (interleaved_pos, normalized_chat_name, chat_name, chat_state) in it {
                        let item_id = egui::Id::new(&normalized_chat_name);
                        let item_location = Location { row: interleaved_pos };

                        let tab_label = egui::RichText::new(chat_name)
                            .color(pick_tab_colour(state, &normalized_chat_name));

                        let response = ui.dnd_drag_source(item_id, item_location, |ui| {
                            let chat_tab = ui.selectable_value(
                                &mut state.active_chat_tab_name,
                                normalized_chat_name.to_owned(),
                                tab_label,
                            );

                            if matches!(chat_state, ChatState::JoinInProgress) {
                                ui.spinner();
                            }

                            if chat_tab.clicked() {
                                state
                                    .core
                                    .chat_switch_requested(&state.active_chat_tab_name, None);
                            }

                            if chat_tab.middle_clicked() {
                                state.core.chat_tab_closed(&normalized_chat_name);
                            }

                            chat_tab.context_menu(|ui| {
                                tab_context_menu(
                                    ui,
                                    state,
                                    &normalized_chat_name,
                                    &mode,
                                    &mut chats_to_clear,
                                )
                            });
                            chat_tab
                        }).response;

                        // Detect drops onto the item.
                        if let (Some(pointer), Some(hovered_payload)) = (
                            ui.input(|i| i.pointer.interact_pos()),
                            response.dnd_hover_payload::<Location>()
                        ) {
                            let rect = response.rect;

                            // Preview insertion:
                            let stroke = egui::Stroke::new(1.0, Color32::WHITE);
                            let insert_row_idx = if *hovered_payload == item_location {
                                // We are dragged onto ourselves
                                ui.painter().hline(rect.x_range(), rect.center().y, stroke);
                                interleaved_pos
                            } else if pointer.y < rect.center().y {
                                // Above us
                                ui.painter().hline(rect.x_range(), rect.top(), stroke);
                                interleaved_pos
                            } else {
                                // Below us
                                ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
                                interleaved_pos + 1
                            };

                            if let Some(dragged_payload) = response.dnd_release_payload::<Location>() {
                                // The user dropped onto this item.
                                from = Some(dragged_payload);
                                to = Some(Location {
                                    row: insert_row_idx,
                                });
                            }
                        }
                    }
                });

                if let Some(dragged_payload) = dropped_payload {
                    // The user dropped onto the column, but not on any one item.
                    from = Some(dragged_payload);
                    to = Some(Location {
                        row: usize::MAX, // Inset last
                    });
                }
            });

        if let (Some(from), Some(mut to)) = (from, to) {
            if to.row == usize::MAX {
                state.place_tab_after(from.row, state.chat_count() - 1);
            } else {
                to.row -= (from.row < to.row) as usize;
                state.place_tab_after(from.row, to.row);
            }
        }

        for target in chats_to_clear {
            state.clear_chat(&target);
        }
    }

    fn show_system_tabs(&self, state: &mut UIState, ui: &mut Ui) {
        for label in [
            super::HIGHLIGHTS_TAB_NAME.to_owned(),
            super::SERVER_TAB_NAME.to_owned(),
        ] {
            let coloured_label =
                egui::RichText::new(label[1..].to_string()).color(pick_tab_colour(state, &label));
            let chat_tab = ui.selectable_value(
                &mut state.active_chat_tab_name,
                label.to_owned(),
                coloured_label,
            );
            if chat_tab.clicked() {
                state
                    .core
                    .chat_switch_requested(&state.active_chat_tab_name, None);
            }
        }
    }
}
