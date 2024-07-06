#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use tokio::sync::mpsc::channel;

pub mod actor;
pub mod app;
pub mod core;
pub mod gui;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const UI_EVENT_QUEUE_SIZE: usize = 1000;
const LOG_FILE_PATH: &str = "./runtime.log";

fn setup_logging() {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE_PATH)
        .expect("failed to open the file for logging app events");

    let time_format =
        simplelog::format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]");
    simplelog::WriteLogger::init(
        simplelog::LevelFilter::Trace,
        simplelog::ConfigBuilder::new()
            .set_time_format_custom(time_format)
            .set_time_offset_to_local()
            // https://github.com/jhpratt/num_threads/issues/18 -- time = "0.3.34" compiled to x86_64-apple-darwin can't determine UTC offset on Apple silicon (aarch64-apple-darwin)
            .unwrap_or_else(|e| e)
            .build(),
        file,
    )
    .expect("Failed to configure the logger");
    log_panics::init();
}

fn main() {
    if let Err(e) = crate::core::os::fix_cwd() {
        panic!("Failed to set proper current working directory: {:?}", e);
    }
    setup_logging();

    let (ui_queue_handle, ui_queue) = channel(UI_EVENT_QUEUE_SIZE);
    let mut app = app::Application::new(ui_queue_handle);
    app.initialize();

    let app_queue_handle = app.app_queue.clone();

    let app_thread = std::thread::spawn(move || {
        app.run();
    });

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_icon(std::sync::Arc::new(
            eframe::icon_data::from_png_bytes(&include_bytes!("../media/icons/taskbar.png")[..])
                .unwrap(),
        )),
        ..Default::default()
    };
    eframe::run_native(
        &format!("steel v{}", VERSION),
        native_options,
        Box::new(|cc| {
            Ok(Box::new(gui::window::ApplicationWindow::new(
                cc,
                ui_queue,
                app_queue_handle,
            )))
        }),
    )
    .expect("failed to set up the app window");
    app_thread.join().unwrap();
}
