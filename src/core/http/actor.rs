use steel_core::{chat::ConnectionStatus, ipc::server::AppMessageIn};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::{
    actor::Actor,
    core::http::{websocket::client::websocket_thread_main, APISettings, HTTPMessageIn},
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
            HTTPMessageIn::JoinChannel(channel) => {
                log::debug!(
                    "HTTP backend: join channel request for {} (not yet implemented)",
                    channel
                );
                // TODO: Implement via API
            }
            HTTPMessageIn::LeaveChannel(channel) => {
                log::debug!(
                    "HTTP backend: leave channel request for {} (not yet implemented)",
                    channel
                );
                // TODO: Implement via API
            }
            HTTPMessageIn::SendMessage {
                r#type,
                destination,
                content,
            } => {
                log::debug!(
                    "HTTP backend: send message to {} (not yet implemented): {:?} - {}",
                    destination,
                    r#type,
                    content
                );
                // TODO: Implement via API
            }
        }
    }
}

impl HTTPActor {
    fn connect(&mut self, settings: APISettings) {
        self.output
            .send(AppMessageIn::ConnectionChanged(
                ConnectionStatus::InProgress,
            ))
            .unwrap();

        let tx = self.output.clone();

        self.ws_thread = Some(std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(websocket_thread_main(tx, settings));
        }));

        self.ws_thread.take().map(|t| {
            let res = t.join();
            println!("Thread result: {res:?}");
        });

        self.output
            .send(AppMessageIn::ConnectionChanged(
                ConnectionStatus::Disconnected { by_user: false },
            ))
            .unwrap();
    }

    fn disconnect(&mut self) {}
}
