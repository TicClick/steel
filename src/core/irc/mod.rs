mod actor;
mod event_handler;
pub mod state;

#[cfg(test)]
mod actor_test;

use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use crate::actor::{Actor, ActorHandle};
use crate::core::chat_backend::ChatBackend;

use steel_core::chat::{ChatType, MessageType};
use steel_core::ipc::server::AppMessageIn;

#[derive(Debug)]
pub enum IRCMessageIn {
    Connect(Box<irc::client::data::Config>),
    Disconnect,
    JoinChannel(String),
    LeaveChannel(String),
    SendMessage {
        r#type: MessageType,
        destination: String,
        content: String,
    },
}

pub struct IRCActorHandle {
    actor: UnboundedSender<IRCMessageIn>,
}

impl ActorHandle for IRCActorHandle {}

impl ChatBackend for IRCActorHandle {
    fn connect(&self, settings: &steel_core::settings::chat::Chat) {
        let irc_config = &settings.irc;
        self.connect_irc(
            &irc_config.username,
            &irc_config.password,
            &irc_config.server,
            irc_config.ping_timeout,
        );
    }

    fn disconnect(&self) {
        self.disconnect_irc();
    }

    fn send_message(&self, destination: &str, _chat_type: ChatType, content: &str) {
        self.send_message(destination, content);
    }

    fn send_action(&self, destination: &str, _chat_type: ChatType, action: &str) {
        self.send_action(destination, action);
    }

    fn join_channel(&self, channel: &str) {
        self.join_channel(channel);
    }

    fn leave_channel(&self, channel: &str) {
        self.leave_channel(channel);
    }
}

impl IRCActorHandle {
    pub fn new(app_event_sender: UnboundedSender<AppMessageIn>) -> Self {
        let (irc_event_sender, irc_event_receiver) = unbounded_channel();
        let mut actor = actor::IRCActor::new(irc_event_receiver, app_event_sender);
        std::thread::spawn(move || {
            actor.run();
        });
        Self {
            actor: irc_event_sender,
        }
    }

    fn send_or_log(&self, message: IRCMessageIn) {
        if let Err(e) = self.actor.send(message) {
            log::error!("Failed to send IRC message: channel closed ({e})");
        }
    }

    pub fn connect_irc(&self, username: &str, password: &str, server: &str, ping_timeout: u32) {
        let config = irc::client::data::Config {
            username: Some(username.to_owned()),
            nickname: Some(username.to_owned()),
            password: Some(password.to_owned()),
            server: Some(server.to_owned()),
            port: Some(6667),
            nick_password: Some(password.to_owned()),
            realname: Some(username.to_owned()),
            use_tls: Some(false),
            ping_timeout: Some(ping_timeout),
            ping_time: Some(10),
            ..Default::default()
        };

        self.send_or_log(IRCMessageIn::Connect(Box::new(config)));
    }

    pub fn disconnect_irc(&self) {
        self.send_or_log(IRCMessageIn::Disconnect);
    }

    pub fn send_action(&self, destination: &str, action: &str) {
        self.send_or_log(IRCMessageIn::SendMessage {
            r#type: MessageType::Action,
            destination: destination.to_owned(),
            content: action.to_owned(),
        });
    }

    pub fn send_message(&self, destination: &str, content: &str) {
        self.send_or_log(IRCMessageIn::SendMessage {
            r#type: MessageType::Text,
            destination: destination.to_owned(),
            content: content.to_owned(),
        });
    }

    pub fn join_channel(&self, channel: &str) {
        self.send_or_log(IRCMessageIn::JoinChannel(channel.to_owned()));
    }

    pub fn leave_channel(&self, channel: &str) {
        self.send_or_log(IRCMessageIn::LeaveChannel(channel.to_owned()));
    }
}
