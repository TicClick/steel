#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use steel::core;
use steel::run_app;
use steel::setup_logging;
use tokio::sync::mpsc::unbounded_channel;

fn main() {
    if let Err(e) = crate::core::os::fix_cwd() {
        panic!("Failed to set proper current working directory: {:?}", e);
    }
    setup_logging();

    let (ui_queue_in, ui_queue_out) = unbounded_channel();
    let app_thread = run_app(ui_queue_in, ui_queue_out);

    app_thread.join().unwrap();
}
