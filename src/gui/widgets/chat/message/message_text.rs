use eframe::egui::{self, Widget};
use steel_core::{
    chat::{
        links::{Action, LinkType},
        MessageChunk,
    },
    ipc::client::CoreClient,
    settings::chat::ChatBehaviour,
    TextStyle,
};

use crate::gui::{
    widgets::chat::links::{
        beatmap_link::{BeatmapDifficultyLink, BeatmapLink},
        channel_link::ChannelLink,
        chat_link::ChatLink,
        regular_link::RegularLink,
    },
    DecoratedText,
};

pub struct ChatMessageText<'msg, 'app> {
    chunks: &'msg Vec<MessageChunk>,
    styles: Option<&'msg Vec<TextStyle>>,

    behaviour: &'app ChatBehaviour,
    core_client: &'app CoreClient,
}

impl<'msg, 'app> ChatMessageText<'msg, 'app> {
    pub fn new(
        chunks: &'msg Vec<MessageChunk>,
        styles: Option<&'msg Vec<TextStyle>>,
        behaviour: &'app ChatBehaviour,
        core_client: &'app CoreClient,
    ) -> Self {
        Self {
            chunks,
            styles,
            behaviour,
            core_client,
        }
    }
}

impl Widget for ChatMessageText<'_, '_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let layout = egui::Layout::from_main_dir_and_cross_align(
            egui::Direction::LeftToRight,
            egui::Align::Center,
        )
        .with_main_wrap(true)
        .with_cross_justify(false);

        let resp = ui.with_layout(layout, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for c in self.chunks {
                match &c {
                    MessageChunk::Text(text) => {
                        let display_text = egui::RichText::new(text).with_styles(self.styles);
                        ui.label(display_text);
                    }
                    MessageChunk::Link {
                        title,
                        location,
                        link_type,
                    } => {
                        let display_text = egui::RichText::new(title).with_styles(self.styles);
                        match link_type {
                            LinkType::HTTP | LinkType::HTTPS => {
                                ui.add(RegularLink::new(&display_text, location));
                            }
                            LinkType::OSU(osu_action) => match osu_action {
                                Action::Chat(chat_name) => {
                                    ui.add(ChatLink::new(
                                        chat_name,
                                        &display_text,
                                        location,
                                        self.behaviour,
                                        self.core_client,
                                    ));
                                }
                                Action::OpenBeatmap(beatmap_id) => {
                                    ui.add(BeatmapLink::new(
                                        *beatmap_id,
                                        &display_text,
                                        self.behaviour,
                                    ));
                                }

                                Action::OpenDifficulty(difficulty_id) => {
                                    ui.add(BeatmapDifficultyLink::new(
                                        *difficulty_id,
                                        &display_text,
                                        self.behaviour,
                                    ));
                                }

                                Action::Multiplayer(_lobby_id) => {
                                    ui.add(RegularLink::new(&display_text, location));
                                }
                            },
                            LinkType::Channel => {
                                ui.add(ChannelLink::new(&display_text, location, self.core_client));
                            }
                        }
                    }
                }
            }
        });
        resp.response
    }
}
