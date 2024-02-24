use super::state::UIState;
use steel_core::settings::ThemeMode;

use eframe::egui;

#[derive(PartialEq)]
enum Section {
    Overview,
    ChatTabs,
    Chat,
}

impl Default for Section {
    fn default() -> Self {
        Self::Overview
    }
}

#[derive(Default)]
pub struct UsageWindow {
    active_tab: Section,
}

impl UsageWindow {
    pub fn show(&mut self, ctx: &egui::Context, state: &mut UIState, is_open: &mut bool) {
        egui::Window::new("help").open(is_open).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.selectable_value(&mut self.active_tab, Section::Overview, "overview");
                    ui.selectable_value(&mut self.active_tab, Section::ChatTabs, "chat tabs");
                    ui.selectable_value(&mut self.active_tab, Section::Chat, "chat");
                });
                ui.separator();
                match self.active_tab {
                    Section::Overview => self.show_overview_usage(ctx, ui, state),
                    Section::ChatTabs => self.show_chat_tabs_usage(ui),
                    Section::Chat => self.show_chat_usage(ui),
                }
            });
        });
    }
}

impl UsageWindow {
    fn show_overview_usage(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, state: &mut UIState) {
        ui.vertical(|ui| {
            ui.heading("overview");
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label(
                    "most of the UI elements are self-explanatory -- hover over menu items, buttons, \
                    input fields, and other controls to see what they do. example: "
                );
                if let Some(theme) = ctx.style().visuals.light_dark_small_toggle_button(ui) {
                    state.settings.ui.theme = if theme.dark_mode {
                        ThemeMode::Dark
                    } else {
                        ThemeMode::Light
                    };
                }
            });

            ui.label(
                "\noverall, steel mostly behaves like you would expect from a regular chat client -- \
                it hauls messages back and forth across the network, reconnects automatically with 15 second retries, \
                draws your attention to missed messages, etc, etc.\n\
                \n\
                if something unexpected happens, make sure to check the log file, and tell me about the issue."
            );
        });
    }

    fn show_chat_tabs_usage(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.heading("chat tabs");
            ui.label(
                "tabs can be closed by middle clicking them. right click brings up a context menu with extra actions. \
                to reorder the tabs, drag them around while holding shift."
            );

            ui.heading("system tabs");
            ui.label(
                "- \"highlights\": all your mentions -- click the chat name to find out the context\n\
                - \"system\": messages from the chat server. most of the time it's just Echo's rad ASCII art, \
                but occasionally a stray error might show up (for example, in case of incorrect password)"
            );
        });
    }

    fn show_chat_usage(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.heading("chat");
            ui.label(
                "- type / to see available chat commands. the commands without input parameters are instant, so take heed.\n\
                - hover over a timestamp to see its date and UTC time. the messages are stamped on delivery \
                to your device, so unless the network is terribly laggy, 99.9% of the time it's accurate enough."
            );

            ui.heading("actions");
            ui.label(
                "- Ctrl + F anywhere: open chat filter dialog \n\
                - left click on user: insert username into text input\n\
                - right click on user: show context menu"
            );

            ui.heading("some more UX wisdom");
            ui.label(
                "- chat links are clickable and can be copied\n\
                - \"mIRC colours\" aren't supported. AOL has died, and so has MSN -- sorry! we'll all be there."
            )
        });
    }
}
