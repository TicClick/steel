use eframe::egui;

pub struct SelectableButton {
    text: egui::WidgetText,
    selectable_text: String,
}

impl SelectableButton {
    pub fn new(text: impl Into<egui::WidgetText>, selectable_text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            selectable_text: selectable_text.into(),
        }
    }
}

impl egui::Widget for SelectableButton {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let resp = ui.add(
            egui::Label::new(
                egui::RichText::new(&self.selectable_text).color(egui::Color32::TRANSPARENT),
            )
            .selectable(true),
        );

        // Draw button appearance over the selectable label, then draw visible text on top.

        let button_rect = resp.rect.shrink2(egui::Vec2::new(2., 0.));
        let visuals = ui.style().interact(&resp);
        ui.painter()
            .rect_filled(button_rect, visuals.corner_radius, visuals.bg_fill);
        if visuals.bg_stroke.width > 0.0 {
            ui.painter().rect_stroke(
                button_rect,
                visuals.corner_radius,
                visuals.bg_stroke,
                egui::epaint::StrokeKind::Outside,
            );
        }

        let text_galley = self.text.into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Button,
        );
        ui.painter().galley(
            button_rect.center() - text_galley.size() / 2.0,
            text_galley,
            visuals.text_color(),
        );

        resp.on_hover_cursor(egui::CursorIcon::Default)
    }
}
