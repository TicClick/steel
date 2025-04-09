use eframe::egui::{self, Theme};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::gui;

use crate::gui::state::UIState;
use steel_core::{
    ipc::{server::AppMessageIn, ui::UIMessageIn},
    settings::{chat::ChatPosition, Settings, ThemeMode},
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

    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::Pos2 {
        x: settings.application.window.x as f32,
        y: settings.application.window.y as f32,
    }));

    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::Vec2 {
        x: settings.application.window.width as f32,
        y: settings.application.window.height as f32,
    }));

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
    chat: gui::chat::ChatWindow,
    chat_tabs: gui::chat_tabs::ChatTabs,
    settings: gui::settings::SettingsWindow,
    about: gui::about::About,
    update_window: gui::update_window::UpdateWindow,
    usage_window: gui::usage::UsageWindow,

    ui_queue: UnboundedReceiver<UIMessageIn>,
    s: UIState,
    filter_ui: gui::filter::FilterWindow,
}

impl ApplicationWindow {
    pub fn new(
        cc: &eframe::CreationContext,
        ui_queue: UnboundedReceiver<UIMessageIn>,
        app_queue_handle: UnboundedSender<AppMessageIn>,
        initial_settings: Settings,
    ) -> Self {
        let state = UIState::new(app_queue_handle, initial_settings.clone());
        set_startup_ui_settings(&cc.egui_ctx, &initial_settings);

        Self {
            menu: gui::menu::Menu::new(),
            chat: gui::chat::ChatWindow::new(),
            chat_tabs: gui::chat_tabs::ChatTabs::default(),
            settings: gui::settings::SettingsWindow::new(),
            about: gui::about::About::default(),
            update_window: gui::update_window::UpdateWindow::default(),
            usage_window: gui::usage::UsageWindow::default(),
            ui_queue,
            s: state,
            filter_ui: gui::filter::FilterWindow::default(),
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

            if let Some(rect) = i.viewport().outer_rect {
                self.s.settings.application.window.x = (rect.left_top().x / ppi) as i32;
                self.s.settings.application.window.y = (rect.left_top().y / ppi) as i32;

                self.s.settings.application.window.width = (rect.width() / ppi) as i32;
                self.s.settings.application.window.height = (rect.height() / ppi) as i32;
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
                self.s.push_chat_message(target, message, ctx);
            }

            UIMessageIn::SettingsChanged(settings) => {
                update_ui_settings(ctx, &settings);
                self.s.update_settings(&settings);
            }

            UIMessageIn::ConnectionStatusChanged(conn) => {
                self.s.connection = conn;
            }

            UIMessageIn::NewChatRequested { target, switch } => {
                self.s.add_new_chat(target, switch);
                if switch {
                    self.refresh_window_title(ctx);
                }
            }

            UIMessageIn::NewChatStateReceived { target, state } => {
                self.s.set_chat_state(&target, state);
            }

            UIMessageIn::ChatSwitchRequested(name, message_id) => {
                let lowercase_name = name.to_lowercase();
                self.s.read_tracker.mark_as_read(&lowercase_name);
                if self.s.has_chat(&name) {
                    // Update chat tracking in ReadTracker
                    let message_count = self.s.chat_message_count();
                    let previous_chat = self.s.active_chat_tab_name.clone();

                    // Update chat tracking and unread marker positions
                    self.s.read_tracker.update_chat_tracking(
                        &previous_chat,
                        &lowercase_name,
                        message_count,
                    );

                    self.s.active_chat_tab_name = lowercase_name;

                    if message_id.is_some() {
                        self.chat.scroll_to = match self.s.settings.chat.behaviour.chat_position {
                            ChatPosition::Bottom => Some(message_id.unwrap() + 1),
                            ChatPosition::Top => message_id,
                        };
                    }
                }
                self.refresh_window_title(ctx);
            }

            UIMessageIn::NewMessageReceived { target, message } => {
                let name_updated = self
                    .s
                    .push_chat_message(target.clone(), message.clone(), ctx);
                if name_updated {
                    self.refresh_window_title(ctx);
                }

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
                self.s.push_server_message(&text);
                ctx.request_repaint();
            }

            UIMessageIn::ChatClosed(name) => {
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

            UIMessageIn::UsageWindowRequested => {
                self.menu.show_usage = true;
            }

            UIMessageIn::UpdateStateChanged(state) => {
                self.s.update_state = state;
            }
        }
    }
}

const MIN_IDLE_FRAME_TIME: std::time::Duration = std::time::Duration::from_millis(200);

impl eframe::App for ApplicationWindow {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        puffin::set_scopes_on(true);
        puffin::GlobalProfiler::lock().new_frame();
        puffin_egui::profiler_window(ctx);

        ctx.request_repaint_after(MIN_IDLE_FRAME_TIME);
        self.process_pending_events(ctx);

        self.usage_window
            .show(ctx, &mut self.s, &mut self.menu.show_usage);
        self.settings
            .show(ctx, &mut self.s, &mut self.menu.show_settings);

        self.about.show(ctx, &mut self.s, &mut self.menu.show_about);

        self.update_window
            .show(ctx, &mut self.s, &mut self.menu.show_update);

        self.menu
            .show(ctx, frame, &mut self.s, &mut self.chat.response_widget_id);
        self.chat_tabs.show(ctx, &mut self.s);

        self.filter_ui.show(ctx, &mut self.s);

        self.chat.show(ctx, &self.s);

        if !self.menu.dialogs_visible() {
            self.chat.return_focus(ctx, &self.s);
        }

        self.refresh_window_geometry_settings(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.s.core.exit_requested(&self.s.settings);
    }
}
