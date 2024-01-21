use eframe::egui;

use super::state::UIState;

pub const COMMAND_PREFIX: char = '/';

trait Command {
    fn new() -> Self
    where
        Self: Sized;

    fn aliases(&self) -> &Vec<String>;
    fn preferred_alias(&self) -> &String {
        self.aliases().first().unwrap()
    }
    fn argcount(&self) -> usize;

    fn hint(&self, ui: &mut egui::Ui);
    fn rich_text_example(&self) -> egui::RichText;
    fn action(&self, state: &UIState, args: Vec<String>);

    fn should_be_hinted(&self, input_prefix: &str, argcount: usize) -> bool {
        self.aliases()
            .iter()
            .any(|alias| alias.starts_with(input_prefix) || input_prefix.starts_with(alias))
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

    fn aliases(&self) -> &Vec<String> {
        &self.aliases
    }
    fn argcount(&self) -> usize {
        1
    }
    fn rich_text_example(&self) -> egui::RichText {
        egui::RichText::new("/me <action>")
    }
    fn hint(&self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label("send a third-person action to the chat.");
            ui.label("example: /me gets to live one more day");
        });
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
    fn aliases(&self) -> &Vec<String> {
        &self.aliases
    }
    fn argcount(&self) -> usize {
        1
    }
    fn rich_text_example(&self) -> egui::RichText {
        egui::RichText::new("/chat <user>, /chat #<channel>")
    }
    fn hint(&self, ui: &mut egui::Ui) {
        ui.label("open a chat tab with <user>, or join #<channel>");
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
    fn aliases(&self) -> &Vec<String> {
        &self.aliases
    }
    fn argcount(&self) -> usize {
        0
    }
    fn rich_text_example(&self) -> egui::RichText {
        egui::RichText::new("/close, /part")
    }
    fn hint(&self, ui: &mut egui::Ui) {
        ui.label("close the active tab, or leave the channel");
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
    fn aliases(&self) -> &Vec<String> {
        &self.aliases
    }
    fn argcount(&self) -> usize {
        0
    }
    fn rich_text_example(&self) -> egui::RichText {
        egui::RichText::new("/clear, /c")
    }
    fn hint(&self, ui: &mut egui::Ui) {
        ui.label("clear the active tab, removing all messages");
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
        Self {
            commands: vec![
                Box::new(Me::new()),
                // Box::new(OpenChat::new()),
                // Box::new(CloseChat::new()),
                // Box::new(ClearChat::new()),
            ],
        }
    }
}

impl CommandHelper {
    pub fn contains_command(&self, input: &str) -> bool {
        input.starts_with(COMMAND_PREFIX)
    }

    pub fn maybe_show(&self, ui: &mut egui::Ui, state: &UIState, input: &mut String) {
        if !self.contains_command(input) {
            return;
        }

        let args: Vec<String> = input.split_whitespace().map(|i| i.to_owned()).collect();
        let argcount = args.len() - 1;

        for cmd in &self.commands {
            if cmd.should_be_hinted(&args[0], argcount)
                && ui
                    .button(cmd.rich_text_example())
                    .on_hover_ui_at_pointer(|ui| cmd.hint(ui))
                    .clicked()
            {
                // TODO(TicClick): Move the cursor to the end of the message

                if argcount == 0 {
                    // Nothing entered: complete the command
                    *input = format!("{} ", cmd.preferred_alias());
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
