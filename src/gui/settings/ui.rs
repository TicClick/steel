use eframe::egui;

use steel_core::settings::ThemeMode;

use super::SettingsWindow;
use crate::gui::state::UIState;

impl SettingsWindow {
    pub(super) fn show_ui_tab(&mut self, ui: &mut eframe::egui::Ui, state: &mut UIState) {
        let suffix = match state.settings.ui.theme {
            ThemeMode::Dark => "dark theme",
            ThemeMode::Light => "light theme",
        };
        let validation_result = crate::gui::validate_username(&self.username_input);

        ui.vertical(|ui| {
            ui.heading("application");
            ui.horizontal(|ui| {
                ui.label("interface scaling");
                let previous_frame_scaling_value = state.settings.ui.scaling;
                let font_size_slider = egui::Slider::new(&mut state.settings.ui.scaling, 1.0..=2.5)
                    .fixed_decimals(2)
                    .drag_value_speed(0.1);

                let resp = ui.add(font_size_slider);
                if (previous_frame_scaling_value != state.settings.ui.scaling && !resp.dragged())
                    || (resp.drag_stopped())
                {
                    ui.ctx().set_pixels_per_point(state.settings.ui.scaling);
                }
            });

            ui.heading(format!("chat colours ({suffix})"));

            ui.label("usernames");
            ui.indent("colours-usernames", |ui| {
                ui.horizontal(|ui| {
                    ui.color_edit_button_srgb(state.settings.ui.colours_mut().own.as_u8());
                    ui.label("self")
                        .on_hover_text_at_pointer("the colour of your username");
                });

                ui.horizontal(|ui| {
                    ui.color_edit_button_srgb(
                        state.settings.ui.colours_mut().default_users.as_u8(),
                    );
                    ui.label("default users")
                        .on_hover_text_at_pointer("default colour of all chat users");
                });

                ui.horizontal(|ui| {
                    ui.color_edit_button_srgb(state.settings.ui.colours_mut().moderators.as_u8());
                    ui.label("moderators").on_hover_text_at_pointer(
                        "default colour of all moderators (GMT, NAT, DEV)",
                    );
                });
            });

            ui.label("chat messages");
            ui.indent("colours-messages", |ui| {
                ui.horizontal(|ui| {
                    ui.color_edit_button_srgb(state.settings.ui.colours_mut().highlight.as_u8());
                    ui.label("highlights").on_hover_text_at_pointer(
                        "the colour of chat messages and tabs containing unread highlights",
                    );
                });

                ui.horizontal(|ui| {
                    ui.color_edit_button_srgb(
                        state
                            .settings
                            .ui
                            .colours_mut()
                            .search_result_current
                            .as_u8(),
                    );
                    ui.label("current search result").on_hover_text_at_pointer(
                        "colour of the currently selected search result in chat",
                    );
                });

                ui.horizontal(|ui| {
                    ui.color_edit_button_srgb(
                        state.settings.ui.colours_mut().search_result_other.as_u8(),
                    );
                    ui.label("other search results")
                        .on_hover_text_at_pointer("colour of other search results in chat");
                });
            });

            ui.label("tabs");
            ui.indent("colours-tabs", |ui| {
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
            });

            ui.heading(format!("custom user colours ({suffix})")).on_hover_text_at_pointer(
                "works for all the folks out there: exquisite, boring, dangerous, ~ s p e c i a l ~"
            );

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
                crate::gui::chat_validation_error(ui, reason);
            }

            let username_row_height = *self.text_row_height.get_or_insert_with(|| {
                ui.text_style_height(&egui::TextStyle::Body) + 2. * ui.spacing().item_spacing.y
            });
            let area_height = username_row_height
                * (state.settings.ui.colours().custom_users.len().clamp(0, 10) as f32);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .min_scrolled_height(area_height)
                .show(ui, |ui| {
                    let mut to_remove = Vec::new();
                    for (username, colour) in
                        state.settings.ui.colours_mut().custom_users.iter_mut()
                    {
                        ui.horizontal(|ui| {
                            ui.color_edit_button_srgb(colour.as_u8());
                            let user_button = ui
                                .button(username)
                                .on_hover_text_at_pointer("middle click = remove");

                            let mut remove_user = user_button.middle_clicked();
                            user_button.context_menu(|ui| {
                                if ui.button("Remove").clicked() {
                                    remove_user = true;
                                    ui.close();
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
                });
        });
    }
}
