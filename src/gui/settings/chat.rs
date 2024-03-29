use eframe::egui;
use std::collections::BTreeSet;

use super::SettingsWindow;
use crate::gui::state::UIState;
use steel_core::chat::ChatLike;
use steel_core::settings::{ChatBackend, Settings};

#[derive(Default)]
pub struct AutojoinSection {
    autojoin_channel_input: String,
}

impl AutojoinSection {
    pub fn show(&mut self, settings: &mut Settings, ui: &mut eframe::egui::Ui) {
        let validation_result = crate::gui::validate_channel_name(&self.autojoin_channel_input);

        ui.vertical(|ui| {
            ui.heading("auto-join channels").on_hover_text_at_pointer(
                "these channels will be open automatically when you connect to the server -- think of them as favourites"
            );
            ui.horizontal(|ui| {
                let add_autojoin_channel = ui.button("+").on_hover_text_at_pointer("<Enter> = add");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.autojoin_channel_input)
                        .hint_text("channel name"),
                );

                let add_autojoin_channel = !self.autojoin_channel_input.is_empty()
                    && (add_autojoin_channel.clicked()
                        || (response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))));

                if add_autojoin_channel && validation_result.is_ok() {
                    let channel_name = if self.autojoin_channel_input.is_channel() {
                        self.autojoin_channel_input.to_owned()
                    } else {
                        format!("#{}", self.autojoin_channel_input)
                    };
                    if !settings.chat.autojoin.contains(&channel_name) {
                        settings.chat.autojoin.push(channel_name);
                    }
                    self.autojoin_channel_input.clear();
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
                for name in settings.chat.autojoin.iter() {
                    let channel_button = ui
                        .button(name)
                        .on_hover_text_at_pointer("middle click = remove");
                    let mut remove_channel = channel_button.middle_clicked();
                    channel_button.context_menu(|ui| {
                        if ui.button("Remove").clicked() {
                            remove_channel = true;
                            ui.close_menu();
                        }
                    });
                    if remove_channel {
                        to_remove.insert(name.to_owned());
                    }
                }
                settings.chat.autojoin.retain(|s| !to_remove.contains(s));
            });
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
            });

            self.autojoin.show(&mut state.settings, ui);
        });
    }
}
