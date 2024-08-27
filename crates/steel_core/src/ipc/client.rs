use tokio::sync::mpsc::UnboundedSender;

use crate::ipc::server::AppMessageIn;
use crate::settings::application::AutoUpdate;
use crate::settings::Settings;

#[derive(Debug)]
pub struct CoreClient {
    server: UnboundedSender<AppMessageIn>,
}

impl CoreClient {
    pub fn new(server: UnboundedSender<AppMessageIn>) -> Self {
        Self { server }
    }
}

impl CoreClient {
    pub fn channel_opened(&self, channel: &str) {
        self.server
            .send(AppMessageIn::UIChannelOpened(channel.to_owned()))
            .unwrap();
    }

    pub fn private_chat_opened(&self, chat: &str) {
        self.server
            .send(AppMessageIn::UIPrivateChatOpened(chat.to_owned()))
            .unwrap();
    }

    pub fn channel_join_requested(&self, channel: &str) {
        self.server
            .send(AppMessageIn::UIChannelJoinRequested(channel.to_owned()))
            .unwrap();
    }

    pub fn settings_requested(&self) {
        self.server.send(AppMessageIn::UISettingsRequested).unwrap();
    }

    pub fn settings_updated(&self, settings: &Settings) {
        self.server
            .send(AppMessageIn::UISettingsUpdated(settings.clone()))
            .unwrap();
    }

    pub fn chat_tab_closed(&self, normalized_name: &str) {
        self.server
            .send(AppMessageIn::UIChatClosed(normalized_name.to_owned()))
            .unwrap();
    }

    pub fn chat_tab_cleared(&self, normalized_name: &str) {
        self.server
            .send(AppMessageIn::UIChatCleared(normalized_name.to_owned()))
            .unwrap();
    }

    pub fn chat_message_sent(&self, target: &str, text: &str) {
        self.server
            .send(AppMessageIn::UIChatMessageSent {
                target: target.to_owned(),
                text: text.to_owned(),
            })
            .unwrap();
    }

    pub fn chat_action_sent(&self, target: &str, text: &str) {
        self.server
            .send(AppMessageIn::UIChatActionSent {
                target: target.to_owned(),
                text: text.to_owned(),
            })
            .unwrap();
    }

    pub fn connect_requested(&self) {
        self.server.send(AppMessageIn::UIConnectRequested).unwrap();
    }

    pub fn disconnect_requested(&self) {
        self.server
            .send(AppMessageIn::UIDisconnectRequested)
            .unwrap();
    }

    pub fn exit_requested(&self) {
        self.server.send(AppMessageIn::UIExitRequested).unwrap();
    }

    pub fn chat_switch_requested(&self, target: &str, message_id: Option<usize>) {
        self.server
            .send(AppMessageIn::UIChatSwitchRequested(
                target.to_owned(),
                message_id,
            ))
            .unwrap();
    }

    pub fn usage_window_requested(&self) {
        self.server
            .send(AppMessageIn::UIUsageWindowRequested)
            .unwrap();
    }

    pub fn update_settings_changed(&self, s: &AutoUpdate) {
        self.server
            .send(AppMessageIn::UpdateSettingsChanged(s.clone()))
            .unwrap();
    }

    pub fn check_application_updates(&self) {
        self.server
            .send(AppMessageIn::CheckApplicationUpdates)
            .unwrap();
    }

    pub fn download_application_update(&self) {
        self.server
            .send(AppMessageIn::DownloadApplicationUpdate)
            .unwrap();
    }

    pub fn abort_application_update(&self) {
        self.server
            .send(AppMessageIn::AbortApplicationUpdate)
            .unwrap();
    }
}
