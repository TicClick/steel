use tokio::sync::mpsc::UnboundedSender;

use irc::client::prelude::*;
use irc_proto::mode::{ChannelMode, Mode};

use steel_core::chat::irc::IRCError;
use steel_core::chat::{ChatType, Message, MessageType};
use steel_core::ipc::server::AppMessageIn;

static ACTION_PREFIX: &str = "\x01ACTION";
static ACTION_SUFFIX: &str = "\x01";

fn skip_and_join(v: &[String], n: usize) -> String {
    if v.len() > n {
        v[n..].join(" ")
    } else {
        String::new()
    }
}

pub fn empty_handler(_sender: &UnboundedSender<AppMessageIn>, _msg: irc::proto::Message) {}

pub fn privmsg_handler(sender: &UnboundedSender<AppMessageIn>, msg: irc::proto::Message) {
    if let irc::proto::Command::PRIVMSG(_, ref text) = msg.command {
        let (message_type, text) =
            if let Some(text_without_prefix) = text.strip_prefix(ACTION_PREFIX) {
                let action_text = match text_without_prefix.strip_suffix(ACTION_SUFFIX) {
                    Some(clean_action) => clean_action,
                    None => text_without_prefix,
                };
                (MessageType::Action, action_text.trim())
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
            .send(AppMessageIn::chat_message_received(
                message_target,
                Message::new(&username, text, message_type),
            ))
            .unwrap_or_else(|e| log::error!("Failed to send message: {e}"));
    }
}

pub fn motd_handler(sender: &UnboundedSender<AppMessageIn>, msg: irc::proto::Message) {
    if let irc::proto::Command::Response(_, args) = msg.command {
        sender
            .send(AppMessageIn::server_message_received(skip_and_join(
                &args, 1,
            )))
            .unwrap_or_else(|e| log::error!("Failed to send server message: {e}"));
    }
}

pub fn default_handler(sender: &UnboundedSender<AppMessageIn>, msg: irc::proto::Message) {
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
                        content: skip_and_join(args, 2),
                    }
                }
                _ => {
                    IRCError::ServerError {
                        code: r,
                        chat: None,
                        content: skip_and_join(args, 1),
                    }
                }
            };
            sender
                .send(AppMessageIn::chat_error(error))
                .unwrap_or_else(|e| log::error!("Failed to send chat error: {e}"));
        } else {
            debug_handler(sender, msg);
        }
    }
}

pub fn debug_handler(_sender: &UnboundedSender<AppMessageIn>, msg: irc::proto::Message) {
    println!("message without handler: {msg:?}");
}

pub fn join_handler(sender: &UnboundedSender<AppMessageIn>, channel: String) {
    sender
        .send(AppMessageIn::channel_joined(channel, ChatType::Channel))
        .unwrap_or_else(|e| log::error!("Failed to send channel join: {e}"));
}

pub fn dispatch_message(
    sender: &UnboundedSender<AppMessageIn>,
    msg: irc_proto::Message,
    own_username: &str,
) {
    sender
        .send(AppMessageIn::connection_activity())
        .unwrap_or_else(|e| log::error!("Failed to send activity: {e}"));

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
                    sender
                        .send(AppMessageIn::moderator_added(user))
                        .unwrap_or_else(|e| log::error!("Failed to send moderator: {e}"));
                }
            }
        }

        Command::Response(Response::RPL_NAMREPLY, cmd) => {
            if let Some(users) = cmd.get(3) {
                for user in users.split_ascii_whitespace().filter(|u| u.starts_with('@')) {
                    sender
                        .send(AppMessageIn::moderator_added(user[1..].to_owned()))
                        .unwrap_or_else(|e| log::error!("Failed to send moderator: {e}"));
                }
            }
        }

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
