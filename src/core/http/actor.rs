use futures::{SinkExt, StreamExt};
use rosu_v2::prelude::Scopes;
use steel_core::{chat::ConnectionStatus, ipc::server::AppMessageIn};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{self, Message, client::IntoClientRequest, protocol::WebSocketConfig},
};
use ureq::http::{self, HeaderValue};

use crate::{
    actor::Actor,
    core::http::{APISettings, HTTPMessageIn},
};

pub struct HTTPActor {
    input: UnboundedReceiver<HTTPMessageIn>,
    output: UnboundedSender<AppMessageIn>,

    ws_thread: Option<std::thread::JoinHandle<()>>,
}

impl Actor<HTTPMessageIn, AppMessageIn> for HTTPActor {
    fn new(input: UnboundedReceiver<HTTPMessageIn>, output: UnboundedSender<AppMessageIn>) -> Self {
        Self {
            input,
            output,
            ws_thread: None,
        }
    }

    fn run(&mut self) {
        while let Some(msg) = self.input.blocking_recv() {
            self.handle_message(msg);
        }
    }

    fn handle_message(&mut self, message: HTTPMessageIn) {
        match message {
            HTTPMessageIn::Connect(settings) => self.connect(settings),
            HTTPMessageIn::Disconnect => self.disconnect(),
        }
    }
}

impl HTTPActor {
    fn connect(&mut self, settings: APISettings) {
        // self.output
        //     .send(AppMessageIn::ConnectionChanged(
        //         ConnectionStatus::InProgress,
        //     ))
        //     .unwrap();

        let tx = self.output.clone();

        self.ws_thread = Some(std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(http_thread_main(tx, settings));
        }));

        self.ws_thread.take().map(|t| {
            let res = t.join();
            println!("Thread result: {res:?}");
        });
    }

    fn disconnect(&mut self) {}
}

async fn http_thread_main(tx: UnboundedSender<AppMessageIn>, settings: APISettings) {
    println!("Authenticating...");
    let api = rosu_v2::OsuBuilder::new()
        .with_local_authorization(
            settings.redirect_uri,
            Scopes::Public
                | Scopes::Identify
                | Scopes::ChatRead
                | Scopes::ChatWrite
                | Scopes::ChatWriteManage,
        )
        .client_id(settings.client_id)
        .client_secret(settings.client_secret)
        .build()
        .await
        .unwrap();

    println!("Got token");
    if let Some(token) = api.token().access() {
        println!("Access token: {token}");
        println!("Refresh token: {}", api.token().refresh().unwrap());
        let mut request = settings.ws_base_uri.into_client_request().unwrap();

        let token_header = HeaderValue::from_str(token).unwrap();
        request.headers_mut().insert("Authorization", token_header);

        println!("WS request: {request:?}");
        println!("Authorization header: {:?}", request.headers().get("Authorization"));

        let ws_config = WebSocketConfig::default();
        let (mut ws, resp) = connect_async_with_config(request, Some(ws_config), true)
            .await
            .unwrap();

        println!("Connected, response: {resp:?}");
        let now = std::time::Instant::now();

        while let Some(msg) = ws.next().await {
            let utcnow = chrono::Utc::now();
            match msg {
                Err(e) => println!("[{utcnow}] Connection broke: {e}"),
                Ok(msg) => match msg {
                    Message::Text(t) => {
                        println!("[{utcnow}] {t}");
                        if t.contains("connection.ready") {

                            let ka = api.chat_keepalive().await.unwrap();
                            println!("keepalive sent and received: {ka:?}");

                            // ws.send(Message::Text(
                            //     "{\"event\": \"chat.end\"}".into()
                            // )).await.unwrap();
                            ws.send(Message::Text(
                                "{\"event\": \"chat.start\"}".into()
                            )).await.unwrap();
                            println!("sent chat start message");
                        }
                    }
                    _ => println!("[{utcnow}] {msg:?}"),
                }
            }

            if now.elapsed().as_secs() >= 120 {
                break;
            }
        }
    }
}
