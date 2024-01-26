use chrono::{DurationRound, Timelike};
use eframe::egui;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::core::chat::ChatState;
use crate::gui;

use crate::gui::state::UIState;
use steel_core::chat::{ConnectionStatus, Message};
use steel_core::ipc::{server::AppMessageIn, ui::UIMessageIn};

use crate::core::settings;

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
        .insert(name.to_owned(), egui::FontData::from_static(payload));
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

struct DateAnnouncer {
    pub prev_event: Option<chrono::DateTime<chrono::Local>>,
    pub current_event: chrono::DateTime<chrono::Local>,
}

impl Default for DateAnnouncer {
    fn default() -> Self {
        let now = chrono::Local::now();
        Self {
            prev_event: None,
            current_event: now,
        }
    }
}

impl DateAnnouncer {
    fn should_announce(&self) -> bool {
        match self.prev_event {
            None => {
                self.current_event.hour() == 0
                    && self.current_event.minute() == 0
                    && self.current_event.second() == 0
            }
            Some(dt) => {
                dt.date_naive() < self.current_event.date_naive()
                    && self.current_event.hour() == 0
                    && self.current_event.minute() == 0
            }
        }
    }

    fn refresh(&mut self) {
        self.prev_event = Some(self.current_event);
        self.current_event = chrono::Local::now();
    }
}

pub struct ApplicationWindow {
    menu: gui::menu::Menu,
    chat: gui::chat::ChatWindow,
    chat_tabs: gui::chat_tabs::ChatTabs,
    settings: gui::settings::SettingsWindow,
    about: gui::about::About,
    update_window: gui::update_window::UpdateWindow,
    usage_window: gui::usage::UsageWindow,

    ui_queue: Receiver<UIMessageIn>,
    s: UIState,
    date_announcer: DateAnnouncer,
}

impl ApplicationWindow {
    pub fn new(
        cc: &eframe::CreationContext,
        ui_queue: Receiver<UIMessageIn>,
        app_queue_handle: Sender<AppMessageIn>,
    ) -> Self {
        setup_custom_fonts(&cc.egui_ctx);

        Self {
            menu: gui::menu::Menu::new(),
            chat: gui::chat::ChatWindow::new(),
            chat_tabs: gui::chat_tabs::ChatTabs::default(),
            settings: gui::settings::SettingsWindow::new(),
            about: gui::about::About::default(),
            update_window: gui::update_window::UpdateWindow::default(),
            usage_window: gui::usage::UsageWindow::default(),
            ui_queue,
            s: UIState::new(app_queue_handle),
            date_announcer: DateAnnouncer::default(),
        }
    }

    pub fn process_pending_events(&mut self, ctx: &egui::Context) {
        if self.date_announcer.should_announce() {
            let text = format!(
                "A new day is born ({})",
                self.date_announcer
                    .current_event
                    .date_naive()
                    .format(crate::core::DEFAULT_DATE_FORMAT)
            );
            self.s.push_to_all_chats(
                Message::new_system(&text).with_time(
                    self.date_announcer
                        .current_event
                        .duration_trunc(chrono::Duration::days(1))
                        .unwrap(),
                ),
            );
            ctx.request_repaint();
        }
        self.date_announcer.refresh();

        while let Ok(event) = self.ui_queue.try_recv() {
            match event {
                UIMessageIn::SettingsChanged(settings) => {
                    self.s.set_settings(ctx, settings);
                }

                UIMessageIn::ConnectionStatusChanged(conn) => {
                    self.s.connection = conn;
                    match conn {
                        ConnectionStatus::Disconnected { .. } => {
                            self.s.mark_all_as_disconnected();
                        }
                        ConnectionStatus::InProgress | ConnectionStatus::Scheduled(_) => (),
                        ConnectionStatus::Connected => {
                            self.s.mark_all_as_connected();
                        }
                    }
                }

                UIMessageIn::NewChatRequested(name, state, switch_to_chat) => {
                    if self.s.has_chat(&name) {
                        self.s.set_chat_state(&name, state, None);
                    } else {
                        self.s.add_new_chat(name, state, switch_to_chat);
                    }
                }

                UIMessageIn::NewChatStatusReceived {
                    target,
                    state,
                    details,
                } => {
                    if self.s.has_chat(&target) {
                        self.s.set_chat_state(&target, state, Some(&details));
                    }
                }

                UIMessageIn::ChatSwitchRequested(name, message_id) => {
                    if self.s.has_chat(&name) {
                        self.s.highlights.mark_as_read(&name);
                        self.s.active_chat_tab_name = name;
                        self.chat.scroll_to = Some(message_id);
                    }
                }

                UIMessageIn::ChannelJoined(name) => {
                    self.s.set_chat_state(
                        &name,
                        ChatState::Joined,
                        Some("You have joined the channel"),
                    );
                }

                UIMessageIn::NewMessageReceived { target, message } => {
                    self.s
                        .push_chat_message(target.clone(), message.clone(), ctx);

                    #[cfg(feature = "glass")]
                    match message.username == self.s.settings.chat.irc.username {
                        false => {
                            self.s
                                .glass
                                .handle_incoming_message(&self.s.core, &target, &message)
                        }
                        true => {
                            self.s
                                .glass
                                .handle_outgoing_message(&self.s.core, &target, &message)
                        }
                    }

                    ctx.request_repaint();
                }

                UIMessageIn::NewServerMessageReceived(text) => {
                    self.s.push_server_message(&text);
                    ctx.request_repaint();
                }

                UIMessageIn::ChatClosed(name) => {
                    self.s.remove_chat(name);
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
            }
        }
    }

    fn set_theme(&mut self, ctx: &egui::Context) {
        let theme = match self.s.settings.ui.theme {
            settings::ThemeMode::Dark => egui::Visuals::dark(),
            settings::ThemeMode::Light => egui::Visuals::light(),
        };
        ctx.set_visuals(theme);
    }
}

const MIN_IDLE_FRAME_TIME: std::time::Duration = std::time::Duration::from_millis(200);

impl eframe::App for ApplicationWindow {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.request_repaint_after(MIN_IDLE_FRAME_TIME);
        self.process_pending_events(ctx);

        if !self.s.active_chat_tab_name.is_empty() {
            let title = match self.s.active_chat_tab_name.starts_with('$') {
                true => format!("steel v{}", crate::VERSION),
                false => format!(
                    "{} – steel v{}",
                    self.s.active_chat_tab_name,
                    crate::VERSION
                ),
            };
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        }

        self.set_theme(ctx);

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
        self.chat.show(ctx, &self.s);

        if !self.menu.dialogs_visible() {
            self.chat.return_focus(ctx, &self.s);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.s.core.exit_requested();
    }
}
