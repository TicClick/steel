use steel_core::settings::chat::Chat;

pub trait ChatBackend {
    fn connect(&self, settings: &Chat);
    fn disconnect(&self);
    fn send_message(&self, destination: &str, content: &str);
    fn send_action(&self, destination: &str, action: &str);
    fn join_channel(&self, channel: &str);
    fn leave_channel(&self, channel: &str);
}
