use steel_core::ipc::server::AppMessageIn;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use crate::actor::{Actor, ActorHandle};

pub mod actor;

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

    pub fn connect(&self, settings: APISettings) {
        self.actor.send(HTTPMessageIn::Connect(settings)).unwrap();
    }
}
