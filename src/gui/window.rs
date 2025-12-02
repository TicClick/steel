use eframe::egui::{self, Theme};
use steel_core::chat::Message;
use steel_core::ipc::client::CoreClient;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::gui::chat::chat_controller::ChatViewController;
use crate::gui::error::GuiError;
use crate::gui::widgets::report_dialog::show_report_dialog;
use crate::gui::{self, widgets::error_popup::ErrorPopup};
use crate::gui::{HIGHLIGHTS_TAB_NAME, SERVER_TAB_NAME};

use crate::gui::state::UIState;
use steel_core::{
    ipc::{server::AppMessageIn, ui::UIMessageIn},
    settings::{Settings, ThemeMode},
};

const UI_EVENT_INTAKE_PER_REFRESH: u32 = 100;

pub const NOTO_ARABIC: &str = "noto-arabic";
pub const NOTO_HEBREW: &str = "noto-hebrew";
pub const NOTO_JAPANESE: &str = "noto-japanese";
pub const NOTO_CHINESE_SIMPLIFIED: &str = "noto-chinese-simplified";
pub const NOTO_CHINESE_TRADITIONAL: &str = "noto-chinese-traditional";
pub const NOTO_KOREAN: &str = "noto-korean";
pub const NOTO_THAI: &str = "noto-thai";
pub const NOTO_REGULAR: &str = "noto-regular";

// Courtesy of emilk @ https://github.com/emilk/egui/blob/master/examples/custom_font/src/main.rs
fn add_font(fonts: &mut egui::FontDefinitions, name: &str, payload: &'static [u8]) {
    // Install my own font (maybe supporting non-latin characters).
    // .ttf and .otf files supported.
    fonts
        .font_data
        .insert(name.to_owned(), egui::FontData::from_static(payload).into());
    // Put my font first (highest priority) for proportional text:
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, name.to_owned());

    // Put my font as last fallback for monospace:
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push(name.to_owned());
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    add_font(
        &mut fonts,
        NOTO_ARABIC,
        include_bytes!("../../media/fonts/noto-arabic/NotoNaskhArabic-VariableFont_wght.ttf"),
    );
    add_font(
        &mut fonts,
        NOTO_HEBREW,
        include_bytes!("../../media/fonts/noto-hebrew/NotoSansHebrew-VariableFont_wdth,wght.ttf"),
    );
    add_font(
        &mut fonts,
        NOTO_JAPANESE,
        include_bytes!("../../media/fonts/noto-japanese/NotoSansJP-Regular.otf"),
    );
    add_font(
        &mut fonts,
        NOTO_CHINESE_SIMPLIFIED,
        include_bytes!("../../media/fonts/noto-chinese-simplified/NotoSansSC-Regular.otf"),
    );
    add_font(
        &mut fonts,
        NOTO_CHINESE_TRADITIONAL,
        include_bytes!("../../media/fonts/noto-chinese-traditional/NotoSansTC-Regular.otf"),
    );
    add_font(
        &mut fonts,
        NOTO_KOREAN,
        include_bytes!("../../media/fonts/noto-korean/NotoSansKR-Regular.otf"),
    );
    add_font(
        &mut fonts,
        NOTO_THAI,
        include_bytes!("../../media/fonts/noto-thai/NotoSansThai-VariableFont_wdth,wght.ttf"),
    );
    add_font(
        &mut fonts,
        NOTO_REGULAR,
        include_bytes!("../../media/fonts/noto-regular/NotoSans-Regular.ttf"),
    );
    ctx.set_fonts(fonts);
}

fn set_startup_ui_settings(ctx: &egui::Context, settings: &Settings) {
    setup_custom_fonts(ctx);

    ctx.style_mut(|style| {
        style.url_in_tooltip = true;
    });

    if settings.application.window.maximized {
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
    } else {
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::Pos2 {
            x: settings.application.window.x as f32,
            y: settings.application.window.y as f32,
        }));

        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::Vec2 {
            x: settings.application.window.width as f32,
            y: settings.application.window.height as f32,
        }));
    }

    update_ui_settings(ctx, settings);
}

fn update_ui_settings(ctx: &egui::Context, settings: &Settings) {
    ctx.set_pixels_per_point(settings.ui.scaling);
    ctx.set_theme(match settings.ui.theme {
        ThemeMode::Dark => Theme::Dark,
        ThemeMode::Light => Theme::Light,
    });
    ctx.style_mut(|style| {
        style.url_in_tooltip = true;
    });
}

pub struct ApplicationWindow {
    menu: gui::menu::Menu,
    chat_view_controller: ChatViewController,
    chat_tabs: gui::chat_tabs::ChatTabs,
    settings: gui::settings::SettingsWindow,
    about: gui::about::About,
    update_window: gui::update_window::UpdateWindow,
    usage_window: gui::usage::UsageWindow,

    ui_queue: UnboundedReceiver<UIMessageIn>,
    s: UIState,

    error_popup: gui::widgets::error_popup::ErrorPopup,
}

impl ApplicationWindow {
    pub fn new(
        cc: &eframe::CreationContext,
        ui_queue: UnboundedReceiver<UIMessageIn>,
        app_queue_handle: UnboundedSender<AppMessageIn>,
        initial_settings: Settings,
        original_exe_path: Option<std::path::PathBuf>,
    ) -> Self {
        set_startup_ui_settings(&cc.egui_ctx, &initial_settings);

        let mut window = Self {
            menu: gui::menu::Menu::new(),
            chat_view_controller: ChatViewController::default(),
            chat_tabs: gui::chat_tabs::ChatTabs::default(),
            settings: gui::settings::SettingsWindow::new(),
            about: gui::about::About::default(),
            update_window: gui::update_window::UpdateWindow::default(),
            usage_window: gui::usage::UsageWindow::default(),
            ui_queue,
            s: UIState::new(
                app_queue_handle.clone(),
                initial_settings.clone(),
                original_exe_path,
            ),
            error_popup: ErrorPopup::new(CoreClient::new(app_queue_handle)),
        };

        window.add_chat_to_controller(SERVER_TAB_NAME, false);
        window.add_chat_to_controller(HIGHLIGHTS_TAB_NAME, false);

        window
    }

    fn add_chat_to_controller(&mut self, target: &str, switch: bool) {
        if let Some(_chat) = self.s.add_new_chat(target.to_owned(), switch) {
            self.chat_view_controller.add(target.to_lowercase());
        }
    }

    fn refresh_window_title(&self, ctx: &egui::Context) {
        let new_tab_title = match self.s.active_chat_tab_name.starts_with('$') {
            true => format!("steel v{}", crate::VERSION),
            false => {
                if let Some(chat) = self.s.active_chat() {
                    format!("{} – steel v{}", chat.name, crate::VERSION)
                } else {
                    format!("steel v{}", crate::VERSION)
                }
            }
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(new_tab_title));
    }

    fn refresh_window_geometry_settings(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            let ppi = i.viewport().native_pixels_per_point.unwrap_or(1.);

            self.s.settings.application.window.maximized = i.viewport().maximized.unwrap_or(false);

            if let Some(outer_rect) = i.viewport().outer_rect {
                self.s.settings.application.window.x = (outer_rect.left_top().x / ppi) as i32;
                self.s.settings.application.window.y = (outer_rect.left_top().y / ppi) as i32;
            }

            if let Some(inner_rect) = i.viewport().inner_rect {
                self.s.settings.application.window.width = (inner_rect.width() / ppi) as i32;
                self.s.settings.application.window.height = (inner_rect.height() / ppi) as i32;
            }
        });
    }

    pub fn process_pending_events(&mut self, ctx: &egui::Context) {
        // If the main window is restored after having being minimized for some time, it still needs to be responsive
        // enough.
        let mut i = 0;
        while let Ok(event) = self.ui_queue.try_recv() {
            self.dispatch_event(event, ctx);
            i += 1;
            if i >= UI_EVENT_INTAKE_PER_REFRESH {
                break;
            }
        }
    }

    fn dispatch_event(&mut self, event: UIMessageIn, ctx: &egui::Context) {
        match event {
            UIMessageIn::NewSystemMessage { target, message } => {
                self.s.push_chat_message(&target, message, ctx);
            }

            UIMessageIn::SettingsChanged(settings) => {
                update_ui_settings(ctx, &settings);
                self.s.update_settings(&settings);
            }

            UIMessageIn::ConnectionStatusChanged(conn) => {
                self.s.connection = conn;
                match conn {
                    steel_core::chat::ConnectionStatus::Connected => {
                        self.s.connection_indicator.connect(
                            self.s.settings.chat.irc.server.clone(),
                            self.s.settings.chat.irc.ping_timeout,
                        );
                    }
                    steel_core::chat::ConnectionStatus::Disconnected { .. } => {
                        self.s.connection_indicator.disconnect();
                    }
                    _ => {}
                }
            }

            UIMessageIn::ConnectionActivity => {
                self.s.connection_indicator.refresh();
            }

            UIMessageIn::NewChatRequested { target, switch } => {
                self.add_chat_to_controller(&target, switch);
                if switch {
                    self.refresh_window_title(ctx);
                }
            }

            UIMessageIn::NewChatStateReceived { target, state } => {
                self.s.set_chat_state(&target, state);
            }

            UIMessageIn::ChatSwitchRequested(name, message_id) => {
                let lowercase_name = name.to_lowercase();

                if let Some(mid) = message_id {
                    if !self.s.validate_reference(&lowercase_name, mid) {
                        self.error_popup
                            .push_error(Box::new(GuiError::message_not_found(mid, name)), false);
                    }
                }

                if let Some(chat) = self.s.find_chat_mut(&lowercase_name) {
                    chat.mark_as_read();
                    self.s.active_chat_tab_name = lowercase_name;

                    if let Some(message_id) = message_id {
                        self.chat_view_controller.scroll_chat_to(
                            &self.s,
                            &self.s.active_chat_tab_name,
                            message_id,
                        );
                    }
                }
                self.refresh_window_title(ctx);
            }

            UIMessageIn::NewMessageReceived { target, message } => {
                self.s.push_chat_message(&target, message.clone(), ctx);

                #[cfg(feature = "glass")]
                match message.username == self.s.settings.chat.irc.username {
                    false => self
                        .s
                        .glass
                        .handle_incoming_message(&self.s.core, &target, &message),
                    true => self
                        .s
                        .glass
                        .handle_outgoing_message(&self.s.core, &target, &message),
                }

                ctx.request_repaint();
            }

            UIMessageIn::NewServerMessageReceived(text) => {
                let message = Message::new_system(&text);
                self.s.push_chat_message(SERVER_TAB_NAME, message, ctx);
                ctx.request_repaint();
            }

            UIMessageIn::ChatClosed(name) => {
                self.chat_view_controller.remove(&name);
                self.s.remove_chat(name);
                self.refresh_window_title(ctx);
            }

            UIMessageIn::ChatCleared(name) => {
                self.s.clear_chat(&name);
            }

            UIMessageIn::ChatModeratorAdded(name) => {
                for mods in [
                    &mut self.s.settings.ui.dark_colours.mod_users,
                    &mut self.s.settings.ui.light_colours.mod_users,
                ] {
                    mods.insert(name.to_lowercase().replace(' ', "_"));
                }
            }

            UIMessageIn::WindowTitleRefreshRequested => {
                self.refresh_window_title(ctx);
            }

            UIMessageIn::UIUserMentionRequested(username) => {
                self.chat_view_controller
                    .insert_user_mention(ctx, &self.s, username);
            }

            UIMessageIn::UsageWindowRequested => {
                self.menu.show_usage = true;
            }

            UIMessageIn::ChatFilterRequested => {
                self.chat_view_controller.enable_filter(&self.s);
            }

            UIMessageIn::UpdateStateChanged(state) => {
                self.s.update_state = state;
            }

            UIMessageIn::BackendError { error, is_fatal } => {
                self.error_popup.push_error(error, is_fatal);
            }

            #[allow(unused_variables)] // glass
            UIMessageIn::GlassSettingsChanged { settings_data_yaml } => {
                #[cfg(feature = "glass")]
                {
                    if let Ok(glass_settings) =
                        serde_yaml::from_str::<glass::config::GlassSettings>(&settings_data_yaml)
                    {
                        self.s.update_glass_settings(glass_settings);
                    }
                }
            }

            UIMessageIn::ReportDialogRequested {
                username,
                chat_name,
            } => {
                self.s.report_dialog = Some(crate::gui::state::ReportDialogState {
                    username,
                    chat_name,
                    reason: String::new(),
                });
            }
        }
    }
}

const MIN_IDLE_FRAME_TIME: std::time::Duration = std::time::Duration::from_millis(200);

impl eframe::App for ApplicationWindow {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.request_repaint_after(MIN_IDLE_FRAME_TIME);
        self.process_pending_events(ctx);

        // Check if flash timeout has elapsed
        self.s.check_flash_timeout(ctx);

        self.error_popup.show(ctx);

        self.usage_window
            .show(ctx, &mut self.s, &mut self.menu.show_usage);
        self.settings
            .show(ctx, &mut self.s, &mut self.menu.show_settings);

        self.about.show(ctx, &mut self.s, &mut self.menu.show_about);

        self.update_window
            .show(ctx, &mut self.s, &mut self.menu.show_update);

        let active_chat_name = self.s.active_chat_tab_name.clone();
        self.menu.show(
            ctx,
            frame,
            &mut self.s,
            &mut self
                .chat_view_controller
                .response_widget_id(&active_chat_name),
        );

        self.chat_tabs.show(ctx, &mut self.s);

        // Return focus BEFORE showing chat view to prevent tiny chat and input flicker.
        if self.s.is_connected()
            && self.s.settings.chat.behaviour.keep_focus_on_input
            && !self.menu.dialogs_visible()
        {
            self.chat_view_controller
                .return_focus(ctx, &active_chat_name);
        }

        self.chat_view_controller.show(ctx, &self.s);

        show_report_dialog(ctx, &mut self.s);

        self.refresh_window_geometry_settings(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.s.core.exit_requested(Some(&self.s.settings), 0);
    }
}
