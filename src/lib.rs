use steel_core::ipc::ui::UIMessageIn;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub mod actor;
pub mod app;
pub mod core;
pub mod gui;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const LOG_FILE_NAME: &str = "runtime.log";

pub fn setup_logging() {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE_NAME)
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

pub fn run_app(
    ui_queue_in: UnboundedSender<UIMessageIn>,
    ui_queue_out: UnboundedReceiver<UIMessageIn>,
) -> std::thread::JoinHandle<()> {
    let mut app = app::Application::new(ui_queue_in);
    app.initialize();

    let app_queue = app.app_queue.clone();
    let settings = app.current_settings().to_owned();

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
                ui_queue_out,
                app_queue,
                settings,
            )))
        }),
    )
    .expect("failed to set up the app window");

    app_thread
}
