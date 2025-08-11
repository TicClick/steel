use eframe::egui;
use steel_core::chat::{Chat, MessageType};
use steel_core::settings::ui::ChatColours;
use steel_core::TextStyle;

use crate::gui::state::UIState;

const MAX_USERNAME_FILTER_LENGTH: usize = 20;
const MAX_MESSAGE_FILTER_LENGTH: usize = 100;

#[cfg(target_os = "macos")]
const SEARCH_BUTTON_HINT: &str = "Enter or ⌘+F: Next result\n\
    Shift+Enter or ⌘+Shift+F: Previous result";

#[cfg(not(target_os = "macos"))]
const SEARCH_BUTTON_HINT: &'static str = "Enter or Ctrl+F: Next result\n\
    Shift+Enter or Ctrl+Shift+F: Previous result";

enum FilterAction {
    Search,
    NavigateToResult(usize),
}

#[derive(Default)]
pub struct ChatFilter {
    pub should_show_filter: bool,
    pub user_filter_input: String,
    pub message_filter_input: String,

    cached_search_results: Vec<usize>,
    current_search_index: usize,
    last_search_query: (String, String), // (user_filter, message_filter)
    search_performed: bool,
}

impl ChatFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enable(&mut self) {
        self.should_show_filter = true;
    }

    pub fn is_active(&self) -> bool {
        self.should_show_filter && self.search_performed && !self.cached_search_results.is_empty()
    }

    pub fn has_results(&self) -> bool {
        !self.cached_search_results.is_empty()
    }

    pub fn get_current_result_index(&self) -> Option<usize> {
        self.cached_search_results
            .get(self.current_search_index)
            .copied()
    }

    pub fn is_message_highlighted(&self, message_idx: usize) -> Option<bool> {
        if !self.should_show_filter || !self.cached_search_results.contains(&message_idx) {
            return None;
        }

        let is_current_result =
            if let Some(&current_idx) = self.cached_search_results.get(self.current_search_index) {
                current_idx == message_idx
            } else {
                false
            };

        Some(is_current_result)
    }

    pub fn get_highlight_style(
        &self,
        message_idx: usize,
        colours: &ChatColours,
    ) -> Option<TextStyle> {
        match self.is_message_highlighted(message_idx) {
            Some(true) => Some(TextStyle::SearchResult(
                colours.search_result_current.clone().into(),
            )),
            Some(false) => Some(TextStyle::SearchResult(
                colours.search_result_other.clone().into(),
            )),
            None => None,
        }
    }

    fn search_messages(&mut self, chat: &Chat) {
        let current_query = (
            self.user_filter_input.clone(),
            self.message_filter_input.clone(),
        );

        if current_query != self.last_search_query {
            self.cached_search_results.clear();
            self.current_search_index = 0;
            self.last_search_query = current_query;

            if self.user_filter_input.is_empty() && self.message_filter_input.is_empty() {
                return;
            }

            for (idx, message) in chat.messages.iter().enumerate() {
                if message.r#type == MessageType::System {
                    continue;
                }

                let username_matches = self.user_filter_input.is_empty()
                    || message
                        .username
                        .to_lowercase()
                        .contains(&self.user_filter_input);
                let message_matches = self.message_filter_input.is_empty()
                    || message
                        .text
                        .to_lowercase()
                        .contains(&self.message_filter_input);

                if username_matches && message_matches {
                    self.cached_search_results.push(idx);
                }
            }
        }
    }

    fn navigate_to_next_result(&mut self) -> Option<usize> {
        if !self.cached_search_results.is_empty() {
            self.current_search_index =
                (self.current_search_index + 1) % self.cached_search_results.len();
            self.cached_search_results
                .get(self.current_search_index)
                .copied()
        } else {
            None
        }
    }

    fn navigate_to_prev_result(&mut self) -> Option<usize> {
        if !self.cached_search_results.is_empty() {
            self.current_search_index =
                (self.cached_search_results.len() + self.current_search_index - 1)
                    % self.cached_search_results.len();
            self.cached_search_results
                .get(self.current_search_index)
                .copied()
        } else {
            None
        }
    }

    fn clear_search_cache(&mut self) {
        self.cached_search_results.clear();
        self.current_search_index = 0;
        self.last_search_query = (String::new(), String::new());
        self.search_performed = false;
    }

    pub fn perform_search(&mut self, chat: &Chat) -> Option<usize> {
        self.search_messages(chat);
        self.search_performed = true;
        if !self.cached_search_results.is_empty() {
            self.current_search_index = 0;
            self.cached_search_results.first().copied()
        } else {
            None
        }
    }

    fn handle_search_navigation(&mut self, shift_pressed: bool) -> Option<FilterAction> {
        if shift_pressed {
            // Shift+action - navigate to previous result
            self.navigate_to_prev_result()
                .map(FilterAction::NavigateToResult)
        } else {
            // Normal action - search or navigate to next result
            if self.cached_search_results.is_empty()
                || self.last_search_query
                    != (
                        self.user_filter_input.clone(),
                        self.message_filter_input.clone(),
                    )
            {
                Some(FilterAction::Search)
            } else {
                self.navigate_to_next_result()
                    .map(FilterAction::NavigateToResult)
            }
        }
    }

    fn handle_text_field_enter(
        &mut self,
        ui: &egui::Ui,
        resp: &egui::Response,
        filter_action: &mut Option<FilterAction>,
        is_shift_pressed: bool,
    ) {
        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            *filter_action = self.handle_search_navigation(is_shift_pressed);
            resp.request_focus();
        }
    }

    pub fn handle_input(&mut self, ctx: &egui::Context) -> (bool, Option<usize>) {
        let mut activated_now = false;
        let mut scroll_to = None;

        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F)) {
            if !self.should_show_filter {
                // First time opening filter - just show the UI, don't search yet
                self.should_show_filter = true;
                activated_now = true;
            } else {
                // If filter is already shown, navigate to next/prev result
                match ctx.input(|i| i.modifiers.shift) {
                    true => {
                        if let Some(message_idx) = self.navigate_to_prev_result() {
                            scroll_to = Some(message_idx);
                        }
                    }
                    false => {
                        if let Some(message_idx) = self.navigate_to_next_result() {
                            scroll_to = Some(message_idx);
                        }
                    }
                }
            }
        }

        if self.should_show_filter && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.should_show_filter = false;
            self.clear_search_cache();
            activated_now = false;
        }

        // Don't automatically search when filter is shown

        (activated_now, scroll_to)
    }

    pub fn show_ui(
        &mut self,
        ctx: &egui::Context,
        _state: &UIState,
        chat: &Chat,
        activated_now: bool,
    ) -> Option<usize> {
        if !self.should_show_filter {
            return None;
        }

        let mut filter_action: Option<FilterAction> = None;

        let username_input_id = format!("username-filter-input-{}", chat.normalized_name);
        let message_input_id = format!("message-filter-input-{}", chat.normalized_name);

        if activated_now {
            ctx.memory_mut(|mem| mem.request_focus(username_input_id.clone().into()));
        }

        egui::TopBottomPanel::top(format!("filter-panel-{}", chat.normalized_name))
            .frame(
                egui::Frame::central_panel(&ctx.style()).inner_margin(egui::Margin {
                    left: 8,
                    right: 8,
                    top: 4,
                    bottom: 4,
                }),
            )
            .show(ctx, |ui| {
                let is_shift_pressed = ui.input(|i| i.modifiers.shift);

                ui.horizontal_centered(|ui| {
                    let message_filter_input =
                        egui::TextEdit::singleline(&mut self.message_filter_input)
                            .char_limit(MAX_MESSAGE_FILTER_LENGTH)
                            .id_source(&message_input_id)
                            .hint_text("Message text")
                            .desired_width((7.0 * MAX_MESSAGE_FILTER_LENGTH as f32).min(200.0));
                    let resp = ui.add(message_filter_input);
                    self.handle_text_field_enter(ui, &resp, &mut filter_action, is_shift_pressed);

                    if resp.changed() {
                        self.message_filter_input = self.message_filter_input.to_lowercase();
                    }

                    if activated_now {
                        resp.request_focus();
                    }

                    let user_filter_field = egui::TextEdit::singleline(&mut self.user_filter_input)
                        .char_limit(MAX_USERNAME_FILTER_LENGTH)
                        .id_source(&username_input_id)
                        .hint_text("Username")
                        .desired_width(7.0 * MAX_USERNAME_FILTER_LENGTH as f32);
                    let resp = ui.add(user_filter_field);
                    self.handle_text_field_enter(ui, &resp, &mut filter_action, is_shift_pressed);

                    if resp.changed() {
                        self.user_filter_input = self.user_filter_input.to_lowercase();
                    }

                    let resp = ui
                        .button("Find")
                        .on_hover_text_at_pointer(SEARCH_BUTTON_HINT);

                    if resp.clicked() {
                        filter_action =
                            self.handle_search_navigation(ctx.input(|i| i.modifiers.shift));
                    }

                    if !self.cached_search_results.is_empty() {
                        ui.separator();
                        ui.label(format!(
                            "{} of {} results",
                            self.current_search_index + 1,
                            self.cached_search_results.len()
                        ));
                    } else if self.search_performed {
                        ui.separator();
                        ui.label("No results");
                    }
                });
            });

        match filter_action {
            Some(FilterAction::Search) => self.perform_search(chat),
            Some(FilterAction::NavigateToResult(message_idx)) => Some(message_idx),
            None => None,
        }
    }
}
