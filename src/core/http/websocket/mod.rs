use rosu_v2::{model::chat::ChatChannelMessage, prelude::User};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod client;

#[derive(Debug, Serialize, Deserialize)]
pub enum EventType {
    // Server messages.
    #[serde(rename = "connection.ready")]
    ConnectionReady,
    #[serde(rename = "logout")]
    Logout,
    #[serde(rename = "new")]
    New,
    #[serde(rename = "read")]
    Read,
    #[serde(rename = "chat.channel.join")]
    ChatChannelJoin,
    #[serde(rename = "chat.channel.part")]
    ChatChannelPart,
    #[serde(rename = "chat.message.new")]
    ChatMessageNew,

    // Client messages.
    #[serde(rename = "chat.start")]
    ChatStart,
    #[serde(rename = "chat.stop")]
    ChatStop,
}

#[derive(Serialize, Deserialize)]
pub struct GeneralWebsocketEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<EventType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl GeneralWebsocketEvent {
    pub fn new(event: EventType) -> Self {
        Self {
            event: Some(event),
            data: None,
            error: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ChatMessageNewData {
    pub messages: Vec<ChatChannelMessage>,
    pub users: Vec<User>,
}
