use std::sync::{Arc, OnceLock};

use eframe::egui;
use parking_lot::RwLock;
use steel_core::settings::application::DetachedWindowGeometry;

use crate::gui::chat::chat_controller::ChatViewController;
use crate::gui::state::UIState;

pub fn viewport_id(chat_key: &str) -> egui::ViewportId {
    egui::ViewportId::from_hash_of(("detached-chat", chat_key))
}

fn app_icon() -> Arc<egui::IconData> {
    static APP_ICON: OnceLock<Arc<egui::IconData>> = OnceLock::new();
    APP_ICON
        .get_or_init(|| {
            Arc::new(
                eframe::icon_data::from_png_bytes(
                    &include_bytes!("../../../media/icons/logo.png")[..],
                )
                .expect("failed to decode the app icon"),
            )
        })
        .clone()
}

fn record_geometry(ctx: &egui::Context, state: &mut UIState, chat_key: &str) {
    ctx.input(|i| {
        let Some(inner_rect) = i.viewport().inner_rect else {
            return;
        };
        let width = inner_rect.width() as i32;
        let height = inner_rect.height() as i32;

        // skip when the viewport is transitioning -- wgpu (Metal) can report 0x0
        if width <= 0 || height <= 0 {
            return;
        }

        let geometry = state
            .settings
            .application
            .detached_chat_windows
            .entry(chat_key.to_owned())
            .or_default();

        if let Some(outer_rect) = i.viewport().outer_rect {
            geometry.x = Some(outer_rect.left_top().x as i32);
            geometry.y = Some(outer_rect.left_top().y as i32);
        }

        geometry.width = width;
        geometry.height = height;
    });
}

fn initial_geometry(
    builder: egui::ViewportBuilder,
    geometry: Option<&DetachedWindowGeometry>,
) -> egui::ViewportBuilder {
    let geometry = geometry.cloned().unwrap_or_default();
    let size = match geometry.width > 0 && geometry.height > 0 {
        true => egui::vec2(geometry.width as f32, geometry.height as f32),
        false => egui::vec2(600.0, 400.0),
    };
    let mut builder = builder.with_inner_size(size);
    if let (Some(x), Some(y)) = (geometry.x, geometry.y) {
        builder = builder.with_position(egui::pos2(x as f32, y as f32));
    }
    builder
}

/// The caller must NOT hold a lock on `shared`: if egui falls back to embedded viewports,
/// the callbacks run inline and take the lock themselves.
pub fn show_detached_chat_windows(
    ctx: &egui::Context,
    shared: &Arc<RwLock<UIState>>,
    controller: &ChatViewController,
) {
    let detached: Vec<(String, String, Option<DetachedWindowGeometry>)> = {
        let s = shared.read();
        s.detached_chats
            .iter()
            .filter_map(|key| {
                s.find_chat(key).map(|chat| {
                    (
                        key.clone(),
                        chat.name.clone(),
                        s.settings
                            .application
                            .detached_chat_windows
                            .get(key)
                            .cloned(),
                    )
                })
            })
            .collect()
    };

    for (chat_key, display_name, geometry) in detached {
        let Some(view) = controller.view_handle(&chat_key) else {
            continue;
        };
        let shared = Arc::clone(shared);
        let id = viewport_id(&chat_key);

        let mut builder = egui::ViewportBuilder::default()
            .with_title(format!("{display_name} – steel v{}", crate::VERSION))
            .with_icon(app_icon());
        let window_exists = ctx.input(|i| i.raw.viewports.contains_key(&id));
        if !window_exists {
            builder = initial_geometry(builder, geometry.as_ref());
        }

        ctx.show_viewport_deferred(id, builder, move |ui, _class| {
            let ctx = ui.ctx().clone();
            let mut state = shared.write();

            let focused = ctx.input(|i| i.viewport().focused.unwrap_or(false));
            if focused {
                if let Some(chat) = state.find_chat_mut(&chat_key) {
                    chat.mark_as_read();
                }
            }

            view.lock().show(ui, &state);

            record_geometry(&ctx, &mut state, &chat_key);

            if ctx.input(|i| i.viewport().close_requested()) {
                state.reattach_chat(&chat_key);
                // Wake the root window so it stops declaring this viewport.
                ctx.request_repaint_of(egui::ViewportId::ROOT);
            }
        });
    }
}
