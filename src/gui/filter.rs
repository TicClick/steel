use eframe::egui;

use steel_core::chat::Message;

use super::state::UIState;

// FIXME: This works on a premise that `input` of all filters is always lowercase, which isn't necessarily true
// (since a user can modify it directly).

// FIXME: A proper solution is keep a copy of all messages in lowercase somewhere to avoid doing that on every iteration.

pub trait FilterCondition: Sized {
    fn matches(&self, message: &Message) -> bool;
    fn reset(&mut self);
}

#[derive(Debug, Default)]
pub struct UsernameFilter {
    pub input: String,
}

impl From<&str> for UsernameFilter {
    fn from(value: &str) -> Self {
        Self {
            input: value.to_lowercase().to_owned(),
        }
    }
}

impl FilterCondition for UsernameFilter {
    fn matches(&self, message: &Message) -> bool {
        if self.input.is_empty() {
            return true;
        }
        message.username.to_lowercase().contains(&self.input)
    }

    fn reset(&mut self) {
        self.input.clear();
    }
}

#[derive(Debug, Default)]
pub struct TextFilter {
    pub input: String,
}

impl From<&str> for TextFilter {
    fn from(value: &str) -> Self {
        Self {
            input: value.to_lowercase().to_owned(),
        }
    }
}

impl FilterCondition for TextFilter {
    fn matches(&self, message: &Message) -> bool {
        if self.input.is_empty() {
            return true;
        }
        message.text.to_lowercase().contains(&self.input)
    }

    fn reset(&mut self) {
        self.input.clear();
    }
}

#[derive(Debug, Default)]
pub struct FilterCollection {
    pub username: UsernameFilter,
    pub text: TextFilter,
    pub active: bool,
}

impl FilterCollection {
    pub fn matches(&self, message: &Message) -> bool {
        if !self.active {
            return true;
        }
        self.username.matches(message) && self.text.matches(message)
    }

    pub fn reset(&mut self) {
        self.username.reset();
        self.text.reset();
    }
}

#[derive(Default)]
pub struct FilterWindow {
    show_ui: bool,
}

impl FilterWindow {
    pub fn show(&mut self, ctx: &egui::Context, state: &mut UIState) {
        let mut activated_now = false;

        if !self.show_ui && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F)) {
            self.show_ui = true;
            state.filter.active = true;
            activated_now = true;
        }

        if self.show_ui && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_ui = false;
            activated_now = false;
        }

        if !self.show_ui {
            state.filter.active = false;
            return;
        }

        egui::Window::new("chat filter")
            .auto_sized()
            .open(&mut self.show_ui)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("message");
                        let resp = ui.add(egui::TextEdit::singleline(&mut state.filter.text.input).desired_width(150.));
                        if activated_now {
                            resp.request_focus();
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("username");
                        ui.add(egui::TextEdit::singleline(&mut state.filter.username.input).desired_width(150.));
                    });
                    if ui.button("reset").clicked() {
                        state.filter.reset();
                    }
                });
            });
    }
}
