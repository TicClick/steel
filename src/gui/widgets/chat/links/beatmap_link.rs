use eframe::egui;
use steel_core::settings::chat::ChatBehaviour;

use crate::gui::context_menu::url::menu_item_copy_url;

use super::regular_link::RegularLink;

struct BaseBeatmapLink<'link, 'app> {
    display_text: &'link egui::RichText,
    on_hover_text: String,
    location: String,
    behaviour: &'app ChatBehaviour,
}

impl egui::Widget for BaseBeatmapLink<'_, '_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        match self.behaviour.handle_osu_beatmap_links {
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

pub struct BeatmapLink<'link, 'app> {
    inner: BaseBeatmapLink<'link, 'app>,
}

impl<'link, 'app> BeatmapLink<'link, 'app> {
    pub fn new(
        beatmap_id: u64,
        display_text: &'link egui::RichText,
        behaviour: &'app ChatBehaviour,
    ) -> Self {
        let location = format!("https://osu.ppy.sh/beatmapsets/{}", beatmap_id);
        let on_hover_text = format!("Beatmap #{} (open in browser)", beatmap_id);
        Self {
            inner: BaseBeatmapLink {
                display_text,
                on_hover_text,
                location,
                behaviour,
            },
        }
    }
}

impl egui::Widget for BeatmapLink<'_, '_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self.inner)
    }
}

pub struct BeatmapDifficultyLink<'link, 'app> {
    inner: BaseBeatmapLink<'link, 'app>,
}

impl<'link, 'app> BeatmapDifficultyLink<'link, 'app> {
    pub fn new(
        difficulty_id: u64,
        display_text: &'link egui::RichText,
        behaviour: &'app ChatBehaviour,
    ) -> Self {
        let location = format!("https://osu.ppy.sh/beatmaps/{}", difficulty_id);
        let on_hover_text = format!("Difficulty #{} (open in browser)", difficulty_id);
        Self {
            inner: BaseBeatmapLink {
                display_text,
                on_hover_text,
                location,
                behaviour,
            },
        }
    }
}

impl egui::Widget for BeatmapDifficultyLink<'_, '_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self.inner)
    }
}
