use std::collections::BTreeSet;

use chrono::DurationRound;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use steel_core::chat::irc::IRCError;
use steel_core::chat::{ChatLike, ChatState, ConnectionStatus, Message};

use crate::core::irc::IRCActorHandle;
use crate::core::settings;
use steel_core::ipc::{server::AppMessageIn, ui::UIMessageIn};

const EVENT_QUEUE_SIZE: usize = 1000;

#[derive(Clone, Default)]
pub struct ApplicationState {
    pub settings: settings::Settings,
    pub chats: BTreeSet<String>,
    pub connection: ConnectionStatus,
}

pub struct Application {
    state: ApplicationState,
    events: Receiver<AppMessageIn>,

    irc: IRCActorHandle,
    ui_queue: Sender<UIMessageIn>,
    pub app_queue: Sender<AppMessageIn>,

    date_announcer: Option<std::thread::JoinHandle<()>>,
}

impl Application {
    pub fn new(ui_queue: Sender<UIMessageIn>) -> Self {
        let (app_queue, events) = channel(EVENT_QUEUE_SIZE);
        Self {
            state: ApplicationState::default(),
            events,
            irc: IRCActorHandle::new(app_queue.clone()),
            ui_queue,
            app_queue,
            date_announcer: None,
        }
    }

    pub fn run(&mut self) {
        while let Some(event) = self.events.blocking_recv() {
            match event {
                AppMessageIn::ConnectionChanged(status) => {
                    self.handle_connection_status(status);
                }
                AppMessageIn::ChatMessageReceived { target, message } => {
                    self.handle_chat_message(target, message, false);
                }
                AppMessageIn::ServerMessageReceived { content } => {
                    self.handle_server_message(content);
                }
                AppMessageIn::ChatError(e) => {
                    self.handle_chat_error(e);
                }
                AppMessageIn::ChannelJoined(channel) => {
                    self.handle_channel_join(channel);
                }

                AppMessageIn::UIConnectRequested => {
                    self.connect();
                }
                AppMessageIn::UIDisconnectRequested => {
                    self.disconnect();
                }
                AppMessageIn::UIExitRequested => {
                    break;
                }
                AppMessageIn::UIChannelOpened(target)
                | AppMessageIn::UIPrivateChatOpened(target) => {
                    self.maybe_remember_chat(&target, true);
                }
                AppMessageIn::UIChatSwitchRequested(target, id) => {
                    self.ui_handle_chat_switch_requested(target, id);
                }
                AppMessageIn::UIChannelJoinRequested(channel) => {
                    self.handle_ui_channel_join_requested(channel);
                }
                AppMessageIn::UIChatClosed(target) => {
                    self.ui_handle_close_chat(&target);
                }
                AppMessageIn::UIChatMessageSent { target, text } => {
                    self.send_text_message(&target, &text);
                }
                AppMessageIn::UISettingsRequested => {
                    self.ui_handle_settings_requested();
                }
                AppMessageIn::UISettingsUpdated(settings) => {
                    self.ui_handle_settings_updated(settings);
                }
            }
        }
    }
}

impl Application {
    pub fn handle_ui_channel_join_requested(&mut self, channel: String) {
        self.maybe_remember_chat(&channel, true);
        self.join_channel(&channel);
    }

    pub fn ui_handle_chat_switch_requested(&self, chat: String, message_id: usize) {
        self.ui_queue
            .blocking_send(UIMessageIn::ChatSwitchRequested(chat, message_id))
            .unwrap();
    }

    pub fn initialize(&mut self) {
        self.load_settings(settings::Source::DefaultPath, true);
        log::set_max_level(self.state.settings.journal.app_events.level);

        let ui_queue = self.ui_queue.clone();
        self.date_announcer = Some(std::thread::spawn(|| date_announcer(ui_queue)));

        if self.state.settings.chat.autoconnect {
            self.connect();
        }
    }

    pub fn load_settings(&mut self, source: settings::Source, fallback: bool) {
        self.state.settings = settings::Settings::from_file(&source, fallback);
        self.setup_mod_users();
        self.ui_handle_settings_requested();
    }

    // TODO: remove when osu!api integration is ready
    // Snapshots taken on 2023-05-26
    #[rustfmt::skip]
    fn setup_mod_users(&mut self) {
        let mods = &mut self.state.settings.ui.colours_mut().mod_users;
        for group in [
            // GMT -- https://osu.ppy.sh/groups/4
            ["- Felix", "[ Another ]", "[ryuu]", "0x84f", "abraker", "Albionthegreat", "Azer", "bibitaru", "Burak", "ChillierPear", "chromb", "Civil oath", "Corne2Plum3", "D I O", "Death", "Dntm8kmeeatu", "Edward", "Ephemeral", "FAMoss", "Flutteh", "Galkan", "Ganondorf", "Garden", "Halfslashed", "Imakuri", "JBHyperion", "Jerry", "kanpakyin", "Kobold84", "KSHR", "Kudou Chitose", "Kyubey", "Laurakko", "LeoFLT", "Loctav", "Maot", "MillhioreF", "My Angel Chino", "Nathanael", "Nevo", "Niva", "Nozhomi", "Okoratu", "OnosakiHito", "Osu Tatakae Ouendan", "osu!team", "p3n", "Petal", "Petit", "Protastic101", "QHideaki13", "Redo_", "Repflez", "Roan", "RockRoller", "ruexia", "S o h", "Saten", "Shiro", "Shurelia", "Sies", "spboxer3", "terho", "THAT_otaku", "TicClick", "TKS", "ToGlette", "topecnz", "Trigonoculus", "Trosk-", "Uni", "Venix", "Yason", "Zallius",]
                .as_slice(),
            // NAT -- https://osu.ppy.sh/groups/7
            ["_Stan", "-Mo-", "achyoo", "Agatsu", "AirinCat", "Akasha-", "Antalf", "Capu", "Chaoslitz", "Dada", "Deif", "elicz1", "Enneya", "Firika", "FuJu", "Greaper", "Hivie", "Ideal", "Maxus", "Naxess", "nexusqi", "Nomination Assessment Team", "pishifat", "radar", "StarCastler", "Stixy", "yaspo", "Yogurtt", "Zelq",]
                .as_slice(),
            // DEV -- https://osu.ppy.sh/groups/11
            ["Damnae", "Domco", "Ephemeral", "flyte", "MillhioreF", "nanaya", "nekodex", "notbakaneko", "osu!team", "peppy", "RBRat3", "smoogipoo", "ThePooN", "Tom94",]
                .as_slice(),
        ] {
            mods.extend(group.iter().map(|u| u.to_lowercase().replace(' ', "_")))
        }
        mods.insert("banchobot".into());
    }

    pub fn ui_handle_settings_requested(&self) {
        self.ui_queue
            .blocking_send(UIMessageIn::SettingsChanged(self.state.settings.clone()))
            .unwrap();
    }

    pub fn ui_handle_settings_updated(&mut self, settings: settings::Settings) {
        self.state.settings = settings;
        self.state.settings.save();
    }

    pub fn handle_connection_status(&mut self, status: ConnectionStatus) {
        self.state.connection = status;
        self.ui_queue
            .blocking_send(UIMessageIn::ConnectionStatusChanged(status))
            .unwrap();
        log::debug!("IRC connection status changed to {:?}", status);
        match status {
            ConnectionStatus::Connected => {
                let mut chats = self.state.settings.chat.autojoin.clone();
                chats.append(&mut self.state.chats.iter().cloned().collect());

                for chat in chats {
                    self.maybe_remember_chat(&chat, false);
                    if chat.is_channel() {
                        self.join_channel(&chat);
                    }
                }
            }
            ConnectionStatus::InProgress | ConnectionStatus::Scheduled(_) => (),
            ConnectionStatus::Disconnected { by_user } => {
                if self.state.settings.chat.reconnect && !by_user {
                    self.queue_reconnect();
                }
            }
        }
    }

    fn queue_reconnect(&self) {
        let queue = self.app_queue.clone();
        let delta = chrono::Duration::seconds(15);
        let reconnect_time = chrono::Local::now() + delta;
        self.ui_queue
            .blocking_send(UIMessageIn::ConnectionStatusChanged(
                ConnectionStatus::Scheduled(reconnect_time),
            ))
            .unwrap();

        std::thread::spawn(move || {
            std::thread::sleep(delta.to_std().unwrap());
            queue
                .blocking_send(AppMessageIn::UIConnectRequested)
                .expect("failed to trigger reconnection");
        });
    }

    fn push_chat_to_ui(&self, target: &str, switch: bool) {
        let chat_state = if target.is_channel() {
            ChatState::JoinInProgress
        } else {
            ChatState::Joined
        };
        self.ui_queue
            .blocking_send(UIMessageIn::NewChatRequested(
                target.to_owned(),
                chat_state,
                switch,
            ))
            .unwrap();
    }

    pub fn handle_chat_message(
        &mut self,
        target: String,
        message: Message,
        switch_if_missing: bool,
    ) {
        self.maybe_remember_chat(&target, switch_if_missing);
        self.ui_queue
            .blocking_send(UIMessageIn::NewMessageReceived { target, message })
            .unwrap();
    }

    fn maybe_remember_chat(&mut self, target: &str, switch_if_missing: bool) {
        let normalized = target.to_lowercase();
        if !self.state.chats.contains(&normalized) {
            self.state.chats.insert(normalized);
            self.push_chat_to_ui(target, switch_if_missing);
        }
    }

    pub fn handle_server_message(&mut self, content: String) {
        log::debug!("IRC server message: {}", content);
        self.ui_queue
            .blocking_send(UIMessageIn::NewServerMessageReceived(content))
            .unwrap();
    }

    pub fn handle_chat_error(&mut self, e: IRCError) {
        log::error!("IRC chat error: {:?}", e);
        if matches!(e, IRCError::FatalError(_)) {
            self.irc.disconnect();
        }

        let error_text = e.to_string();
        if let IRCError::ServerError {
            code: _,
            chat: Some(chat),
            content,
        } = e
        {
            let normalized = chat.to_lowercase();
            self.state.chats.remove(&normalized);
            self.ui_queue
                .blocking_send(UIMessageIn::NewChatStatusReceived {
                    target: chat,
                    state: ChatState::Left,
                    details: content,
                })
                .unwrap();
        }
        self.ui_queue
            .blocking_send(UIMessageIn::NewServerMessageReceived(error_text))
            .unwrap();
    }

    pub fn handle_channel_join(&mut self, channel: String) {
        self.ui_queue
            .blocking_send(UIMessageIn::ChannelJoined(channel))
            .unwrap();
    }

    pub fn connect(&mut self) {
        match self.state.connection {
            ConnectionStatus::Connected | ConnectionStatus::InProgress => {}
            ConnectionStatus::Disconnected { .. } | ConnectionStatus::Scheduled(_) => {
                let irc_config = self.state.settings.chat.irc.clone();
                self.irc.connect(&irc_config.username, &irc_config.password);
            }
        }
    }

    pub fn disconnect(&mut self) {
        if !matches!(self.state.connection, ConnectionStatus::Connected) {
            return;
        }
        self.irc.disconnect();
    }

    pub fn ui_handle_close_chat(&mut self, name: &str) {
        let normalized = name.to_lowercase();
        self.state.chats.remove(&normalized);
        if name.is_channel() {
            self.leave_channel(name);
        }
        self.ui_queue
            .blocking_send(UIMessageIn::ChatClosed(name.to_owned()))
            .unwrap();
    }

    pub fn send_text_message(&mut self, target: &str, text: &str) {
        self.irc.send_message(target, text);
        let message = Message::new_text(&self.state.settings.chat.irc.username, text);
        self.ui_queue
            .blocking_send(UIMessageIn::NewMessageReceived {
                target: target.to_owned(),
                message,
            })
            .unwrap();
    }

    pub fn join_channel(&self, channel: &str) {
        self.irc.join_channel(channel);
    }

    pub fn leave_channel(&self, channel: &str) {
        self.irc.leave_channel(channel);
    }
}

fn date_announcer(sender: Sender<UIMessageIn>) {
    loop {
        let now = chrono::Local::now();
        let midnight = now
            .duration_trunc(chrono::Duration::days(1))
            .unwrap()
            .checked_add_days(chrono::Days::new(1))
            .unwrap();
        let delta = midnight - now;
        if delta.num_seconds() > 0 {
            std::thread::sleep(delta.to_std().unwrap());
        }

        sender
            .blocking_send(UIMessageIn::DateChanged(midnight))
            .unwrap();
        std::thread::sleep(std::time::Duration::from_secs(30 * 60));
    }
}
