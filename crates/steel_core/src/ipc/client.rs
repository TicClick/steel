use std::error::Error;
use std::path::PathBuf;

use tokio::sync::mpsc::UnboundedSender;

use crate::ipc::server::AppMessageIn;
use crate::settings::application::AutoUpdate;
use crate::settings::Settings;

#[derive(Debug, Clone)]
pub struct CoreClient {
    server: UnboundedSender<AppMessageIn>,
}

impl CoreClient {
    pub fn new(server: UnboundedSender<AppMessageIn>) -> Self {
        Self { server }
    }
}

impl CoreClient {
    pub fn chat_opened(&self, chat: &str) {
        self.server
            .send(AppMessageIn::UIChatOpened(chat.to_owned()))
            .unwrap();
    }

    pub fn insert_user_mention(&self, username: &str) {
        self.server
            .send(AppMessageIn::UIUserMentionRequested(username.to_owned()))
            .unwrap();
    }

    pub fn push_ui_error(&self, error: Box<dyn Error + Sync + Send>, is_fatal: bool) {
        self.server
            .send(AppMessageIn::UIShowError { error, is_fatal })
            .unwrap();
    }

    pub fn update_window_title(&self) {
        self.server
            .send(AppMessageIn::UIWindowTitleUpdateRequested)
            .unwrap();
    }

    pub fn settings_requested(&self) {
        self.server.send(AppMessageIn::UISettingsRequested).unwrap();
    }

    pub fn settings_updated(&self, settings: &Settings) {
        self.server
            .send(AppMessageIn::UISettingsUpdated(Box::new(settings.clone())))
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

    pub fn restart_requested(&self, settings: Option<&Settings>, path: Option<PathBuf>) {
        if let Some(settings) = settings {
            self.server
                .send(AppMessageIn::UISettingsUpdated(Box::new(settings.clone())))
                .unwrap();
        }
        self.server
            .send(AppMessageIn::UIRestartRequested(path))
            .unwrap();
    }

    pub fn exit_requested(&self, settings: Option<&Settings>, return_code: i32) {
        if let Some(settings) = settings {
            self.server
                .send(AppMessageIn::UISettingsUpdated(Box::new(settings.clone())))
                .unwrap();
        }
        self.server
            .send(AppMessageIn::UIExitRequested(return_code))
            .unwrap();
    }

    pub fn chat_switch_requested(&self, target: &str, message_id: Option<usize>) {
        self.server
            .send(AppMessageIn::UIChatSwitchRequested(
                target.to_owned(),
                message_id,
            ))
            .unwrap();
    }

    pub fn chat_filter_requested(&self) {
        self.server
            .send(AppMessageIn::UIChatFilterRequested)
            .unwrap();
    }

    pub fn open_chat_log(&self, target: &str) {
        self.server
            .send(AppMessageIn::UIFilesystemPathRequested(target.to_owned()))
            .unwrap();
    }

    pub fn open_fs_path(&self, target: &str) {
        self.server
            .send(AppMessageIn::UIFilesystemPathRequested(target.to_owned()))
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

    pub fn glass_settings_requested(&self) {
        self.server
            .send(AppMessageIn::UIGlassSettingsRequested)
            .unwrap();
    }

    pub fn glass_settings_updated(&self, settings_yaml: String) {
        self.server
            .send(AppMessageIn::UIGlassSettingsUpdated(settings_yaml))
            .unwrap();
    }
}
