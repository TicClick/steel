use steel_core::{chat::ChatType, settings::chat::Chat};

pub trait ChatBackend {
    fn connect(&self, settings: &Chat);
    fn disconnect(&self);
    fn send_message(&self, destination: &str, chat_type: ChatType, content: &str);
    fn send_action(&self, destination: &str, chat_type: ChatType, action: &str);
    fn join_channel(&self, channel: &str);
    fn leave_channel(&self, channel: &str);
}
