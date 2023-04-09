use tokio::sync::mpsc::{Receiver, Sender};

use crate::{app::AppMessageIn, gui};
use eframe::egui;

use super::{UIMessageIn, UIState};
use crate::core::settings;

pub struct ApplicationWindow {
    menu: gui::menu::Menu,
    chat: gui::chat::ChatWindow,
    chat_tabs: gui::chat_tabs::ChatTabs,
    settings: gui::settings::Settings,

    ui_queue: Receiver<UIMessageIn>,
    s: UIState,
}

impl ApplicationWindow {
    pub fn new(
        _cc: &eframe::CreationContext,
        ui_queue: Receiver<UIMessageIn>,
        app_queue_handle: Sender<AppMessageIn>,
    ) -> Self {
        Self {
            menu: gui::menu::Menu::new(),
            chat: gui::chat::ChatWindow::new(),
            chat_tabs: gui::chat_tabs::ChatTabs::default(),
            settings: gui::settings::Settings::default(),
            ui_queue,
            s: UIState::new(app_queue_handle),
        }
    }

    pub fn process_pending_events(&mut self) {
        while let Ok(event) = self.ui_queue.try_recv() {
            match event {
                UIMessageIn::SettingsChanged(settings) => {
                    self.s.set_settings(settings);
                }
                UIMessageIn::ConnectionStatusChanged(conn) => {
                    self.s.connection = conn;
                }
                UIMessageIn::NewChatOpened(name) => {
                    self.s.add_new_chat(name);
                }
                UIMessageIn::NewMessageReceived { target, message } => {
                    self.s.push_chat_message(target, message);
                }
                UIMessageIn::NewServerMessageReceived(_) => {}
                UIMessageIn::ChatClosed(name) => {
                    self.s.remove_chat(name);
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(MIN_IDLE_FRAME_TIME);
        self.process_pending_events();
        self.set_theme(ctx);

        self.menu.show(ctx, &mut self.s);
        self.chat_tabs.show(ctx, &mut self.s);
        self.chat.show(ctx, &self.s);

        self.settings
            .show(ctx, &mut self.s, &mut self.menu.show_settings);
        self.chat.return_focus(ctx);
    }

    fn on_close_event(&mut self) -> bool {
        self.s
            .app_queue_handle
            .blocking_send(AppMessageIn::UIExitRequested)
            .unwrap();
        true
    }
}
