use eframe::egui;

use super::state::UIState;

pub const COMMAND_PREFIX: char = '/';

pub const COMMAND_ALIAS_ME: [&str; 1] = ["/me"];

trait Command<'command> {
    fn aliases(&self) -> Vec<&'command str>;
    fn preferred_alias(&self) -> &'command str {
        self.aliases().first().unwrap()
    }
    fn argcount(&self) -> usize;

    fn hint(&self, ui: &mut egui::Ui);
    fn rich_text_example(&self) -> egui::RichText;
    fn action(&self, state: &UIState, args: Vec<String>);

    fn should_be_hinted(&self, input_parts: &[String], argcount: usize) -> bool {
        self.aliases()
            .iter()
            .any(|alias| alias.starts_with(&input_parts[0]) || input_parts[0].starts_with(alias))
            && (argcount < self.argcount() || argcount == 0 && self.argcount() == 0)
    }

    fn is_applicable(&self, input_parts: &[String]) -> bool {
        self.aliases().contains(&input_parts[0].as_str())
    }
}

struct Me {}
impl<'command> Command<'command> for Me {
    fn aliases(&self) -> Vec<&'command str> {
        COMMAND_ALIAS_ME.to_vec()
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
    fn action(&self, state: &UIState, args: Vec<String>) {
        state
            .core
            .chat_action_sent(&state.active_chat_tab_name, args.join(" ").as_str());
    }
}

pub struct CommandHelper<'command> {
    commands: Vec<Box<dyn Command<'command>>>,
}

impl Default for CommandHelper<'_> {
    fn default() -> Self {
        Self {
            commands: vec![Box::new(Me {})],
        }
    }
}

impl CommandHelper<'_> {
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
            if cmd.should_be_hinted(&args, argcount) {
                if ui
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
                        cmd.action(state, args);
                        input.clear();
                    }
                    ui.close_menu();
                    break;
                }
            }
        }
    }

    pub fn try_run_command(&self, state: &UIState, input: &mut String) -> bool {
        if !self.contains_command(input) {
            return false;
        }
        let args: Vec<String> = input.split_whitespace().map(|i| i.to_owned()).collect();
        for cmd in &self.commands {
            if cmd.is_applicable(&args) {
                cmd.action(state, args);
                input.clear();
                return true;
            }
        }
        false
    }
}
