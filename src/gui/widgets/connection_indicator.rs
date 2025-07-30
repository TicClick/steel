use std::time::Instant;

use eframe::egui;

#[derive(Debug)]
pub struct ConnectionIndicator {
    last_activity: Instant,
    _last_delta_update: Instant,
    _cached_delta_ms: i64,

    connected: bool,
    server: String,
    ping_timeout: u32,
}

impl ConnectionIndicator {
    pub fn new(connected: bool, server: String, ping_timeout: u32) -> Self {
        let now = Instant::now();
        Self {
            last_activity: now,
            _last_delta_update: now,
            _cached_delta_ms: 0,
            connected,
            server,
            ping_timeout,
        }
    }

    pub fn refresh(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn delta_ms(&mut self) -> i64 {
        let update_lag = self._last_delta_update.elapsed();
        if update_lag.as_millis() > 1000 {
            self._cached_delta_ms = self.last_activity.elapsed().as_millis() as i64;
            self._last_delta_update = Instant::now();
        }

        self._cached_delta_ms
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
    }

    pub fn connect(&mut self, server: String, ping_timeout: u32) {
        self.refresh();
        self.connected = true;
        self.server = server;
        self.ping_timeout = ping_timeout;
    }

    pub fn signal_strength(&self, delta_ms: i64) -> i32 {
        match self.connected {
            true => match delta_ms {
                0..10_000 => 4,
                10_000..25_000 => 3,
                25_000..=40_000 => 2,
                _ => 1,
            },
            false => 0,
        }
    }
}

impl Default for ConnectionIndicator {
    fn default() -> Self {
        Self::new(false, String::new(), 40)
    }
}

impl ConnectionIndicator {
    pub fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let delta_ms = self.delta_ms();
        let signal_strength = self.signal_strength(delta_ms);

        let response = ui.ctx().read_response(ui.next_auto_id());
        let visuals = response.map_or(&ui.style().visuals.widgets.inactive, |response| {
            ui.style().interact(&response)
        });

        let frame = egui::Frame::new()
            .inner_margin(ui.style().spacing.button_padding)
            .stroke(visuals.bg_stroke)
            .corner_radius(visuals.corner_radius);

        let response = frame
            .show(ui, |ui| {
                let bar_width = 4;
                let bar_spacing = 2;
                let total_width = 4 * bar_width + 3 * bar_spacing;
                let max_height = 12;

                let (rect, response) = ui.allocate_exact_size(
                    egui::Vec2::new(total_width as f32, max_height as f32),
                    egui::Sense::hover(),
                );

                let painter = ui.painter();

                for i in 0..4 {
                    let bar_height = (i + 1) * 3;
                    let x = rect.min.x + i as f32 * ((bar_width + bar_spacing) as f32);
                    let y = rect.max.y - bar_height as f32;

                    let bar_rect = egui::Rect::from_min_size(
                        egui::Pos2::new(x, y),
                        egui::Vec2::new(bar_width as f32, bar_height as f32),
                    );

                    let fill_color = if i < signal_strength {
                        ui.style().visuals.text_color()
                    } else {
                        ui.style().visuals.panel_fill
                    };

                    painter.rect(
                        bar_rect,
                        0.0,
                        fill_color,
                        egui::Stroke::new(1.0, ui.style().visuals.text_color()),
                        egui::StrokeKind::Inside,
                    );
                }

                response
            })
            .response;

        let delta_s = (delta_ms as f32) / 1000.0;
        let on_hover_text = match self.connected {
            true => format!(
                "network activity: {delta_s:.1} s ago\n\
                server: {}\n\
                ping timeout: {} s",
                self.server, self.ping_timeout
            ),
            false => "offline".into(),
        };
        response.on_hover_text_at_pointer(on_hover_text)
    }
}
