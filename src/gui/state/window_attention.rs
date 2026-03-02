use steel_core::settings::{NotificationStyle, Notifications};

use eframe::egui;

#[cfg(target_os = "macos")]
fn set_dock_badge(visible: bool) {
    use objc2_app_kit::NSApplication;
    use objc2_foundation::NSString;
    let label = if visible {
        Some(NSString::from_str("●"))
    } else {
        None
    };
    if let Some(th) = objc2::MainThreadMarker::new() {
        NSApplication::sharedApplication(th)
            .dockTile()
            .setBadgeLabel(label.as_deref());
    }
}

#[cfg(target_os = "macos")]
fn request_dock_attention() -> isize {
    use objc2_app_kit::{NSApplication, NSRequestUserAttentionType};
    unsafe {
        NSApplication::sharedApplication(objc2::MainThreadMarker::new_unchecked())
            .requestUserAttention(NSRequestUserAttentionType::CriticalRequest)
    }
}

#[cfg(target_os = "macos")]
fn cancel_dock_attention(token: isize) {
    use objc2_app_kit::NSApplication;
    unsafe {
        NSApplication::sharedApplication(objc2::MainThreadMarker::new_unchecked())
            .cancelUserAttentionRequest(token);
    }
}

#[derive(Debug, Default)]
pub struct WindowAttention {
    notification_start_time: Option<std::time::Instant>,
    #[cfg(target_os = "macos")]
    attention_token: Option<isize>,
    // Track false -> true focus transition to avoid acting on every frame.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    was_focused: bool,
}

impl WindowAttention {
    /// Called when an incoming message warrants a window attention request.
    #[cfg_attr(target_os = "macos", allow(unused_variables))]
    pub fn request(&mut self, ctx: &egui::Context, style: NotificationStyle) {
        #[cfg(target_os = "linux")]
        {
            // On X11/Wayland a single Informational ping is enough; the WM handles the rest.
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                eframe::egui::UserAttentionType::Informational,
            ));
        }

        #[cfg(target_os = "macos")]
        {
            // On MacOS, cancelling the "jumping icon" activity requires a token that is received when the attention is enqueued.
            self.attention_token = Some(request_dock_attention());
            match style {
                NotificationStyle::Moderate => {
                    if let Some(token) = self.attention_token.take() {
                        cancel_dock_attention(token);
                    }
                }
                NotificationStyle::Intensive => {
                    set_dock_badge(true);
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                eframe::egui::UserAttentionType::Critical,
            ));
            if matches!(style, NotificationStyle::Moderate) {
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                    eframe::egui::UserAttentionType::Informational,
                ));
            }
        }

        self.notification_start_time = Some(std::time::Instant::now());
    }

    /// Called every frame. Clear the attention request once the timeout elapses.
    #[cfg_attr(target_os = "macos", allow(unused_variables))]
    pub fn check_timeout(&mut self, ctx: &egui::Context, settings: &Notifications) {
        if !settings.enable_notification_timeout
            || !matches!(settings.notification_style, NotificationStyle::Intensive)
        {
            return;
        }

        if let Some(start_time) = self.notification_start_time {
            let elapsed = start_time.elapsed().as_secs();
            if elapsed >= settings.notification_timeout_seconds as u64 {
                #[cfg(target_os = "windows")]
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                    eframe::egui::UserAttentionType::Informational,
                ));

                #[cfg(target_os = "macos")]
                {
                    if let Some(token) = self.attention_token.take() {
                        cancel_dock_attention(token);
                    }
                }

                #[cfg(target_os = "linux")]
                {}

                self.notification_start_time = None;
            }
        }
    }

    /// Called every frame. Clear any pending attention request when the window gains focus.
    pub fn on_focus_changed(&mut self, ctx: &egui::Context) {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let is_focused = ctx.input(|i| i.viewport().focused.unwrap_or(false));
            if is_focused && !self.was_focused {
                #[cfg(target_os = "linux")]
                {
                    // On X11, WM_HINTS.urgent only triggers on false→true transition.
                    // Some WMs don't auto-clear it on focus. On Wayland, winit's
                    // attention_requested AtomicBool resets via compositor callback,
                    // but until it does new requests are dropped. Reset explicitly.
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                        eframe::egui::UserAttentionType::Reset,
                    ));
                }

                #[cfg(target_os = "macos")]
                {
                    if let Some(token) = self.attention_token.take() {
                        cancel_dock_attention(token);
                    }
                    self.notification_start_time = None;
                    set_dock_badge(false);
                }
            }
            self.was_focused = is_focused;
        }
    }
}
