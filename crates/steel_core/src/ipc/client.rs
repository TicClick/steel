use std::error::Error;
use std::path::PathBuf;

use tokio::sync::mpsc::UnboundedSender;

use crate::chat::ChatType;
use crate::ipc::error::{IpcError, IpcResult};
use crate::ipc::server::AppMessageIn;
use crate::ipc::server::UICommand;
use crate::ipc::server::UpdateEvent;
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

    fn try_send(&self, message: AppMessageIn) -> IpcResult<()> {
        self.server
            .send(message)
            .map_err(|_| IpcError::ChannelClosed {
                context: "Application channel closed".into(),
            })
    }

    fn send_or_log(&self, message: AppMessageIn) {
        if let Err(e) = self.try_send(message) {
            log::error!("Failed to send IPC message: {e}");
        }
    }
}

impl CoreClient {
    pub fn chat_opened(&self, chat: &str, chat_type: ChatType) {
        self.send_or_log(AppMessageIn::UI(UICommand::ChatOpened(
            chat.to_owned(),
            chat_type,
        )));
    }

    pub fn insert_user_mention(&self, username: &str) {
        self.send_or_log(AppMessageIn::UI(UICommand::UserMentionRequested(
            username.to_owned(),
        )));
    }

    pub fn push_ui_error(&self, error: Box<dyn Error + Sync + Send>, is_fatal: bool) {
        self.send_or_log(AppMessageIn::UI(UICommand::ShowError { error, is_fatal }));
    }

    pub fn update_window_title(&self) {
        self.send_or_log(AppMessageIn::UI(UICommand::WindowTitleUpdateRequested));
    }

    pub fn settings_requested(&self) {
        self.send_or_log(AppMessageIn::UI(UICommand::SettingsRequested));
    }

    pub fn settings_updated(&self, settings: &Settings) {
        self.send_or_log(AppMessageIn::UI(UICommand::SettingsUpdated(Box::new(
            settings.clone(),
        ))));
    }

    pub fn chat_tab_closed(&self, normalized_name: &str, chat_type: ChatType) {
        self.send_or_log(AppMessageIn::UI(UICommand::ChatClosed(
            normalized_name.to_owned(),
            chat_type,
        )));
    }

    pub fn chat_tab_cleared(&self, normalized_name: &str, chat_type: ChatType) {
        self.send_or_log(AppMessageIn::UI(UICommand::ChatCleared(
            normalized_name.to_owned(),
            chat_type,
        )));
    }

    pub fn chat_message_sent(&self, target: &str, chat_type: ChatType, text: &str) {
        self.send_or_log(AppMessageIn::UI(UICommand::ChatMessageSent {
            target: target.to_owned(),
            chat_type,
            text: text.to_owned(),
        }));
    }

    pub fn chat_action_sent(&self, target: &str, chat_type: ChatType, text: &str) {
        self.send_or_log(AppMessageIn::UI(UICommand::ChatActionSent {
            target: target.to_owned(),
            chat_type,
            text: text.to_owned(),
        }));
    }

    pub fn connect_requested(&self) {
        self.send_or_log(AppMessageIn::UI(UICommand::ConnectRequested));
    }

    pub fn disconnect_requested(&self) {
        self.send_or_log(AppMessageIn::UI(UICommand::DisconnectRequested));
    }

    pub fn restart_requested(&self, settings: Option<&Settings>, path: Option<PathBuf>) {
        if let Some(settings) = settings {
            self.send_or_log(AppMessageIn::UI(UICommand::SettingsUpdated(Box::new(
                settings.clone(),
            ))));
        }
        self.send_or_log(AppMessageIn::UI(UICommand::RestartRequested(path)));
    }

    pub fn exit_requested(&self, settings: Option<&Settings>, return_code: i32) {
        if let Some(settings) = settings {
            self.send_or_log(AppMessageIn::UI(UICommand::SettingsUpdated(Box::new(
                settings.clone(),
            ))));
        }
        self.send_or_log(AppMessageIn::UI(UICommand::ExitRequested(return_code)));
    }

    pub fn chat_switch_requested(
        &self,
        target: &str,
        chat_type: ChatType,
        message_id: Option<usize>,
    ) {
        self.send_or_log(AppMessageIn::UI(UICommand::ChatSwitchRequested(
            target.to_owned(),
            chat_type,
            message_id,
        )));
    }

    pub fn chat_filter_requested(&self) {
        self.send_or_log(AppMessageIn::UI(UICommand::ChatFilterRequested));
    }

    pub fn open_chat_log(&self, target: &str) {
        self.send_or_log(AppMessageIn::UI(UICommand::FilesystemPathRequested(
            target.to_owned(),
        )));
    }

    pub fn open_fs_path(&self, target: &str) {
        self.send_or_log(AppMessageIn::UI(UICommand::FilesystemPathRequested(
            target.to_owned(),
        )));
    }

    pub fn usage_window_requested(&self) {
        self.send_or_log(AppMessageIn::UI(UICommand::UsageWindowRequested));
    }

    pub fn update_settings_changed(&self, s: &AutoUpdate) {
        self.send_or_log(AppMessageIn::Update(UpdateEvent::SettingsChanged(
            s.clone(),
        )));
    }

    pub fn check_application_updates(&self) {
        self.send_or_log(AppMessageIn::Update(UpdateEvent::CheckRequested));
    }

    pub fn download_application_update(&self) {
        self.send_or_log(AppMessageIn::Update(UpdateEvent::DownloadRequested));
    }

    pub fn abort_application_update(&self) {
        self.send_or_log(AppMessageIn::Update(UpdateEvent::AbortRequested));
    }

    pub fn glass_settings_requested(&self) {
        self.send_or_log(AppMessageIn::UI(UICommand::GlassSettingsRequested));
    }

    pub fn glass_settings_updated(&self, settings_yaml: String) {
        self.send_or_log(AppMessageIn::UI(UICommand::GlassSettingsUpdated(
            settings_yaml,
        )));
    }

    pub fn report_dialog_requested(&self, username: &str, chat_name: &str) {
        self.send_or_log(AppMessageIn::UI(UICommand::ReportDialogRequested {
            username: username.to_owned(),
            chat_name: chat_name.to_owned(),
        }));
    }
}

pub use crate::ipc::server::{ChatEvent, HTTPEvent, SystemEvent};
