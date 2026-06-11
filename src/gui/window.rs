use std::sync::Arc;

use chrono::Datelike;
use eframe::egui::{self, Theme};
use parking_lot::RwLock;
#[cfg(feature = "puffin")]
use puffin;
#[cfg(feature = "puffin")]
use puffin_egui;
use steel_core::chat::Message;
use steel_core::ipc::client::CoreClient;
use steel_core::string_utils::{UsernameKey, UsernameString};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::gui::chat::chat_controller::ChatViewController;
use crate::gui::chat::detached;
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

    ctx.global_style_mut(|style| {
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
    ctx.global_style_mut(|style| {
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
    s: Arc<RwLock<UIState>>,

    error_popup: gui::widgets::error_popup::ErrorPopup,

    #[cfg(feature = "puffin")]
    auto_profiler: gui::auto_profiler::AutoProfiler,
    #[cfg(feature = "puffin")]
    profile_output: Option<std::path::PathBuf>,
}

impl ApplicationWindow {
    pub fn new(
        cc: &eframe::CreationContext,
        ui_queue: UnboundedReceiver<UIMessageIn>,
        app_queue_handle: UnboundedSender<AppMessageIn>,
        initial_settings: Settings,
        original_exe_path: Option<std::path::PathBuf>,
        #[cfg(feature = "puffin")] profile_output: Option<std::path::PathBuf>,
    ) -> Self {
        #[cfg(feature = "puffin")]
        puffin::set_scopes_on(true);

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
            s: Arc::new(RwLock::new(UIState::new(
                app_queue_handle.clone(),
                initial_settings.clone(),
                original_exe_path,
            ))),
            error_popup: ErrorPopup::new(CoreClient::new(app_queue_handle)),
            #[cfg(feature = "puffin")]
            auto_profiler: gui::auto_profiler::AutoProfiler::new(),
            #[cfg(feature = "puffin")]
            profile_output,
        };

        let shared = Arc::clone(&window.s);
        let mut s = shared.write();
        window.add_chat_to_controller(&mut s, SERVER_TAB_NAME, false);
        window.add_chat_to_controller(&mut s, HIGHLIGHTS_TAB_NAME, false);
        drop(s);

        window
    }

    fn add_chat_to_controller(&mut self, s: &mut UIState, target: &str, switch: bool) {
        if let Some(_chat) = s.add_new_chat(target.to_owned(), switch) {
            self.chat_view_controller.add(target.normalize());
        }
    }

    fn refresh_window_title(&self, ctx: &egui::Context, s: &UIState) {
        let new_tab_title = match s.active_chat_tab_name.starts_with('$') {
            true => format!("steel v{}", crate::VERSION),
            false => {
                if let Some(chat) = s.active_chat() {
                    format!("{} – steel v{}", chat.name, crate::VERSION)
                } else {
                    format!("steel v{}", crate::VERSION)
                }
            }
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(new_tab_title));
    }

    fn refresh_window_geometry_settings(&self, ctx: &egui::Context, s: &mut UIState) {
        ctx.input(|i| {
            let ppi = i.viewport().native_pixels_per_point.unwrap_or(1.);

            s.settings.application.window.maximized = i.viewport().maximized.unwrap_or(false);

            if let Some(outer_rect) = i.viewport().outer_rect {
                s.settings.application.window.x = (outer_rect.left_top().x / ppi) as i32;
                s.settings.application.window.y = (outer_rect.left_top().y / ppi) as i32;
            }

            if let Some(inner_rect) = i.viewport().inner_rect {
                s.settings.application.window.width = (inner_rect.width() / ppi) as i32;
                s.settings.application.window.height = (inner_rect.height() / ppi) as i32;
            }
        });
    }

    pub fn process_pending_events(&mut self, ctx: &egui::Context, s: &mut UIState) {
        // If the main window is restored after having being minimized for some time, it still needs to be responsive
        // enough.
        let mut i = 0;
        while let Ok(event) = self.ui_queue.try_recv() {
            self.dispatch_event(event, ctx, s);
            i += 1;
            if i >= UI_EVENT_INTAKE_PER_REFRESH {
                break;
            }
        }
    }

    fn dispatch_event(&mut self, event: UIMessageIn, ctx: &egui::Context, s: &mut UIState) {
        match event {
            UIMessageIn::NewSystemMessage { target, message } => {
                s.push_chat_message(&target, message, ctx);
                let normalized = target.normalize();
                if s.is_detached(&normalized) {
                    ctx.request_repaint_of(detached::viewport_id(&normalized));
                }
            }

            UIMessageIn::SettingsChanged(settings) => {
                update_ui_settings(ctx, &settings);
                s.update_settings(&settings);
                s.repaint_detached_windows(ctx);
            }

            UIMessageIn::SettingsPatched(patch) => {
                s.apply_settings_patch(patch);
            }

            UIMessageIn::ConnectionStatusChanged(conn) => {
                s.connection = conn;
                match conn {
                    steel_core::chat::ConnectionStatus::Connected => {
                        s.connection_indicator.connect();
                    }
                    steel_core::chat::ConnectionStatus::Disconnected { .. } => {
                        s.connection_indicator.disconnect();
                    }
                    _ => {}
                }
            }

            UIMessageIn::ConnectionDetailsChanged(details) => {
                s.connection_indicator.update_details(details);
            }

            UIMessageIn::ConnectionActivity => {
                s.connection_indicator.refresh();
            }

            UIMessageIn::NewChatRequested { target, switch } => {
                self.add_chat_to_controller(s, &target, switch);
                let normalized = target.normalize();
                if s.settings
                    .application
                    .detached_chat_windows
                    .contains_key(&normalized)
                {
                    s.detach_chat(&normalized);
                }
                if switch {
                    self.refresh_window_title(ctx, s);
                }
            }

            UIMessageIn::NewChatStateReceived { target, state } => {
                s.set_chat_state(&target, state);
            }

            UIMessageIn::ChatSwitchRequested(name, message_id) => {
                let lowercase_name = name.to_lowercase();

                if let Some(mid) = message_id {
                    if !s.validate_reference(&lowercase_name, mid) {
                        self.error_popup
                            .push_error(Box::new(GuiError::message_not_found(mid, name)), false);
                    }
                }

                if s.is_detached(&lowercase_name) {
                    if let Some(chat) = s.find_chat_mut(&lowercase_name) {
                        chat.mark_as_read();
                    }
                    if let Some(message_id) = message_id {
                        self.chat_view_controller
                            .scroll_chat_to(s, &lowercase_name, message_id);
                    }
                    ctx.send_viewport_cmd_to(
                        detached::viewport_id(&lowercase_name),
                        egui::ViewportCommand::Focus,
                    );
                } else if let Some(chat) = s.find_chat_mut(&lowercase_name) {
                    chat.mark_as_read();
                    s.active_chat_tab_name = lowercase_name;

                    if let Some(message_id) = message_id {
                        let active_chat_tab_name = s.active_chat_tab_name.clone();
                        self.chat_view_controller.scroll_chat_to(
                            s,
                            &active_chat_tab_name,
                            message_id,
                        );
                    }
                }
                self.refresh_window_title(ctx, s);
            }

            UIMessageIn::NewMessageReceived { target, message } => {
                s.push_chat_message(&target, message.clone(), ctx);

                #[cfg(feature = "glass")]
                {
                    let own_username = s
                        .own_username
                        .as_deref()
                        .unwrap_or(&s.settings.chat.irc.username)
                        .to_owned();
                    match message.username.is_same_username(&own_username) {
                        false => s.glass.handle_incoming_message(&s.core, &target, &message),
                        true => s.glass.handle_outgoing_message(&s.core, &target, &message),
                    }
                }

                let normalized = target.normalize();
                if s.is_detached(&normalized) {
                    ctx.request_repaint_of(detached::viewport_id(&normalized));
                }
                ctx.request_repaint();
            }

            UIMessageIn::NewServerMessageReceived(text) => {
                let message = Message::new_system(&text);
                s.push_chat_message(SERVER_TAB_NAME, message, ctx);
                ctx.request_repaint();
            }

            UIMessageIn::ChatClosed(name) => {
                self.chat_view_controller.remove(&name);
                s.remove_chat(name);
                self.refresh_window_title(ctx, s);
            }

            UIMessageIn::ChatCleared(name) => {
                s.clear_chat(&name);
            }

            UIMessageIn::ChatModeratorAdded(name) => {
                for mods in [
                    &mut s.settings.ui.dark_colours.mod_users,
                    &mut s.settings.ui.light_colours.mod_users,
                ] {
                    mods.insert(UsernameKey::new(&name));
                }
            }

            UIMessageIn::WindowTitleRefreshRequested => {
                self.refresh_window_title(ctx, s);
            }

            UIMessageIn::UIUserMentionRequested(username) => {
                self.chat_view_controller
                    .insert_user_mention(ctx, s, username);
            }

            UIMessageIn::UsageWindowRequested => {
                self.menu.show_usage = true;
            }

            UIMessageIn::ChatFilterRequested => {
                self.chat_view_controller.enable_filter(s);
            }

            UIMessageIn::UpdateStateChanged(state) => {
                s.update_state = state;
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
                        s.update_glass_settings(glass_settings);
                    }
                }
            }

            UIMessageIn::ReportDialogRequested {
                username,
                chat_name,
            } => {
                s.report_dialog = Some(crate::gui::state::ReportDialogState {
                    username,
                    chat_name,
                    reason: String::new(),
                    just_opened: true,
                });
            }

            UIMessageIn::OwnUsernameChanged(username) => {
                s.own_username = Some(username);
            }

            UIMessageIn::Shutdown => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

const MIN_IDLE_FRAME_TIME: std::time::Duration = std::time::Duration::from_millis(200);

impl eframe::App for ApplicationWindow {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame) {
        #[cfg(feature = "puffin")]
        {
            puffin::profile_function!();
            puffin::GlobalProfiler::lock().new_frame();
        }

        let ctx = &ui.ctx().clone();
        ctx.request_repaint_after(MIN_IDLE_FRAME_TIME);

        let shared = Arc::clone(&self.s);
        {
            let s = &mut *shared.write();
            self.process_pending_events(ctx, s);

            s.update_window_attention(ctx);

            self.error_popup.show(ctx);

            self.usage_window.show(ctx, s, &mut self.menu.show_usage);
            self.settings.show(ctx, s, &mut self.menu.show_settings);

            self.about.show(ctx, s, &mut self.menu.show_about);

            self.update_window.show(ctx, s, &mut self.menu.show_update);

            let active_chat_name = s.active_chat_tab_name.clone();
            self.menu.show(
                ui,
                frame,
                s,
                &mut self
                    .chat_view_controller
                    .response_widget_id(&active_chat_name),
            );

            self.chat_tabs.show(ui, s);

            // Return focus BEFORE showing chat view to prevent tiny chat and input flicker.
            let root_focused = ctx.input(|i| i.viewport().focused.unwrap_or(false));
            if root_focused
                && s.is_connected()
                && s.settings.chat.behaviour.keep_focus_on_input
                && !self.menu.dialogs_visible()
                && s.report_dialog.is_none()
            {
                self.chat_view_controller
                    .return_focus(ctx, &active_chat_name);
            }

            self.chat_view_controller.show(ui, s);

            show_report_dialog(ctx, s);

            #[cfg(feature = "puffin")]
            puffin_egui::profiler_window(ctx);

            self.refresh_window_geometry_settings(ctx, s);

            let today = chrono::Local::now().date_naive();
            if today.day() == 1 && today.month() == 4 {
                let colour = match s.settings.ui.theme {
                    ThemeMode::Dark => egui::Color32::from_white_alpha(200),
                    ThemeMode::Light => egui::Color32::from_black_alpha(200),
                };
                egui_snow::Snow::new("snow_effect")
                    .color(colour)
                    .speed(40.0..=100.0)
                    .size(0.5..=3.0)
                    .show(ctx);
            }
        } // the guard must be dropped before viewport callbacks can run

        detached::show_detached_chat_windows(ctx, &self.s, &self.chat_view_controller);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        #[cfg(feature = "puffin")]
        if let Some(ref path) = self.profile_output {
            self.auto_profiler.save(path);
        }

        let s = self.s.read();
        s.core.exit_requested(Some(&s.settings), 0);
    }
}
