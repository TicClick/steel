#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use tokio::sync::mpsc::channel;

use steel::app;
use steel::gui;

const TITLE: &str = concat!("steel v", env!("CARGO_PKG_VERSION"));
const UI_EVENT_QUEUE_SIZE: usize = 1000;

fn main() {
    let (ui_queue_handle, ui_queue) = channel(UI_EVENT_QUEUE_SIZE);
    let mut app = app::server::Application::new(ui_queue_handle);

    let app_queue_handle = app.app_queue.clone();

    let app_thread = std::thread::spawn(move || {
        app.initialize();
        app.run();
    });

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        TITLE,
        native_options,
        Box::new(|cc| {
            Box::new(gui::window::ApplicationWindow::new(
                cc,
                ui_queue,
                app_queue_handle,
            ))
        }),
    )
    .expect("failed to set up the app window");
    app_thread.join().unwrap();
}
