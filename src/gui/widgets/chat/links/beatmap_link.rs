use eframe::egui;

use crate::gui::{context_menu::url::menu_item_copy_url, state::UIState};

use super::regular_link::RegularLink;

struct BaseBeatmapLink<'link> {
    display_text: &'link egui::RichText,
    on_hover_text: String,
    location: String,

    ui_state: &'link UIState,
}

impl egui::Widget for BaseBeatmapLink<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        match self
            .ui_state
            .settings
            .chat
            .behaviour
            .handle_osu_beatmap_links
        {
            false => ui.add(RegularLink::new(self.display_text, &self.location)),
            true => {
                let resp = ui
                    .link(self.display_text.clone())
                    .on_hover_text_at_pointer(self.on_hover_text);

                resp.context_menu(|ui| menu_item_copy_url(ui, &self.location));

                if resp.clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(self.location));
                }

                resp
            }
        }
    }
}

pub struct BeatmapLink<'link> {
    inner: BaseBeatmapLink<'link>,
}

impl<'link> BeatmapLink<'link> {
    pub fn new(
        beatmap_id: u64,
        display_text: &'link egui::RichText,
        ui_state: &'link UIState,
    ) -> Self {
        let location = format!("https://osu.ppy.sh/beatmapsets/{}", beatmap_id);
        let on_hover_text = format!("Beatmap #{} (open in browser)", beatmap_id);
        Self {
            inner: BaseBeatmapLink {
                display_text,
                on_hover_text,
                location,
                ui_state,
            },
        }
    }
}

impl egui::Widget for BeatmapLink<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self.inner)
    }
}

pub struct BeatmapDifficultyLink<'link> {
    inner: BaseBeatmapLink<'link>,
}

impl<'link> BeatmapDifficultyLink<'link> {
    pub fn new(
        difficulty_id: u64,
        display_text: &'link egui::RichText,
        ui_state: &'link UIState,
    ) -> Self {
        let location = format!("https://osu.ppy.sh/beatmaps/{}", difficulty_id);
        let on_hover_text = format!("Difficulty #{} (open in browser)", difficulty_id);
        Self {
            inner: BaseBeatmapLink {
                display_text,
                on_hover_text,
                location,
                ui_state,
            },
        }
    }
}

impl egui::Widget for BeatmapDifficultyLink<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self.inner)
    }
}
