#![allow(clippy::upper_case_acronyms)]

use std::cmp;
use std::collections::BTreeSet;

use eframe::egui;

use crate::core::chat::ChatLike;
use crate::core::settings;

use crate::gui::state::UIState;

#[derive(Clone, Debug, Default, cmp::PartialEq, cmp::Eq)]
pub enum Tab {
    Application,
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
        let validation_result = super::validate_channel_name(&self.autojoin_channel_input);

        ui.vertical(|ui| {
            ui.heading("auto-join channels");
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
                    settings.chat.autojoin.insert(channel_name);
                    self.autojoin_channel_input.clear();
                    response.request_focus();
                }
            });
            if let Err(reason) = validation_result {
                super::chat_validation_error(ui, reason);
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
                for name in to_remove.iter() {
                    settings.chat.autojoin.remove(name);
                }
            });
        });
    }
}

#[derive(Default)]
pub struct Settings {
    active_tab: Tab,
    autojoin: AutojoinSection,
    username_input: String,
    username_colour_input: settings::Colour,
    visible_password: bool,

    highlights_input: String,
    notifications_builtin_sound: settings::BuiltInSound,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            visible_password: false,
            ..Default::default()
        }
    }

    fn show_active_tab_contents(
        &mut self,
        ctx: &egui::Context,
        ui: &mut eframe::egui::Ui,
        state: &mut UIState,
    ) {
        match self.active_tab {
            Tab::Application => self.show_application_tab(ctx, ui, state),
            Tab::Chat => self.show_chat_tab(ctx, ui, state),
            Tab::Interface => self.show_ui_tab(ui, state),
            Tab::Notifications => self.show_notifications_tab(ui, state),
        }
    }

    fn show_application_tab(
        &mut self,
        _ctx: &egui::Context,
        ui: &mut eframe::egui::Ui,
        state: &mut UIState,
    ) {
        let autoupdate = state.settings.application.autoupdate.enabled;

        ui.vertical(|ui| {
            ui.heading("general");
            let hint = format!(
                "checked every {} minutes. to apply an update, restart the application",
                crate::core::updater::AUTOUPDATE_INTERVAL_MINUTES
            );
            ui.checkbox(&mut state.settings.application.autoupdate.enabled, "enable automatic updates")
                .on_hover_text_at_pointer(hint);

            ui.heading("plugins");
            if !state.settings.application.plugins.enabled {
                ui.label(
                    "plugins are third-party modules (.dll, .so) which add extra functions. \
                to activate a plugin, place its file into the application's folder and restart it.\n\
                \n\
                beware that plugins can THEORETICALLY do anything in the application, or on your PC, that you can, so \
                only add them if you know and trust their authors.",
                );
            }

            ui.checkbox(
                &mut state.settings.application.plugins.enabled,
                "enable plugins",
            )
            .on_hover_text_at_pointer("requires application restart");
        });

        if autoupdate != state.settings.application.autoupdate.enabled {
            state
                .updater
                .enable_autoupdate(state.settings.application.autoupdate.enabled);
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
                                .hyperlink_to("IRC password", "https://osu.ppy.sh/p/irc")
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
                settings::ChatBackend::API => {
                    // TODO
                }
            }
        });
    }

    fn show_ui_tab(&mut self, ui: &mut eframe::egui::Ui, state: &mut UIState) {
        let suffix = match state.settings.ui.theme {
            settings::ThemeMode::Dark => "dark theme",
            settings::ThemeMode::Light => "light theme",
        };
        let validation_result = super::validate_username(&self.username_input);

        ui.vertical(|ui| {
            ui.heading(format!("chat colours ({suffix})"));

            ui.horizontal(|ui| {
                ui.color_edit_button_srgb(state.settings.ui.colours_mut().own.as_u8());
                ui.label("self")
                    .on_hover_text_at_pointer("the colour of your username");
            });

            ui.horizontal(|ui| {
                ui.color_edit_button_srgb(state.settings.ui.colours_mut().highlight.as_u8());
                ui.label("highlights").on_hover_text_at_pointer(
                    "the colour of chat messages and tabs containing unread highlights",
                );
            });

            ui.horizontal(|ui| {
                ui.color_edit_button_srgb(state.settings.ui.colours_mut().read_tabs.as_u8());
                ui.label("read tabs")
                    .on_hover_text_at_pointer("the colour of read chat tabs");
            });

            ui.horizontal(|ui| {
                ui.color_edit_button_srgb(state.settings.ui.colours_mut().unread_tabs.as_u8());
                ui.label("unread tabs")
                    .on_hover_text_at_pointer("the colour of unread chat tabs");
            });

            ui.horizontal(|ui| {
                ui.color_edit_button_srgb(state.settings.ui.colours_mut().default_users.as_u8());
                ui.label("default users")
                    .on_hover_text_at_pointer("default colour of all chat users");
            });

            ui.heading(format!("custom user colours ({suffix})"));

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.username_input)
                            .hint_text("username")
                            .desired_width(200.0),
                    );
                    ui.color_edit_button_srgb(self.username_colour_input.as_u8());
                    let add_user = ui
                        .button("add colour")
                        .on_hover_text_at_pointer("<Enter> = add");

                    let add_user = !self.username_input.is_empty()
                        && (add_user.clicked()
                            || (response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))));

                    if add_user && validation_result.is_ok() {
                        state.settings.ui.colours_mut().custom_users.insert(
                            self.username_input.to_lowercase().replace(' ', "_"),
                            self.username_colour_input.clone(),
                        );
                        self.username_input.clear();
                        response.request_focus();
                    }
                });

                if let Err(reason) = validation_result {
                    super::chat_validation_error(ui, reason);
                }
            });

            let mut to_remove = Vec::new();
            for (username, colour) in state.settings.ui.colours_mut().custom_users.iter_mut() {
                ui.horizontal(|ui| {
                    ui.color_edit_button_srgb(colour.as_u8());
                    let user_button = ui
                        .button(username)
                        .on_hover_text_at_pointer("middle click = remove");

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
                state.settings.ui.colours_mut().custom_users.remove(&name);
            }
            // });
        });
    }

    fn show_notifications_tab(&mut self, ui: &mut eframe::egui::Ui, state: &mut UIState) {
        ui.vertical(|ui| {
            ui.heading("highlights");
            ui.label("keywords");
            if self.highlights_input.is_empty() {
                self.highlights_input = state.settings.notifications.highlights.words.join(" ");
            }
            let hl = egui::TextEdit::multiline(&mut self.highlights_input)
                .hint_text("space-separated words");
            if ui
                .add(hl)
                .on_hover_text_at_pointer(
                    "list of words which will trigger notifications:\n\
                - exact case does not matter\n\
                - full match required (\"ha\" will not highlight a message with \"haha\")",
                )
                .changed()
            {
                state.update_highlights(&self.highlights_input);
            }

            ui.heading("notification sound");

            ui.radio_value(
                &mut state.settings.notifications.highlights.sound,
                None,
                "don't play anything",
            );

            let builtin_sound_chosen = matches!(
                state.settings.notifications.highlights.sound,
                Some(settings::Sound::BuiltIn(_))
            );
            ui.horizontal(|ui| {
                let mut response = ui.radio(builtin_sound_chosen, "built-in");
                let inner = egui::ComboBox::from_id_source("sound")
                    .selected_text(self.notifications_builtin_sound.to_string())
                    .show_ui(ui, |ui| {
                        let mut c = ui
                            .selectable_value(
                                &mut self.notifications_builtin_sound,
                                settings::BuiltInSound::Bell,
                                settings::BuiltInSound::Bell.to_string(),
                            )
                            .clicked();
                        c = c
                            || ui
                                .selectable_value(
                                    &mut self.notifications_builtin_sound,
                                    settings::BuiltInSound::DoubleBell,
                                    settings::BuiltInSound::DoubleBell.to_string(),
                                )
                                .clicked();

                        // \o /
                        if format!(
                            "{:x}",
                            md5::compute(state.settings.chat.irc.username.as_bytes())
                        ) == "cdb6d5ffca1edf2659aa721c19ccec1b"
                        {
                            c = c
                                || ui
                                    .selectable_value(
                                        &mut self.notifications_builtin_sound,
                                        settings::BuiltInSound::PartyHorn,
                                        settings::BuiltInSound::PartyHorn.to_string(),
                                    )
                                    .clicked();
                        }
                        c = c
                            || ui
                                .selectable_value(
                                    &mut self.notifications_builtin_sound,
                                    settings::BuiltInSound::Ping,
                                    settings::BuiltInSound::Ping.to_string(),
                                )
                                .clicked();
                        c = c
                            || ui
                                .selectable_value(
                                    &mut self.notifications_builtin_sound,
                                    settings::BuiltInSound::TwoTone,
                                    settings::BuiltInSound::TwoTone.to_string(),
                                )
                                .clicked();

                        c
                    });

                if response.clicked() || inner.inner.unwrap_or(false) {
                    state.settings.notifications.highlights.sound = Some(settings::Sound::BuiltIn(
                        self.notifications_builtin_sound.clone(),
                    ));
                    response.mark_changed();
                }

                let test_button = egui::Button::new("🔈");
                let button_clicked = match state.sound_player.functional() {
                    true => ui.add(test_button).clicked(),
                    false => {
                        let error_text = match state.sound_player.initialization_error() {
                            None => "unknown initialization error".into(),
                            Some(e) => e.to_string(),
                        };
                        ui.add_enabled(false, test_button)
                            .on_disabled_hover_text(error_text);
                        false
                    }
                };

                if button_clicked {
                    state.sound_player.play(&settings::Sound::BuiltIn(
                        self.notifications_builtin_sound.clone(),
                    ));
                }
            });

            // TODO: implement custom sound picker
            // There is no centralized egui-based file dialog solution. nfd2 pulls up GTK3, tinyfiledialogs seems to crash when used naively.
            // Need to either implement it myself, or check potential leads from https://github.com/emilk/egui/issues/270
        });
    }

    pub fn show(&mut self, ctx: &eframe::egui::Context, state: &mut UIState, is_open: &mut bool) {
        if let Some(settings::Sound::BuiltIn(sound)) =
            &state.settings.notifications.highlights.sound
        {
            self.notifications_builtin_sound = sound.clone();
        }

        let mut save_clicked = false;
        egui::Window::new("settings").open(is_open).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.selectable_value(&mut self.active_tab, Tab::Application, "application");
                    ui.selectable_value(&mut self.active_tab, Tab::Chat, "chat");
                    ui.selectable_value(&mut self.active_tab, Tab::Interface, "interface");
                    ui.selectable_value(&mut self.active_tab, Tab::Notifications, "notifications");
                });
                ui.separator();
                self.show_active_tab_contents(ctx, ui, state);
            });

            ui.add_space(10.);

            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    let reset_button = egui::Button::new("reset");
                    if ui
                        .add(reset_button)
                        .on_hover_text_at_pointer("double click to discard changes")
                        .double_clicked()
                    {
                        state.core.settings_requested();
                    }
                    if ui
                        .button("save settings")
                        .on_hover_text_at_pointer("save active settings to file")
                        .clicked()
                    {
                        state.core.settings_updated(&state.settings.clone());
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
