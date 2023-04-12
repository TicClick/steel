#![allow(clippy::upper_case_acronyms)]

use std::cmp;
use std::collections::BTreeSet;

use eframe::egui;

use crate::core::chat::ChatLike;
use crate::core::settings;

use super::UIState;
use crate::app::AppMessageIn;

#[derive(Clone, Debug, Default, cmp::PartialEq, cmp::Eq)]
pub enum Tab {
    #[default]
    Chat,
    Interface,
    Notifications,
}

#[derive(Default)]
pub struct AutojoinSection {
    autojoin_channel_input: String,
}

impl AutojoinSection {
    pub fn show(&mut self, settings: &mut settings::Settings, ui: &mut eframe::egui::Ui) {
        ui.collapsing("auto-join channels", |ui| {
            ui.horizontal(|ui| {
                // TODO: this will overflow the window if too many channels are added
                let add_autojoin_channel = ui.button("+");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.autojoin_channel_input)
                        .hint_text("channel name"),
                );

                let add_autojoin_channel = !self.autojoin_channel_input.is_empty()
                    && (add_autojoin_channel.clicked()
                        || (response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))));

                if add_autojoin_channel {
                    let channel_name = if self.autojoin_channel_input.is_channel() {
                        self.autojoin_channel_input.to_owned()
                    } else {
                        format!("#{}", self.autojoin_channel_input)
                    };
                    settings.chat.autojoin.insert(channel_name);
                    self.autojoin_channel_input.clear();
                    response.request_focus();
                }
            });
            let mut to_remove = BTreeSet::new();
            for name in settings.chat.autojoin.iter() {
                let channel_button = ui.button(name);
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
            for name in to_remove.iter() {
                settings.chat.autojoin.remove(name);
            }
        });
    }
}

#[derive(Default)]
pub struct Settings {
    active_tab: Tab,
    autojoin: AutojoinSection,
    username_input: String,
    username_colour_input: settings::Colour,
    highlights_input: String,
}

impl Settings {
    fn show_active_tab_contents(
        &mut self,
        ctx: &egui::Context,
        ui: &mut eframe::egui::Ui,
        state: &mut UIState,
    ) {
        match self.active_tab {
            Tab::Chat => self.show_chat_tab(ctx, ui, state),
            Tab::Interface => self.show_ui_tab(ui, state),
            Tab::Notifications => self.show_notifications_tab(ui, state),
        }
    }

    fn show_chat_tab(
        &mut self,
        _ctx: &egui::Context,
        ui: &mut eframe::egui::Ui,
        state: &mut UIState,
    ) {
        ui.vertical(|ui| {
            ui.heading("general");
            ui.horizontal(|ui| {
                ui.checkbox(&mut state.settings.chat.autoconnect, "connect on startup");
            });
            self.autojoin.show(&mut state.settings, ui);

            ui.heading("access");
            ui.horizontal(|ui| {
                ui.label("chat transport");
                ui.radio_value(
                    &mut state.settings.chat.backend,
                    settings::ChatBackend::IRC,
                    "IRC",
                )
                .on_hover_text_at_pointer(
                    "legacy chat with a separate password.\n\
                scheduled for deprecation, lacks features.\n\
                lightweight and battle-tested.",
                );

                // TODO: implement
                if false {
                    ui.radio_value(
                        &mut state.settings.chat.backend,
                        settings::ChatBackend::API,
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
                settings::ChatBackend::IRC => {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("username");
                            ui.text_edit_singleline(&mut state.settings.chat.irc.username);
                        });
                        ui.horizontal(|ui| {
                            ui.hyperlink_to("password", "https://osu.ppy.sh/p/irc");
                            ui.text_edit_singleline(&mut state.settings.chat.irc.password);
                        });
                    });
                }
                settings::ChatBackend::API => {
                    // TODO
                }
            }
        });
    }

    fn show_ui_tab(&mut self, ui: &mut eframe::egui::Ui, state: &mut UIState) {
        ui.vertical(|ui| {
            ui.heading("chat colours");
            ui.horizontal(|ui| {
                ui.color_edit_button_srgb(state.settings.ui.colours.own.as_u8());
                ui.label("self");
            });
            ui.collapsing("other users", |ui| {
                // TODO: this will overflow the window if too many users are added
                ui.horizontal(|ui| {
                    let add_user = ui.button("+");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.username_input).hint_text("username"),
                    );
                    ui.color_edit_button_srgb(self.username_colour_input.as_u8());

                    let add_user = !self.username_input.is_empty()
                        && (add_user.clicked()
                            || (response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))));

                    if add_user {
                        state.settings.ui.colours.users.insert(
                            self.username_input.to_lowercase(),
                            self.username_colour_input.clone(),
                        );
                        self.username_input.clear();
                        response.request_focus();
                    }
                });

                let mut to_remove = Vec::new();
                for (username, colour) in state.settings.ui.colours.users.iter_mut() {
                    ui.horizontal(|ui| {
                        let user_button = ui.button(username);
                        ui.color_edit_button_srgb(colour.as_u8());

                        let mut remove_user = user_button.middle_clicked();
                        user_button.context_menu(|ui| {
                            if ui.button("Remove").clicked() {
                                remove_user = true;
                                ui.close_menu();
                            }
                        });
                        if remove_user {
                            to_remove.push(username.clone());
                        }
                    });
                }
                for name in to_remove {
                    state.settings.ui.colours.users.remove(&name);
                }
            });
        });
    }

    fn show_notifications_tab(&mut self, ui: &mut eframe::egui::Ui, state: &mut UIState) {
        ui.vertical(|ui| {
            ui.heading("highlights");
            ui.horizontal(|ui| {
                ui.label("colour");
                ui.color_edit_button_srgb(state.settings.notifications.highlights.colour.as_u8());
            });

            // TODO: implement sounds
            if false {
                ui.horizontal(|ui| {
                    ui.label("sound");
                    egui::ComboBox::from_id_source("sound")
                        .selected_text(match &state.settings.notifications.highlights.sound {
                            None => "None".into(),
                            Some(s) => format!("{}", s),
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut state.settings.notifications.highlights.sound,
                                None,
                                "None",
                            );
                            ui.selectable_value(
                                &mut state.settings.notifications.highlights.sound,
                                Some(settings::Sound::Coin),
                                "Coin",
                            );
                            ui.selectable_value(
                                &mut state.settings.notifications.highlights.sound,
                                Some(settings::Sound::PartyHorn),
                                "Party horn",
                            );
                            ui.selectable_value(
                                &mut state.settings.notifications.highlights.sound,
                                Some(settings::Sound::Bleep),
                                "Bleep",
                            );
                        });
                    if ui.button("🔈").clicked() {
                        // TODO: play sound
                    }
                });
            }

            if self.highlights_input.is_empty() {
                self.highlights_input = state.settings.notifications.highlights.words.join(" ");
            }
            let hl = egui::TextEdit::multiline(&mut self.highlights_input)
                .hint_text("list of words, separated by space");
            if ui.add(hl).changed() {
                state.update_highlights(&self.highlights_input);
            }
        });
    }

    pub fn show(&mut self, ctx: &eframe::egui::Context, state: &mut UIState, is_open: &mut bool) {
        let mut save_clicked = false;
        egui::Window::new("settings").open(is_open).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.selectable_value(&mut self.active_tab, Tab::Chat, "Chat");
                    ui.selectable_value(&mut self.active_tab, Tab::Interface, "Interface");
                    ui.selectable_value(&mut self.active_tab, Tab::Notifications, "Notifications");
                });
                ui.separator();
                self.show_active_tab_contents(ctx, ui, state);
            });

            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if ui.button("reset").clicked() {
                        state
                            .app_queue_handle
                            .blocking_send(AppMessageIn::UISettingsRequested)
                            .unwrap();
                    }
                    if ui.button("save").clicked() {
                        state
                            .app_queue_handle
                            .blocking_send(AppMessageIn::UISettingsUpdated(state.settings.clone()))
                            .unwrap();
                        save_clicked = true;
                    }
                });
            });
        });
        if save_clicked {
            *is_open = false;
        }
    }
}
