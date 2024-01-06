use tokio::sync::mpsc::Sender;

use irc::client::prelude::*;
use irc_proto::mode::{ChannelMode, Mode};

use steel_core::chat::irc::IRCError;
use steel_core::chat::{Message, MessageType};
use steel_core::ipc::server::AppMessageIn;

static ACTION_PREFIX: &str = "\x01ACTION";

pub fn empty_handler(_sender: &Sender<AppMessageIn>, _msg: irc::proto::Message) {}

pub fn privmsg_handler(sender: &Sender<AppMessageIn>, msg: irc::proto::Message) {
    if let irc::proto::Command::PRIVMSG(_, ref text) = msg.command {
        let (message_type, text) = if text.starts_with(ACTION_PREFIX) {
            (
                MessageType::Action,
                text.strip_prefix(ACTION_PREFIX).unwrap().trim(),
            )
        } else {
            (MessageType::Text, text.as_str())
        };

        let message_target = match msg.response_target() {
            Some(target) => target.to_owned(),
            None => "(unknown target)".to_owned(),
        };
        let username = match msg.source_nickname() {
            Some(nickname) => nickname.to_string(),
            None => "(unknown sender)".to_owned(),
        };
        sender
            .blocking_send(AppMessageIn::ChatMessageReceived {
                target: message_target,
                message: Message::new(&username, text, message_type),
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
                Response::ERR_NOSUCHCHANNEL | Response::ERR_NOSUCHNICK | Response::ERR_CANNOTSENDTOCHAN | Response::ERR_WASNOSUCHNICK | Response::ERR_NOTONCHANNEL => {
                    IRCError::ServerError {
                        code: r,
                        chat: Some(args[1].to_owned()),
                        content: args[2..].join(" "),
                    }
                }
                _ => {
                    IRCError::ServerError {
                        code: r,
                        chat: None,
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
            Response::RPL_ENDOFNAMES,  // channel users
            ..
        ) |
        // PING and PONG are handled by the library
        Command::PING(..) |
        Command::PONG(..) => self::empty_handler(sender, msg),
        Command::ChannelMODE(_, modes) => {
            for m in modes {
                if let Mode::Plus(ChannelMode::Oper, Some(user)) = m {
                    sender.blocking_send(AppMessageIn::ChatModeratorAdded(user)).unwrap();
                }
            }
        }

        Command::Response(
            Response::RPL_NAMREPLY,
            cmd
        ) => {
            if let Some(users) = cmd.get(3) {
                for user in users.split_ascii_whitespace().filter(|u| u.starts_with('@')) {
                    sender.blocking_send(AppMessageIn::ChatModeratorAdded(user[1..].to_owned())).unwrap();
                }
            }
        },

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
