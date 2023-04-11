use tokio::sync::mpsc::{Receiver, Sender};

use crate::{app::AppMessageIn, gui};
use eframe::egui;

use super::{UIMessageIn, UIState};
use crate::core::settings;

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
        "noto-arabic",
        include_bytes!("../../media/fonts/noto-arabic/NotoNaskhArabic-VariableFont_wght.ttf"),
    );
    add_font(
        &mut fonts,
        "noto-hebrew",
        include_bytes!("../../media/fonts/noto-hebrew/NotoSansHebrew-VariableFont_wdth,wght.ttf"),
    );
    add_font(
        &mut fonts,
        "noto-japanese",
        include_bytes!("../../media/fonts/noto-japanese/NotoSansJP-Regular.otf"),
    );
    add_font(
        &mut fonts,
        "noto-chinese-simplified",
        include_bytes!("../../media/fonts/noto-chinese-simplified/NotoSansSC-Regular.otf"),
    );
    add_font(
        &mut fonts,
        "noto-chinese-traditional",
        include_bytes!("../../media/fonts/noto-chinese-traditional/NotoSansTC-Regular.otf"),
    );
    add_font(
        &mut fonts,
        "noto-korean",
        include_bytes!("../../media/fonts/noto-korean/NotoSansKR-Regular.otf"),
    );
    add_font(
        &mut fonts,
        "noto-thai",
        include_bytes!("../../media/fonts/noto-thai/NotoSansThai-VariableFont_wdth,wght.ttf"),
    );
    add_font(
        &mut fonts,
        "noto-regular",
        include_bytes!("../../media/fonts/noto-regular/NotoSans-Regular.ttf"),
    );
    ctx.set_fonts(fonts);
}

pub struct ApplicationWindow {
    menu: gui::menu::Menu,
    chat: gui::chat::ChatWindow,
    chat_tabs: gui::chat_tabs::ChatTabs,
    settings: gui::settings::Settings,
    about: gui::about::About,

    ui_queue: Receiver<UIMessageIn>,
    s: UIState,
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
            settings: gui::settings::Settings::default(),
            about: gui::about::About::default(),
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

        self.about.show(ctx, &self.s, &mut self.menu.show_about);

        if !self.menu.dialogs_visible() {
            self.chat.return_focus(ctx);
        }
    }

    fn on_close_event(&mut self) -> bool {
        self.s
            .app_queue_handle
            .blocking_send(AppMessageIn::UIExitRequested)
            .unwrap();
        true
    }
}
