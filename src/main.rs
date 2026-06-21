#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use steel::core;
use steel::run_app;
use steel::setup_logging;
use tokio::sync::mpsc::unbounded_channel;

fn main() {
    // Disable IME on Linux to avoid odd egui v0.34 behaviour where it reorders ligature sequences
    // (such as ff: differ -> diferf, etc). This also fixes Cyrillic input (and likely others).
    // Must be set up before winit is initialized by eframe::run_native.

    // TODO(TicClick): Remove this when it's fixed in the upstream.
    #[cfg(target_os = "linux")]
    {
        // SAFETY: called before any other threads are spawned.
        unsafe {
            std::env::set_var("XMODIFIERS", "@im=none");
            std::env::set_var("GTK_IM_MODULE", "none");
            std::env::set_var("QT_IM_MODULE", "none");
        }
    }

    // Save original executable path before any potential fs::rename operations -- it's not guaranteed to be preserved.
    let original_exe_path = std::env::current_exe().ok();

    if let Err(e) = crate::core::os::fix_cwd() {
        panic!("Failed to set proper current working directory: {e:?}");
    }
    setup_logging();

    let (ui_queue_in, ui_queue_out) = unbounded_channel();
    let app_thread = run_app(
        ui_queue_in,
        ui_queue_out,
        original_exe_path,
        steel::SharedUIContext::default(),
        #[cfg(feature = "puffin")]
        None,
    );

    app_thread.join().unwrap();
}
