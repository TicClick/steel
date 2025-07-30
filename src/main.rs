#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use steel::core;
use steel::run_app;
use steel::setup_logging;
use tokio::sync::mpsc::unbounded_channel;

fn main() {
    // Save original executable path before any potential fs::rename operations -- it's not guaranteed to be preserved.
    let original_exe_path = std::env::current_exe().ok();

    if let Err(e) = crate::core::os::fix_cwd() {
        panic!("Failed to set proper current working directory: {:?}", e);
    }
    setup_logging();

    let (ui_queue_in, ui_queue_out) = unbounded_channel();
    let app_thread = run_app(ui_queue_in, ui_queue_out, original_exe_path);

    app_thread.join().unwrap();
}
