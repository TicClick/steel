mod actor;
mod event_handler;

use std::fmt;

use tokio::sync::mpsc::{channel, Sender};

use crate::actor::{Actor, ActorHandle};
use crate::app::AppMessageIn;
use crate::core::chat;

#[derive(Debug)]
pub enum IRCMessageIn {
    Connect(Box<irc::client::data::Config>),
    Disconnect,
    JoinChannel(String),
    LeaveChannel(String),
    SendMessage {
        r#type: chat::MessageType,
        destination: String,
        content: String,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum IRCError {
    #[error("fatal IRC error: {0}")]
    FatalError(String),
    #[error("IRC error {code:?}: {content}")]
    ServerError {
        code: irc_proto::Response,
        content: String,
    },
}

#[derive(Clone, Copy, Debug, Default)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    InProgress,
    Connected,
}

impl fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Connected => "connected",
                Self::InProgress => "connecting",
                Self::Disconnected => "disconnected",
            }
        )
    }
}

pub struct IRCActorHandle {
    actor: Sender<IRCMessageIn>,
}

impl ActorHandle for IRCActorHandle {}

const IRC_EVENT_QUEUE_SIZE: usize = 1000;

impl IRCActorHandle {
    pub fn new(app_event_sender: Sender<AppMessageIn>) -> Self {
        let (irc_event_sender, irc_event_receiver) = channel(IRC_EVENT_QUEUE_SIZE);
        let mut actor = actor::IRCActor::new(irc_event_receiver, app_event_sender);
        std::thread::spawn(move || {
            actor.run();
        });
        Self {
            actor: irc_event_sender,
        }
    }

    pub fn connect(&self, username: &str, password: &str) {
        let config = irc::client::data::Config {
            username: Some(username.to_owned()),
            nickname: Some(username.to_owned()),
            password: Some(password.to_owned()),
            server: Some("irc.ppy.sh".to_owned()),
            port: Some(6667),
            nick_password: Some(password.to_owned()),
            realname: Some(username.to_owned()),
            use_tls: Some(false),
            ..Default::default()
        };

        self.actor
            .blocking_send(IRCMessageIn::Connect(Box::new(config)))
            .expect("failed to queue chat connection");
    }

    pub fn disconnect(&self) {
        self.actor
            .blocking_send(IRCMessageIn::Disconnect)
            .expect("failed to queue disconnecting from chat");
    }

    pub fn send_action(&self, destination: &str, action: &str) {
        self.actor
            .blocking_send(IRCMessageIn::SendMessage {
                r#type: chat::MessageType::Action,
                destination: destination.to_owned(),
                content: action.to_owned(),
            })
            .expect("failed to queue a chat action");
    }

    pub fn send_message(&self, destination: &str, content: &str) {
        self.actor
            .blocking_send(IRCMessageIn::SendMessage {
                r#type: chat::MessageType::Text,
                destination: destination.to_owned(),
                content: content.to_owned(),
            })
            .expect("failed to queue a chat message")
    }

    pub fn join_channel(&self, channel: &str) {
        self.actor
            .blocking_send(IRCMessageIn::JoinChannel(channel.to_owned()))
            .expect("failed to queue joining a channel");
    }

    pub fn leave_channel(&self, channel: &str) {
        self.actor
            .blocking_send(IRCMessageIn::LeaveChannel(channel.to_owned()))
            .expect("failed to queue leaving a channel");
    }
}
