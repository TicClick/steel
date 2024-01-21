use eframe::egui;

use super::state::UIState;

pub const COMMAND_PREFIX: char = '/';

trait Command {
    fn new() -> Self
    where
        Self: Sized;

    fn description(&self) -> &str;
    fn aliases(&self) -> &Vec<String>;
    fn preferred_alias(&self) -> &String {
        self.aliases().first().unwrap()
    }
    fn example(&self) -> &str;
    fn argcount(&self) -> usize;

    fn ui_hint(&self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = [0.0, 0.0].into();
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("- description: ").strong());
                ui.label(self.description());
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("- example: ").strong());
                ui.label(self.example());
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("- aliases: ").strong());
                ui.label(self.aliases().join(", "));
            });
        });
    }
    fn ui_title(&self) -> egui::RichText;
    fn action(&self, state: &UIState, args: Vec<String>);

    fn should_be_hinted(&self, input_prefix: &str, argcount: usize) -> bool {
        self.aliases()
            .iter()
            .any(|alias| alias.starts_with(input_prefix))
            && (argcount < self.argcount() || argcount == 0 && self.argcount() == 0)
    }

    fn is_applicable(&self, input_prefix: &str) -> bool {
        self.aliases().iter().any(|a| a == input_prefix)
    }
}

struct Me {
    pub aliases: Vec<String>,
}

impl Command for Me {
    fn new() -> Self {
        Self {
            aliases: ["/me".into()].to_vec(),
        }
    }
    fn description(&self) -> &str {
        "send a third-person action to the chat"
    }
    fn example(&self) -> &str {
        "/me gets to live one more day"
    }
    fn aliases(&self) -> &Vec<String> {
        &self.aliases
    }
    fn argcount(&self) -> usize {
        1
    }
    fn ui_title(&self) -> egui::RichText {
        egui::RichText::new("/me <action>")
    }
    fn action(&self, state: &UIState, input_parts: Vec<String>) {
        state
            .core
            .chat_action_sent(&state.active_chat_tab_name, input_parts.join(" ").as_str());
    }
}

struct OpenChat {
    pub aliases: Vec<String>,
}

impl Command for OpenChat {
    fn new() -> Self {
        Self {
            aliases: ["/chat".into(), "/query".into(), "/q".into(), "/join".into()].to_vec(),
        }
    }
    fn description(&self) -> &str {
        "open a chat tab with user, or join a new #channel"
    }
    fn example(&self) -> &str {
        "/chat #russian"
    }
    fn aliases(&self) -> &Vec<String> {
        &self.aliases
    }
    fn argcount(&self) -> usize {
        1
    }
    fn ui_title(&self) -> egui::RichText {
        egui::RichText::new("/chat <user or #channel>")
    }
    fn action(&self, state: &UIState, args: Vec<String>) {
        state.core.private_chat_opened(&args[0]);
    }
}

struct CloseChat {
    pub aliases: Vec<String>,
}

impl Command for CloseChat {
    fn new() -> Self {
        Self {
            aliases: ["/close".into(), "/part".into()].to_vec(),
        }
    }
    fn description(&self) -> &str {
        "close the active tab, or leave the channel"
    }
    fn example(&self) -> &str {
        self.preferred_alias()
    }
    fn aliases(&self) -> &Vec<String> {
        &self.aliases
    }
    fn argcount(&self) -> usize {
        0
    }
    fn ui_title(&self) -> egui::RichText {
        egui::RichText::new(self.preferred_alias())
    }
    fn action(&self, state: &UIState, _args: Vec<String>) {
        state
            .core
            .chat_tab_closed(&state.active_chat_tab_name.to_lowercase());
    }
}

struct ClearChat {
    pub aliases: Vec<String>,
}

impl Command for ClearChat {
    fn new() -> Self {
        Self {
            aliases: ["/clear".into(), "/c".into()].to_vec(),
        }
    }
    fn description(&self) -> &str {
        "clear the active tab, removing all messages"
    }
    fn example(&self) -> &str {
        self.preferred_alias()
    }
    fn aliases(&self) -> &Vec<String> {
        &self.aliases
    }
    fn argcount(&self) -> usize {
        0
    }
    fn ui_title(&self) -> egui::RichText {
        egui::RichText::new(self.preferred_alias())
    }
    fn action(&self, state: &UIState, _args: Vec<String>) {
        state
            .core
            .chat_tab_cleared(&state.active_chat_tab_name.to_lowercase())
    }
}

pub struct CommandHelper {
    commands: Vec<Box<dyn Command>>,
}

impl Default for CommandHelper {
    fn default() -> Self {
        let mut s = Self {
            commands: vec![
                Box::new(Me::new()),
                Box::new(OpenChat::new()),
                Box::new(CloseChat::new()),
                Box::new(ClearChat::new()),
            ],
        };
        s.commands
            .sort_by(|c1, c2| c1.preferred_alias().cmp(c2.preferred_alias()));
        s
    }
}

impl CommandHelper {
    pub fn contains_command(&self, input: &str) -> bool {
        input.starts_with(COMMAND_PREFIX)
    }

    pub fn maybe_show(
        &self,
        ui: &mut egui::Ui,
        state: &UIState,
        input: &mut String,
        chat_input_id: &Option<egui::Id>,
    ) {
        if !self.contains_command(input) {
            return;
        }

        let args: Vec<String> = input.split_whitespace().map(|i| i.to_owned()).collect();
        let argcount = args.len() - 1;

        for cmd in &self.commands {
            if cmd.should_be_hinted(&args[0], argcount)
                && ui
                    .button(cmd.ui_title())
                    .on_hover_ui_at_pointer(|ui| cmd.ui_hint(ui))
                    .clicked()
            {
                if argcount == 0 {
                    // Nothing entered: complete the command
                    *input = format!("{} ", cmd.preferred_alias());
                    if let Some(ciid) = chat_input_id {
                        if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), *ciid) {
                            let ccursor = egui::text::CCursor::new(input.chars().count());
                            state.set_ccursor_range(Some(egui::text::CCursorRange::one(ccursor)));
                            state.store(ui.ctx(), *ciid);
                        }
                    }
                } else if argcount < cmd.argcount() && !input.ends_with(' ') {
                    // add extra hint
                    input.push(' ');
                }

                if cmd.argcount() == 0 {
                    cmd.action(state, args[1..].to_vec());
                    input.clear();
                }
                ui.close_menu();
                break;
            }
        }
    }

    pub fn detect_and_run(&self, state: &UIState, input: &mut String) -> bool {
        if !self.contains_command(input) {
            return false;
        }
        let args: Vec<String> = input.split_whitespace().map(|i| i.to_owned()).collect();
        for cmd in &self.commands {
            if cmd.is_applicable(&args[0]) {
                cmd.action(state, args[1..].to_vec());
                input.clear();
                return true;
            }
        }
        false
    }
}
