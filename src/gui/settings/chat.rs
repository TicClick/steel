use eframe::egui;
use std::collections::BTreeSet;
use steel_core::settings::chat::ChatPosition;

use super::SettingsWindow;
use crate::gui::state::UIState;
use steel_core::chat::{ChatLike, ChatType};
use steel_core::settings::{ChatBackend, Settings};

#[derive(Default)]
pub struct AutojoinSection {
    autojoin_channel_input: String,
    autojoin_user_input: String,
}

impl AutojoinSection {
    fn display_editable_container(
        settings: &mut Settings,
        ui: &mut eframe::egui::Ui,
        input: &mut String,
        input_type: ChatType,
    ) {
        let validation_result = match input_type {
            ChatType::Channel => crate::gui::validate_channel_name(input),
            ChatType::Person => crate::gui::validate_username(input),
        };
        ui.horizontal(|ui| {
            let add_item = ui.button("+").on_hover_text_at_pointer("<Enter> = add");
            let response = ui.add(egui::TextEdit::singleline(input).hint_text("name"));

            let add_item = !input.is_empty()
                && (add_item.clicked()
                    || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))));

            if add_item && validation_result.is_ok() {
                let input_string = match input_type {
                    ChatType::Channel => {
                        if input.is_channel() {
                            input.to_owned()
                        } else {
                            format!("#{}", input)
                        }
                    }
                    ChatType::Person => input.to_owned(),
                };

                if !settings.chat.autojoin.contains(&input_string) {
                    settings.chat.autojoin.push(input_string);
                }
                input.clear();
                response.request_focus();
            }
        });
        if let Err(reason) = validation_result {
            crate::gui::chat_validation_error(ui, reason);
        }

        let mut to_remove = BTreeSet::new();
        let layout = egui::Layout::left_to_right(egui::Align::Max).with_main_wrap(true);
        ui.with_layout(layout, |ui| {
            ui.spacing_mut().item_spacing.x /= 2.;
            for name in settings
                .chat
                .autojoin
                .iter()
                .filter(|item| match input_type {
                    ChatType::Channel => item.is_channel(),
                    ChatType::Person => !item.is_channel(),
                })
            {
                let item_button = ui
                    .button(name)
                    .on_hover_text_at_pointer("middle click = remove");
                let mut remove_item = item_button.middle_clicked();
                item_button.context_menu(|ui| {
                    if ui.button("Remove").clicked() {
                        remove_item = true;
                        ui.close();
                    }
                });
                if remove_item {
                    to_remove.insert(name.to_owned());
                }
            }
            settings.chat.autojoin.retain(|s| !to_remove.contains(s));
        });
    }

    pub fn show(&mut self, settings: &mut Settings, ui: &mut eframe::egui::Ui) {
        ui.vertical(|ui| {
            ui.heading("auto-join channels").on_hover_text_at_pointer(
                "your favourite channels -- they will be automatically opened after connecting",
            );
            Self::display_editable_container(
                settings,
                ui,
                &mut self.autojoin_channel_input,
                ChatType::Channel,
            );
        });

        ui.vertical(|ui| {
            ui.heading("auto-open private messages").on_hover_text_at_pointer(
                "your favourite users -- chats with them will be automatically opened after connecting",
            );
            Self::display_editable_container(
                settings,
                ui,
                &mut self.autojoin_user_input,
                ChatType::Person,
            );
        });
    }
}

impl SettingsWindow {
    pub(super) fn show_chat_tab(
        &mut self,
        _ctx: &egui::Context,
        ui: &mut eframe::egui::Ui,
        state: &mut UIState,
    ) {
        ui.vertical(|ui| {
            ui.heading("connection");

            ui.horizontal(|ui| {
                ui.label("chat transport");
                ui.radio_value(&mut state.settings.chat.backend, ChatBackend::IRC, "IRC")
                    .on_hover_text_at_pointer(
                        "legacy chat with a separate password.\n\
                scheduled for deprecation, lacks features.\n\
                lightweight and battle-tested.",
                    );

                // TODO: implement
                if false {
                    ui.radio_value(
                        &mut state.settings.chat.backend,
                        ChatBackend::API,
                        "osu!api",
                    )
                    .on_hover_text_at_pointer(
                        "the system behind the modern web chat.\n\
                it sends a lot of useful details and context.\n\
                mostly complete, but still experimental.",
                    );
                }
            });
            match state.settings.chat.backend {
                ChatBackend::IRC => {
                    ui.vertical(|ui| {
                        let total_width = ui
                            .horizontal(|ui| {
                                let mut sz = ui.label("username").rect.width();
                                sz += ui
                                    .text_edit_singleline(&mut state.settings.chat.irc.username)
                                    .on_hover_text_at_pointer("replace spaces with underscores")
                                    .rect
                                    .width();
                                sz
                            })
                            .inner;

                        ui.horizontal(|ui| {
                            let label_width = ui
                                .hyperlink_to(
                                    "IRC password",
                                    "https://osu.ppy.sh/home/account/edit#legacy-api",
                                )
                                .rect
                                .width();
                            let input =
                                egui::TextEdit::singleline(&mut state.settings.chat.irc.password)
                                    .password(!self.visible_password)
                                    .desired_width(total_width - label_width - 26.);
                            ui.add(input).on_hover_text_at_pointer(
                                "if you don't have an IRC password, click the link on the left",
                            );
                            if ui
                                .button("👁")
                                .on_hover_text_at_pointer("show/hide password")
                                .clicked()
                            {
                                self.visible_password = !self.visible_password;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("IRC server");
                            let input =
                                egui::TextEdit::singleline(&mut state.settings.chat.irc.server);
                            ui.add(input).on_hover_text_at_pointer(
                                "possible options: \n\
                                - cho.ppy.sh\n\
                                - irc.ppy.sh"
                            );
                        });

                        ui.horizontal(|ui: &mut egui::Ui| {
                            ui.label("ping timeout");
                            let input = egui::Slider::new(&mut state.settings.chat.irc.ping_timeout, 15..=120).integer();
                            ui.add(input).on_hover_text_at_pointer(
                                "if the server doesn't respond to IRC PING (regular status checks) for this amount of time, reconnect.\n\
                                large values help when on slow/unstable network, but may keep you hanging."
                            );
                        });
                    });
                }
                ChatBackend::API => {
                    // TODO
                }
            }

            ui.separator();

            ui.checkbox(&mut state.settings.chat.autoconnect, "connect on startup")
                .on_hover_text_at_pointer(
                    "when launched, connect to the chat automatically using your credentials",
                );

            ui.checkbox(
                &mut state.settings.chat.reconnect,
                "automatically reconnect",
            )
            .on_hover_text_at_pointer(
                "If gone offline, try connecting to the chat every 15 seconds",
            );

            ui.heading("behaviour");
            ui.vertical(|ui| {
                ui.checkbox(
                    &mut state.settings.chat.behaviour.handle_osu_chat_links,
                    "handle osu! chat links",
                )
                .on_hover_text_at_pointer(
                    "open/switch to channels directly in the app instead of doing it in osu!.\n\
                    affected links: osu://chan/",
                );

                ui.checkbox(
                    &mut state.settings.chat.behaviour.handle_osu_beatmap_links,
                    "handle osu! beatmap links",
                )
                .on_hover_text_at_pointer(
                    "open beatmap links in the browser instead of doing it in osu!.\n\
                    affected links: osu://dl/, osu://dl/b/, osu://dl/s/, osu://b/",
                );

                ui.checkbox(
                    &mut state.settings.chat.behaviour.track_unread_messages,
                    "mark new messages",
                )
                .on_hover_text_at_pointer(
                    "mark the end of read messages in inactive tabs, so that you know what's new",
                );

                ui.horizontal(|ui| {
                    ui.label("chat position:");
                    let chat_position_label =
                        state.settings.chat.behaviour.chat_position.to_string();
                    egui::ComboBox::from_id_salt("chat-position")
                        .selected_text(chat_position_label)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut state.settings.chat.behaviour.chat_position,
                                ChatPosition::Top,
                                ChatPosition::Top.to_string(),
                            );
                            ui.selectable_value(
                                &mut state.settings.chat.behaviour.chat_position,
                                ChatPosition::Bottom,
                                ChatPosition::Bottom.to_string(),
                            );
                        })
                });
            });

            self.autojoin.show(&mut state.settings, ui);
        });
    }
}
