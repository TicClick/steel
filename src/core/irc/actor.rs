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

static IRC_EVENT_WAIT_TIMEOUT: Duration = Duration::from_millis(5);

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
            .send(AppMessageIn::ConnectionChanged(
                ConnectionStatus::InProgress,
            ))
            .unwrap();
        self.start_irc_watcher(config, self.output.clone());
    }

    fn start_irc_watcher(
        &mut self,
        config: irc::client::data::Config,
        tx: UnboundedSender<AppMessageIn>,
    ) {
        let own_username = config.username.clone().unwrap();
        let arc = Arc::clone(&self.irc_sync);
        let irc_stream_sender = Arc::clone(&self.irc_stream_sender);

        self.irc_thread = Some(std::thread::spawn(move || {
            let rt = runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            match rt.block_on(irc::client::Client::from_config(config)) {
                Err(e) => {
                    tx.send(AppMessageIn::ChatError(IRCError::FatalError(format!(
                        "failed to start the IRC client: {}",
                        e
                    ))))
                    .unwrap();
                    tx.send(AppMessageIn::ConnectionChanged(
                        ConnectionStatus::Disconnected { by_user: false },
                    ))
                    .unwrap();
                }
                Ok(mut clt) => {
                    clt.identify().unwrap();
                    tx.send(AppMessageIn::ConnectionChanged(ConnectionStatus::Connected))
                        .unwrap();
                    *irc_stream_sender.lock().unwrap() = Some(clt.sender());

                    let mut disconnected_by_user = false;
                    let mut stream = clt.stream().unwrap();
                    loop {
                        let (lock, cv) = &*arc;
                        {
                            let guard = lock.lock().unwrap();
                            let (mut g, _) =
                                cv.wait_timeout(guard, IRC_EVENT_WAIT_TIMEOUT).unwrap();
                            if *g {
                                *g = false;
                                clt.send_quit("").unwrap();
                                disconnected_by_user = true;
                                break;
                            }
                        }

                        match rt.block_on(stream.next()) {
                            Some(result) => match result {
                                Err(reason) => {
                                    tx.send(AppMessageIn::ChatError(IRCError::FatalError(
                                        format!("connection broken: {}", reason),
                                    )))
                                    .unwrap();
                                    break;
                                }
                                Ok(msg) => {
                                    event_handler::dispatch_message(&tx, msg, &own_username);
                                }
                            },
                            None => {
                                tx.send(AppMessageIn::ChatError(IRCError::FatalError(
                                    "remote server has closed the connection, probably because it went offline".to_owned(),
                                )))
                                .unwrap();
                                break;
                            }
                        }
                    }

                    tx.send(AppMessageIn::ConnectionChanged(
                        ConnectionStatus::Disconnected {
                            by_user: disconnected_by_user,
                        },
                    ))
                    .unwrap();
                }
            }
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
