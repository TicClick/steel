use tokio::sync::mpsc::Sender;

use chrono::Utc;
use irc::client::prelude::*;

use crate::app::AppMessageIn;
use crate::core::chat;
use crate::core::irc::IRCError;

static ACTION_PREFIX: &str = "\x01ACTION";

pub fn empty_handler(_sender: &Sender<AppMessageIn>, _msg: irc::proto::Message) {}

pub fn privmsg_handler(sender: &Sender<AppMessageIn>, msg: irc::proto::Message) {
    if let irc::proto::Command::PRIVMSG(_, ref text) = msg.command {
        let (message_type, text) = if text.starts_with(ACTION_PREFIX) {
            (
                chat::MessageType::Action,
                text.strip_prefix(ACTION_PREFIX).unwrap().trim(),
            )
        } else {
            (chat::MessageType::Text, text.as_str())
        };
        sender
            .blocking_send(AppMessageIn::ChatMessageReceived {
                target: match msg.response_target() {
                    Some(target) => target.to_owned(),
                    None => "(unknown target)".to_owned(),
                },
                message: chat::Message {
                    time: Utc::now(),
                    text: text.to_owned(),
                    r#type: message_type,
                    username: match msg.source_nickname() {
                        Some(nickname) => nickname.to_string(),
                        None => "(unknown sender)".to_owned(),
                    },
                },
            })
            .unwrap();
    }
}

pub fn motd_handler(sender: &Sender<AppMessageIn>, msg: irc::proto::Message) {
    if let irc::proto::Command::Response(_, args) = msg.command {
        sender
            .blocking_send(AppMessageIn::ServerMessageReceived {
                content: args[1..].join(" "),
            })
            .unwrap();
    }
}

pub fn default_handler(sender: &Sender<AppMessageIn>, msg: irc::proto::Message) {
    if let irc::proto::Command::Response(r, ref args) = msg.command {
        if r.is_error() {
            let error = match r {
                Response::ERR_PASSWDMISMATCH => {
                    IRCError::FatalError("either your password is invalid, or you need to wait out some time before trying to connect again".to_owned())
                }
                _ => {
                    IRCError::ServerError {
                        code: r,
                        content: args[1..].join(" "),
                    }
                }
            };
            sender
                .blocking_send(AppMessageIn::ChatError(error))
                .unwrap();
        } else {
            debug_handler(sender, msg);
        }
    }
}

pub fn debug_handler(_sender: &Sender<AppMessageIn>, msg: irc::proto::Message) {
    println!("message without handler: {:?}", msg);
}

pub fn join_handler(sender: &Sender<AppMessageIn>, channel: String) {
    sender
        .blocking_send(AppMessageIn::ChannelJoined(channel))
        .unwrap();
}

pub fn dispatch_message(
    sender: &Sender<AppMessageIn>,
    msg: irc_proto::Message,
    own_username: &str,
) {
    match msg.command {
        Command::PRIVMSG(..) => self::privmsg_handler(sender, msg),

        Command::JOIN(channel, ..) => {
            if let Some(Prefix::Nickname(username, ..)) = msg.prefix {
                if username == own_username {
                    self::join_handler(sender, channel);
                }
            }
        }

        // junk that needs to be ignored
        Command::PART(..) |
        Command::QUIT(..) |
        Command::Response(
            Response::RPL_TOPIC |  // channel topic
            Response::RPL_TOPICWHOTIME |  // channel topic author/mtime
            Response::RPL_NAMREPLY |  // channel users
            Response::RPL_ENDOFNAMES,  // channel users
            ..
        ) |
        // PING and PONG are handled by the library
        Command::PING(..) |
        Command::PONG(..) |
        Command::ChannelMODE(..) => self::empty_handler(sender, msg),

        Command::Response(
            Response::RPL_WELCOME |
            Response::RPL_MOTD |
            Response::RPL_MOTDSTART |
            Response::RPL_ENDOFMOTD,
            ..
        ) => self::motd_handler(sender, msg),

        Command::Response(..) => self::default_handler(sender, msg),
        _ => self::debug_handler(sender, msg),
    }
}
