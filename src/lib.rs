use steel_core::{ipc::ui::UIMessageIn, settings::Settings};
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
            .add_filter_ignore_str("rustls")
            .add_filter_ignore_str("eframe")
            .add_filter_ignore_str("egui_glow")
            .add_filter_ignore_str("egui_wgpu")
            .add_filter_ignore_str("wgpu") // also covers wgpu_core and wgpu_hal
            .add_filter_ignore_str("naga")
            .add_filter_ignore_str("ureq")
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

fn select_renderer(setting: steel_core::settings::Renderer) -> eframe::Renderer {
    use steel_core::settings::Renderer as RendererSetting;

    match setting {
        RendererSetting::Glow => {
            log::info!("renderer: glow (forced by settings)");
            eframe::Renderer::Glow
        }
        RendererSetting::Auto | RendererSetting::Wgpu => match probe_wgpu_adapter() {
            Some(adapter) => {
                log::info!("renderer: wgpu ({adapter})");
                eframe::Renderer::Wgpu
            }
            None => {
                log::warn!("renderer: no usable wgpu adapter found, falling back to glow");
                eframe::Renderer::Glow
            }
        },
    }
}

// Probe the available adapter using logic from egui_wgpu::WgpuSetupCreateNew
fn probe_wgpu_adapter() -> Option<String> {
    use eframe::wgpu;

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::from_env()
            .unwrap_or(wgpu::Backends::PRIMARY | wgpu::Backends::GL),
        flags: wgpu::InstanceFlags::from_build_config().with_env(),
        backend_options: wgpu::BackendOptions::from_env_or_default(),
        memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        display: None,
    });
    let request = instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::from_env()
            .unwrap_or(wgpu::PowerPreference::HighPerformance),
        ..Default::default()
    });
    match futures::executor::block_on(request) {
        Ok(adapter) => {
            let info = adapter.get_info();
            Some(format!("{}, {:?} backend", info.name, info.backend))
        }
        Err(e) => {
            log::warn!("wgpu adapter probe failed: {e}");
            None
        }
    }
}

pub fn run_app(
    ui_queue_in: UnboundedSender<UIMessageIn>,
    ui_queue_out: UnboundedReceiver<UIMessageIn>,
    original_exe_path: Option<std::path::PathBuf>,
    #[cfg(feature = "puffin")] profile_output: Option<std::path::PathBuf>,
) -> std::thread::JoinHandle<()> {
    let mut app = app::Application::new(ui_queue_in);
    let settings = match app.initialize() {
        Ok(()) => app.current_settings().to_owned(),
        Err(e) => {
            app.ui_push_backend_error(Box::new(e), true);
            Settings::default()
        }
    };

    #[cfg(feature = "glass")]
    app.ui_handle_glass_settings_requested();

    let app_queue = app.app_queue.clone();
    let app_thread = std::thread::spawn(move || {
        app.run();
    });

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_icon(std::sync::Arc::new(
            eframe::icon_data::from_png_bytes(&include_bytes!("../media/icons/logo.png")[..])
                .unwrap(),
        )),
        renderer: select_renderer(settings.application.renderer),
        ..Default::default()
    };
    eframe::run_native(
        &format!("steel v{VERSION}"),
        native_options,
        Box::new(|cc| {
            Ok(Box::new(gui::window::ApplicationWindow::new(
                cc,
                ui_queue_out,
                app_queue,
                settings,
                original_exe_path,
                #[cfg(feature = "puffin")]
                profile_output,
            )))
        }),
    )
    .expect("failed to set up the app window");

    app_thread
}
