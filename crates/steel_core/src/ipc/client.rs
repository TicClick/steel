use tokio::sync::mpsc::Sender;

use crate::ipc::server::AppMessageIn;
use crate::settings::Settings;

#[derive(Debug)]
pub struct CoreClient {
    server: Sender<AppMessageIn>,
}

impl CoreClient {
    pub fn new(server: Sender<AppMessageIn>) -> Self {
        Self { server }
    }
}

impl CoreClient {
    pub fn channel_opened(&self, channel: &str) {
        self.server
            .blocking_send(AppMessageIn::UIChannelOpened(channel.to_owned()))
            .unwrap();
    }

    pub fn private_chat_opened(&self, chat: &str) {
        self.server
            .blocking_send(AppMessageIn::UIPrivateChatOpened(chat.to_owned()))
            .unwrap();
    }

    pub fn channel_join_requested(&self, channel: &str) {
        self.server
            .blocking_send(AppMessageIn::UIChannelJoinRequested(channel.to_owned()))
            .unwrap();
    }

    pub fn settings_requested(&self) {
        self.server
            .blocking_send(AppMessageIn::UISettingsRequested)
            .unwrap();
    }

    pub fn settings_updated(&self, settings: &Settings) {
        self.server
            .blocking_send(AppMessageIn::UISettingsUpdated(settings.clone()))
            .unwrap();
    }

    pub fn chat_tab_closed(&self, normalized_name: &str) {
        self.server
            .blocking_send(AppMessageIn::UIChatClosed(normalized_name.to_owned()))
            .unwrap();
    }

    pub fn chat_tab_cleared(&self, normalized_name: &str) {
        self.server
            .blocking_send(AppMessageIn::UIChatCleared(normalized_name.to_owned()))
            .unwrap();
    }

    pub fn chat_message_sent(&self, target: &str, text: &str) {
        self.server
            .blocking_send(AppMessageIn::UIChatMessageSent {
                target: target.to_owned(),
                text: text.to_owned(),
            })
            .unwrap();
    }

    pub fn chat_action_sent(&self, target: &str, text: &str) {
        self.server
            .blocking_send(AppMessageIn::UIChatActionSent {
                target: target.to_owned(),
                text: text.to_owned(),
            })
            .unwrap();
    }

    pub fn connect_requested(&self) {
        self.server
            .blocking_send(AppMessageIn::UIConnectRequested)
            .unwrap();
    }

    pub fn disconnect_requested(&self) {
        self.server
            .blocking_send(AppMessageIn::UIDisconnectRequested)
            .unwrap();
    }

    pub fn exit_requested(&self) {
        self.server
            .blocking_send(AppMessageIn::UIExitRequested)
            .unwrap();
    }

    pub fn chat_switch_requested(&self, target: &str, message_id: usize) {
        self.server
            .blocking_send(AppMessageIn::UIChatSwitchRequested(
                target.to_owned(),
                message_id,
            ))
            .unwrap();
    }
}
