#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use tokio::sync::mpsc::channel;

use steel::app;
use steel::gui;

const UI_EVENT_QUEUE_SIZE: usize = 1000;
const LOG_FILE_PATH: &str = "./runtime.log";

fn setup_logging() {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE_PATH)
        .expect("failed to open the file for logging app events");

    let time_format = simplelog::format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]"
    );
    simplelog::WriteLogger::init(
        simplelog::LevelFilter::Trace,
        simplelog::ConfigBuilder::new()
            .set_time_format_custom(time_format)
            .build(),
        file,
    )
    .expect("Failed to configure the logger");
    log_panics::init();
}

fn main() {
    setup_logging();

    let (ui_queue_handle, ui_queue) = channel(UI_EVENT_QUEUE_SIZE);
    let mut app = app::server::Application::new(ui_queue_handle);

    let app_queue_handle = app.app_queue.clone();

    let app_thread = std::thread::spawn(move || {
        app.initialize();
        app.run();
    });

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "steel",
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
