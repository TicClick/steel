use eframe::{
    egui::{
        pos2, vec2, Color32, Label, Rangef, Response, RichText, Sense, Shape, Stroke, TextStyle,
        Vec2, Widget,
    },
    epaint,
};

pub struct UnreadMarker {
    pub ui_height: f32,
    pub line_width: f32,
    pub arrowhead_size: Vec2,
    pub text: String,
    pub color: Color32,
}

impl Default for UnreadMarker {
    fn default() -> Self {
        Self {
            ui_height: 14.0,
            line_width: 50.0,
            arrowhead_size: vec2(4.0, 7.0),
            text: "new".to_string(),
            color: Color32::DARK_RED,
        }
    }
}

impl UnreadMarker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ui_height(mut self, value: f32) -> Self {
        self.ui_height = value;
        self
    }

    pub fn line_width(mut self, value: f32) -> Self {
        self.line_width = value;
        self
    }

    pub fn arrowhead_size(mut self, value: Vec2) -> Self {
        self.arrowhead_size = value;
        self
    }

    pub fn text(mut self, value: String) -> Self {
        self.text = value;
        self
    }

    pub fn color(mut self, value: Color32) -> Self {
        self.color = value;
        self
    }
}

impl Widget for UnreadMarker {
    fn ui(self, ui: &mut eframe::egui::Ui) -> Response {
        let response = ui.horizontal(|ui| {
            let (rect, _) =
                ui.allocate_at_least(vec2(self.line_width, self.ui_height), Sense::hover());
            let p = ui.painter();

            let arrowhead_tip = pos2(rect.left(), p.round_to_pixel_center(rect.center().y));
            let arrowhead_left_top = pos2(
                arrowhead_tip.x - self.arrowhead_size.x,
                arrowhead_tip.y - self.arrowhead_size.y / 2.0,
            );
            let arrowhead_left_bottom = pos2(
                arrowhead_tip.x - self.arrowhead_size.x,
                arrowhead_tip.y + self.arrowhead_size.y / 2.0,
            );

            let arrow_stroke = Stroke::new(1., self.color);

            p.add(Shape::Path(epaint::PathShape::convex_polygon(
                vec![arrowhead_left_top, arrowhead_tip, arrowhead_left_bottom],
                self.color,
                arrow_stroke,
            )));

            p.hline(
                rect.left()..=rect.left() + self.line_width,
                arrowhead_tip.y,
                arrow_stroke,
            );

            let text_end = ui.add(
                Label::new(
                    RichText::new(self.text)
                        .text_style(TextStyle::Small)
                        .color(self.color),
                )
                .selectable(false),
            );

            let p = ui.painter();
            p.hline(
                Rangef::new(
                    text_end.rect.right() + ui.spacing().item_spacing.x,
                    text_end.rect.right() + ui.spacing().item_spacing.x + self.line_width,
                ),
                arrowhead_tip.y,
                arrow_stroke,
            );
        });
        response.response
    }
}
