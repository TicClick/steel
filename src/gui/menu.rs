use eframe::egui;

use crate::core::settings::ui::ThemeMode;
use steel_core::chat::ConnectionStatus;

use crate::gui::state::UIState;

#[derive(Default)]
pub struct Menu {
    pub show_settings: bool,
    pub show_about: bool,
    pub show_update: bool,
    pub show_usage: bool,

    pin_window: bool,
}

impl Menu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dialogs_visible(&self) -> bool {
        self.show_settings || self.show_about || self.show_update || self.show_usage
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
        state: &mut UIState,
        response_widget_id: &mut Option<egui::Id>,
    ) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                if let Some(theme) = ctx.style().visuals.light_dark_small_toggle_button(ui) {
                    let old_theme = state.settings.ui.theme.clone();
                    state.settings.ui.theme = if theme.dark_mode {
                        ThemeMode::Dark
                    } else {
                        ThemeMode::Light
                    };
                    if state.settings.ui.theme != old_theme {
                        state.core.settings_updated(&state.settings);
                    }
                }

                self.show_application_menu(ui, ctx, frame, state);
                self.show_chat_menu(ui, ctx, state, response_widget_id);
                self.show_help_menu(ui, ctx, state);

                let resp = ui.checkbox(&mut self.pin_window, "📌").on_hover_text(
                    "- put the window on top of everything and hide its border\n\
                        - to move the window, click and drag this button",
                );

                if resp.clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                        egui::WindowLevel::AlwaysOnTop,
                    ));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(!self.pin_window));
                } else if resp.is_pointer_button_down_on() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
            });
        });
    }

    fn show_application_menu(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
        _state: &mut UIState,
    ) {
        ui.menu_button("application", |ui| {
            if ui.button("settings").clicked() {
                self.show_settings = !self.show_settings;
                ui.close_menu();
            }

            ui.separator();

            if ui.button("restart").clicked() {
                crate::core::os::restart();
                ui.close_menu();
            }
            if ui.button("exit").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                ui.close_menu();
            }
        });
    }

    fn show_chat_menu(
        &mut self,
        ui: &mut egui::Ui,
        _ctx: &egui::Context,
        state: &mut UIState,
        response_widget_id: &mut Option<egui::Id>,
    ) {
        ui.menu_button("chat", |ui| {
            let (action, enabled) = match state.connection {
                ConnectionStatus::Disconnected { .. } => ("connect".to_owned(), true),
                ConnectionStatus::InProgress => ("connecting...".to_owned(), false),
                ConnectionStatus::Scheduled(when) => {
                    let action = format!(
                        "reconnecting ({}s)",
                        (when - chrono::Local::now()).num_seconds()
                    );
                    (action, true)
                }
                ConnectionStatus::Connected => ("disconnect".to_owned(), true),
            };
            if ui
                .add_enabled(enabled, egui::Button::new(egui::RichText::new(action)))
                .clicked()
            {
                match state.connection {
                    ConnectionStatus::Disconnected { .. } | ConnectionStatus::Scheduled(_) => {
                        state.core.connect_requested()
                    }
                    ConnectionStatus::InProgress => (),
                    ConnectionStatus::Connected => {
                        response_widget_id.take();
                        state.core.disconnect_requested();
                    }
                }
                ui.close_menu();
            }
        });
    }

    fn show_help_menu(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context, _state: &mut UIState) {
        ui.menu_button("help", |ui| {
            if ui.button("open diagnostic log").clicked() {
                crate::core::os::open_runtime_log();
                ui.close_menu();
            }
            if ui.button("check for updates").clicked() {
                self.show_update = !self.show_update;
                ui.close_menu();
            }

            ui.separator();

            if ui.button("usage guide").clicked() {
                self.show_usage = !self.show_usage;
                ui.close_menu();
            }
            if ui.button("about").clicked() {
                self.show_about = !self.show_about;
                ui.close_menu();
            }
        });
    }
}
