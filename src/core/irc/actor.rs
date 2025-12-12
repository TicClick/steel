#![allow(clippy::mutex_atomic)]

use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use futures::prelude::*;
use tokio::runtime;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::actor::Actor;
use crate::core::chat;
use crate::core::irc::event_handler;

use steel_core::chat::{irc::IRCError, ConnectionStatus};
use steel_core::ipc::server::AppMessageIn;

use super::IRCMessageIn;

static IRC_MESSAGE_POLLING_TIMEOUT: Duration = Duration::from_millis(10);

pub struct IRCActor {
    input: UnboundedReceiver<IRCMessageIn>,
    output: UnboundedSender<AppMessageIn>,
    irc_stream_sender: Arc<Mutex<Option<irc::client::Sender>>>,

    irc_sync: Arc<(Mutex<bool>, Condvar)>,
    irc_thread: Option<std::thread::JoinHandle<()>>,
}

impl Actor<IRCMessageIn, AppMessageIn> for IRCActor {
    fn new(input: UnboundedReceiver<IRCMessageIn>, output: UnboundedSender<AppMessageIn>) -> Self {
        IRCActor {
            input,
            output,
            irc_stream_sender: Arc::new(Mutex::new(None)),
            irc_sync: Arc::new((Mutex::new(false), Condvar::new())),
            irc_thread: None,
        }
    }

    fn handle_message(&mut self, message: IRCMessageIn) {
        match message {
            IRCMessageIn::Connect(config) => self.connect(*config),
            IRCMessageIn::Disconnect => self.disconnect(),
            IRCMessageIn::SendMessage {
                r#type,
                destination,
                content,
            } => self.send_message(r#type, destination, content),
            IRCMessageIn::JoinChannel(channel) => self.join_channel(channel),
            IRCMessageIn::LeaveChannel(channel) => self.leave_channel(channel),
        }
    }

    fn run(&mut self) {
        while let Some(msg) = self.input.blocking_recv() {
            self.handle_message(msg);
        }
    }
}

impl IRCActor {
    fn connected(&self) -> bool {
        self.irc_stream_sender.lock().unwrap().is_some()
    }

    pub fn disconnect(&mut self) {
        if !self.connected() {
            return;
        }

        // Signal and release the lock, and only then join the thread to avoid deadlocking at the beginning of the loop
        //  in start_irc_watcher().
        {
            let (mutex, cv) = &*self.irc_sync;
            *mutex.lock().unwrap() = true;
            cv.notify_one();
        }

        if let Some(th) = self.irc_thread.take() {
            th.join().unwrap();
        }

        {
            let (mutex, _) = &*self.irc_sync;
            *mutex.lock().unwrap() = false;
        }
        *self.irc_stream_sender.lock().unwrap() = None;
    }

    fn connect(&mut self, config: irc::client::data::Config) {
        if self.connected() {
            return;
        }

        self.output
            .send(AppMessageIn::connection_changed(
                ConnectionStatus::InProgress,
            ))
            .unwrap_or_else(|e| log::error!("Failed to send connection status: {e}"));

        let tx = self.output.clone();
        let arc = Arc::clone(&self.irc_sync);
        let irc_stream_sender = Arc::clone(&self.irc_stream_sender);

        self.irc_thread = Some(std::thread::spawn(move || {
            irc_thread_main(irc_stream_sender, tx, arc, config)
        }));
    }

    fn send_message(&self, r#type: chat::MessageType, destination: String, mut content: String) {
        let guard = self.irc_stream_sender.lock().unwrap();
        if let Some(sender) = &*guard {
            match r#type {
                chat::MessageType::Action => {
                    sender.send_action(destination, content).unwrap();
                }
                chat::MessageType::Text => {
                    // This fixes #49 -- either Bancho loses an extra prefix colon due to its similarity with the IRC
                    // command separator, or https://github.com/aatxe/irc around the transport level, and I can't be
                    // bothered to figure out who is at fault.
                    if content.starts_with(':') {
                        content.insert(0, ' ');
                    }
                    sender.send_privmsg(destination, content).unwrap();
                }

                // Won't happen -- system messages are reserved for the UI display and come from the core system.
                chat::MessageType::System => (),
            }
        }
    }

    fn join_channel(&self, channel: String) {
        let guard = self.irc_stream_sender.lock().unwrap();
        if let Some(sender) = &*guard {
            sender.send_join(channel).unwrap();
        }
    }

    fn leave_channel(&self, channel: String) {
        let guard = self.irc_stream_sender.lock().unwrap();
        if let Some(sender) = &*guard {
            sender.send_part(channel).unwrap();
        }
    }
}

fn irc_thread_main(
    irc_stream_sender: Arc<Mutex<Option<irc::client::Sender>>>,
    tx: UnboundedSender<AppMessageIn>,
    arc: Arc<(Mutex<bool>, Condvar)>,
    config: irc::client::data::Config,
) {
    let own_username = config.username.clone().unwrap();
    let rt = runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    match rt.block_on(irc::client::Client::from_config(config)) {
        Err(e) => {
            tx.send(AppMessageIn::chat_error(IRCError::FatalError(format!(
                "failed to start the IRC client: {e}"
            ))))
            .unwrap_or_else(|e| log::error!("Failed to send chat error: {e}"));
            tx.send(AppMessageIn::connection_changed(
                ConnectionStatus::Disconnected { by_user: false },
            ))
            .unwrap_or_else(|e| log::error!("Failed to send disconnect: {e}"));
        }
        Ok(mut clt) => {
            clt.identify().unwrap();
            tx.send(AppMessageIn::connection_changed(
                ConnectionStatus::Connected,
            ))
            .unwrap_or_else(|e| log::error!("Failed to send connected: {e}"));
            *irc_stream_sender.lock().unwrap() = Some(clt.sender());

            let mut disconnected_by_user = false;
            let mut stream = clt.stream().unwrap();
            loop {
                let must_exit = check_irc_exit_requested(&arc);
                if must_exit {
                    if let Err(e) = clt.send_quit("") {
                        log::error!("Error sending quit command: {e:?}");
                    }
                    disconnected_by_user = true;
                    break;
                }

                if let Ok(result) = rt.block_on(async {
                    tokio::time::timeout(IRC_MESSAGE_POLLING_TIMEOUT, stream.next()).await
                }) {
                    match result {
                        Some(Ok(msg)) => {
                            event_handler::dispatch_message(&tx, msg, &own_username);
                        }
                        Some(Err(reason)) => {
                            tx.send(AppMessageIn::chat_error(IRCError::FatalError(format!(
                                "connection broken: {reason}"
                            ))))
                            .unwrap_or_else(|e| log::error!("Failed to send chat error: {e}"));
                            break;
                        }
                        None => {
                            tx.send(AppMessageIn::chat_error(IRCError::FatalError(
                                "remote server has closed the connection, probably because it went offline".to_owned(),
                            )))
                            .unwrap_or_else(|e| log::error!("Failed to send chat error: {e}"));
                            break;
                        }
                    }
                }
            }

            tx.send(AppMessageIn::connection_changed(
                ConnectionStatus::Disconnected {
                    by_user: disconnected_by_user,
                },
            ))
            .unwrap_or_else(|e| log::error!("Failed to send disconnect: {e}"));
        }
    }
}

fn check_irc_exit_requested(arc: &Arc<(Mutex<bool>, Condvar)>) -> bool {
    let (lock, _) = &**arc;

    let guard = match lock.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            log::error!("IRC sync mutex was poisoned");
            poisoned.into_inner()
        }
    };

    *guard
}
