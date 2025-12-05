use steel_core::chat::MessageType;
use steel_core::ipc::server::AppMessageIn;
use steel_core::settings::chat::default_api_client_id;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use crate::actor::{Actor, ActorHandle};
use crate::core::chat_backend::ChatBackend;

pub mod actor;
pub mod api;
pub mod websocket;

#[derive(Debug)]
pub struct APISettings {
    pub client_id: u64,
    pub client_secret: String,
    pub redirect_uri: String,
    pub ws_base_uri: String,
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
            client_id,
            client_secret: api_config.client_secret.clone(),
            redirect_uri: api_config.redirect_uri.clone(),
            ws_base_uri: api_config.ws_base_uri.clone(),
        };
        self.connect_http(api_settings);
    }

    fn disconnect(&self) {
        self.disconnect_http();
    }

    fn send_message(&self, destination: &str, content: &str) {
        self.send_message(destination, content);
    }

    fn send_action(&self, destination: &str, action: &str) {
        self.send_action(destination, action);
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

    pub fn connect_http(&self, settings: APISettings) {
        self.actor
            .send(HTTPMessageIn::Connect(settings))
            .expect("failed to queue chat connection");
    }

    pub fn disconnect_http(&self) {
        self.actor
            .send(HTTPMessageIn::Disconnect)
            .expect("failed to queue disconnecting from chat");
    }

    pub fn send_action(&self, destination: &str, action: &str) {
        self.actor
            .send(HTTPMessageIn::SendMessage {
                r#type: MessageType::Action,
                destination: destination.to_owned(),
                content: action.to_owned(),
            })
            .expect("failed to queue a chat action");
    }

    pub fn send_message(&self, destination: &str, content: &str) {
        self.actor
            .send(HTTPMessageIn::SendMessage {
                r#type: MessageType::Text,
                destination: destination.to_owned(),
                content: content.to_owned(),
            })
            .expect("failed to queue a chat message");
    }

    pub fn join_channel(&self, channel: &str) {
        self.actor
            .send(HTTPMessageIn::JoinChannel(channel.to_owned()))
            .expect("failed to queue joining a channel");
    }

    pub fn leave_channel(&self, channel: &str) {
        self.actor
            .send(HTTPMessageIn::LeaveChannel(channel.to_owned()))
            .expect("failed to queue leaving a channel");
    }
}
