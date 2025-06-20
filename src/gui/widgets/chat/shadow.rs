use eframe::egui;

pub struct InnerShadow {
    height: usize,
}

impl InnerShadow {
    pub fn new(height: usize) -> Self {
        Self { height }
    }
}

// (Almost) as seen at https://gist.github.com/juancampa/d8dcf7cdab813062f082eac7415abcfc
impl egui::Widget for InnerShadow {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut shadow_rect = ui.available_rect_before_wrap();

        let central_frame_margin = 8.; // egui::Frame::central_panel().inner_margin
        shadow_rect.set_left(shadow_rect.left() - central_frame_margin);
        shadow_rect.set_width(
            shadow_rect.width() + ui.spacing().scroll.bar_inner_margin + central_frame_margin,
        );
        shadow_rect.set_bottom(shadow_rect.bottom() + ui.spacing().item_spacing.y);

        let colour_ctor = match ui.visuals().dark_mode {
            true => |a: u8| egui::Color32::from_rgba_unmultiplied(120, 120, 120, a),
            false => egui::Color32::from_black_alpha,
        };

        let painter = ui.painter();
        let mut avail_rect = shadow_rect.translate((0.0, shadow_rect.height() - 1.0).into());
        avail_rect.set_height(1.0);
        for i in 0..self.height {
            let alpha = 1.0 - (i as f32 / self.height as f32);
            let shift = -avail_rect.height() * i as f32;
            let rect = avail_rect.translate((0.0, shift).into());
            painter.rect_filled(rect, 0.0, colour_ctor((alpha * alpha * 80.0).floor() as u8));
        }

        ui.response()
    }
}
