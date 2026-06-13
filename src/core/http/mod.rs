use steel_core::chat::{ChatType, MessageType};
use steel_core::ipc::server::AppMessageIn;
use steel_core::settings::chat::{default_api_client_id, OAuthMode};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use crate::actor::{Actor, ActorHandle};
use crate::core::chat_backend::ChatBackend;

pub mod actor;
pub mod api;
pub mod jwt;
pub mod oauth_flow;
pub mod oauth_listener;
pub mod token_manager;
pub mod token_refresh;
pub mod token_storage;
pub mod websocket;

#[derive(Debug, Clone)]
pub struct APISettings {
    pub oauth_mode: OAuthMode,
    pub client_id: u64,
    pub client_secret: String,
    pub redirect_uri: String,
    pub jump_server_url: String,
    pub ws_base_uri: String,
    pub token_rotation_days: u32,
}

pub fn send_progress(output: &UnboundedSender<AppMessageIn>, text: &str) {
    log::info!("Connection progress: {text}");
    output
        .send(AppMessageIn::connection_progress(text))
        .unwrap_or_else(|e| log::error!("Failed to send connection progress: {e}"));
}

impl APISettings {
    pub fn token_refresh_config(&self) -> token_refresh::TokenRefreshConfig {
        token_refresh::TokenRefreshConfig {
            oauth_mode: self.oauth_mode.clone(),
            client_id: self.client_id,
            client_secret: self.client_secret.clone(),
            jump_server_url: self.jump_server_url.clone(),
        }
    }
}

#[derive(Debug)]
pub enum HTTPMessageIn {
    Connect(APISettings),
    Disconnect,
    JoinChannel(String),
    LeaveChannel(String),
    SendMessage {
        r#type: MessageType,
        destination: String,
        chat_type: ChatType,
        content: String,
    },
}

#[derive(Debug)]
pub enum HTTPMessageOut {
    Connect,
    Disconnect,
}

pub struct HTTPActorHandle {
    actor: UnboundedSender<HTTPMessageIn>,
}

impl ActorHandle for HTTPActorHandle {}

impl ChatBackend for HTTPActorHandle {
    fn connect(&self, settings: &steel_core::settings::chat::Chat) {
        let api_config = &settings.api;
        let client_id = api_config.client_id.parse::<u64>().unwrap_or_else(|_| {
            log::error!("Invalid client_id, using default");
            default_api_client_id()
        });
        let api_settings = APISettings {
            oauth_mode: api_config.oauth_mode.clone(),
            client_id,
            client_secret: api_config.client_secret.clone(),
            redirect_uri: api_config.redirect_uri.clone(),
            jump_server_url: api_config.jump_server_url.clone(),
            ws_base_uri: api_config.ws_base_uri.clone(),
            token_rotation_days: api_config.token_rotation_days,
        };

        self.connect_http(api_settings);
    }

    fn disconnect(&self) {
        self.disconnect_http();
    }

    fn send_message(&self, destination: &str, chat_type: ChatType, content: &str) {
        self.send_message(destination, chat_type, content);
    }

    fn send_action(&self, destination: &str, chat_type: ChatType, action: &str) {
        self.send_action(destination, chat_type, action);
    }

    fn join_channel(&self, channel: &str) {
        self.join_channel(channel);
    }

    fn leave_channel(&self, channel: &str) {
        self.leave_channel(channel);
    }
}

impl HTTPActorHandle {
    pub fn new(app_event_sender: UnboundedSender<AppMessageIn>) -> Self {
        let (http_event_sender, http_event_receiver) = unbounded_channel();
        let mut actor = actor::HTTPActor::new(http_event_receiver, app_event_sender);
        std::thread::spawn(move || {
            actor.run();
        });
        Self {
            actor: http_event_sender,
        }
    }

    fn send_or_log(&self, message: HTTPMessageIn) {
        if let Err(e) = self.actor.send(message) {
            log::error!("Failed to send HTTP message: channel closed ({e})");
        }
    }

    pub fn connect_http(&self, settings: APISettings) {
        self.send_or_log(HTTPMessageIn::Connect(settings));
    }

    pub fn disconnect_http(&self) {
        self.send_or_log(HTTPMessageIn::Disconnect);
    }

    pub fn send_action(&self, destination: &str, chat_type: ChatType, action: &str) {
        self.send_or_log(HTTPMessageIn::SendMessage {
            r#type: MessageType::Action,
            destination: destination.to_owned(),
            chat_type,
            content: action.to_owned(),
        });
    }

    pub fn send_message(&self, destination: &str, chat_type: ChatType, content: &str) {
        self.send_or_log(HTTPMessageIn::SendMessage {
            r#type: MessageType::Text,
            destination: destination.to_owned(),
            chat_type,
            content: content.to_owned(),
        });
    }

    pub fn join_channel(&self, channel: &str) {
        self.send_or_log(HTTPMessageIn::JoinChannel(channel.to_owned()));
    }

    pub fn leave_channel(&self, channel: &str) {
        self.send_or_log(HTTPMessageIn::LeaveChannel(channel.to_owned()));
    }
}
