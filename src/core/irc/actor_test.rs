use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::actor::Actor;
use crate::core::irc::actor::IRCActor;
use crate::core::irc::IRCMessageIn;
use steel_core::chat::ConnectionStatus;
use steel_core::ipc::server::{AppMessageIn, ChatEvent};

#[derive(Debug)]
struct TestState {
    // Test lifetime control.
    running: bool,

    // Operation counters.
    connect_count: usize,
    disconnect_count: usize,
    message_count: usize,

    // Progress tracker.
    last_activity_time: Instant,

    // Expected connection state.
    is_connected: bool,
    is_connecting: bool,
    is_disconnecting: bool,

    // Deadlock detection timeout.
    deadlock_timeout_secs: u64,

    // Iteration counter.
    current_iteration: usize,
    max_iterations: usize,
}

impl TestState {
    fn new(max_iterations: usize) -> Self {
        TestState {
            running: true,
            connect_count: 0,
            disconnect_count: 0,
            message_count: 0,
            last_activity_time: Instant::now(),
            is_connected: false,
            is_connecting: false,
            is_disconnecting: false,
            deadlock_timeout_secs: 10,
            current_iteration: 0,
            max_iterations,
        }
    }

    fn update_activity(&mut self) {
        self.last_activity_time = Instant::now();
    }

    fn check_deadlock(&self) -> bool {
        if !self.is_connecting && !self.is_disconnecting {
            return false;
        }

        // This _could_ be a deadlock.
        self.last_activity_time.elapsed().as_secs() > self.deadlock_timeout_secs
    }

    fn start_iteration(&mut self) -> bool {
        if self.current_iteration >= self.max_iterations {
            self.running = false;
            return false;
        }

        self.current_iteration += 1;
        self.is_connecting = true;
        self.is_disconnecting = false;
        self.update_activity();

        println!(
            "Iteration {}/{}",
            self.current_iteration, self.max_iterations
        );
        true
    }

    fn connection_completed(&mut self) {
        self.connect_count += 1;
        self.is_connecting = false;
        self.is_connected = true;
        self.update_activity();
    }

    fn start_disconnection(&mut self) {
        self.is_disconnecting = true;
        self.is_connected = false;
        self.update_activity();
    }

    fn disconnection_completed(&mut self) {
        self.disconnect_count += 1;
        self.is_disconnecting = false;
        self.is_connected = false;
        self.update_activity();
    }

    fn message_received(&mut self) {
        self.message_count += 1;
        self.update_activity();
    }

    fn print_stats(&self) {
        println!("\n=== Stats ===");
        println!(
            "Iterations: {}/{}",
            self.current_iteration, self.max_iterations
        );
        println!("Successful connects: {}", self.connect_count);
        println!("Successful disconnects: {}", self.disconnect_count);
        println!("Messages received: {}", self.message_count);

        if self.check_deadlock() {
            println!("\nWARNING: DEADLOCK SPOTTED");
            if self.is_connecting {
                println!("Deadlock during connection");
            } else if self.is_disconnecting {
                println!("Deadlock during disconnection");
            }
            println!(
                "Last activity: {}s ago",
                self.last_activity_time.elapsed().as_secs()
            );
        } else {
            println!("\nNo deadlock detected");
        }
    }
}

fn run_improved_deadlock_test() {
    let test_state = Arc::new(Mutex::new(TestState::new(100)));
    let test_state_monitor = Arc::clone(&test_state);
    let test_state_controller = Arc::clone(&test_state);

    let (input_tx, input_rx, output_tx, mut output_rx) = create_message_channels();

    let force_exit = Arc::new(AtomicBool::new(false));
    let force_exit_clone = Arc::clone(&force_exit);

    let mut irc_actor = IRCActor::new(input_rx, output_tx);
    let actor_thread = thread::spawn(move || {
        println!("IRCActor thread started");
        irc_actor.run();
        println!("IRCActor thread exited");
    });

    let monitor_thread = thread::spawn(move || {
        println!("Connection monitoring thread started");

        while let Some(msg) = output_rx.blocking_recv() {
            let mut state = test_state_monitor.lock().unwrap();

            match msg {
                AppMessageIn::Chat(ChatEvent::ConnectionChanged(status)) => match status {
                    ConnectionStatus::Connected => {
                        println!("Connected");
                        state.connection_completed();
                    }
                    ConnectionStatus::Disconnected { by_user } => {
                        println!("Disconnected (requested by user: {by_user})");
                        state.disconnection_completed();
                    }
                    ConnectionStatus::InProgress => {
                        println!("Connecting...");
                    }
                    _ => {}
                },
                _ => {
                    state.message_received();
                }
            }
        }

        println!("Connection monitoring thread exited");
    });

    let controller_thread = thread::spawn(move || {
        println!("Test controller thread started");
        while !force_exit_clone.load(Ordering::SeqCst) {
            let mut state = test_state_controller.lock().unwrap();
            let irc_config = create_local_irc_config(state.current_iteration);

            if !state.is_connecting && !state.is_disconnecting && !state.is_connected {
                if !state.start_iteration() {
                    break;
                }

                let config_clone = irc_config.clone();
                drop(state); // Release the lock before sending the command.
                println!("Sending Connect");
                if let Err(e) = input_tx.send(IRCMessageIn::Connect(Box::new(config_clone))) {
                    println!("Failed to send Connect: {e}");
                    break;
                }
            } else if state.is_connected && state.connect_count == state.current_iteration {
                println!("Connected, sending messages...");

                let iteration = state.current_iteration;
                drop(state);

                if let Err(e) = input_tx.send(IRCMessageIn::JoinChannel("#test".to_string())) {
                    println!("Failed to send JoinChannel: {e}");
                    force_exit_clone.store(true, Ordering::SeqCst);
                    break;
                }

                for j in 0..3 {
                    if let Err(e) = input_tx.send(IRCMessageIn::SendMessage {
                        r#type: crate::core::chat::MessageType::Text,
                        destination: "#test".to_string(),
                        content: format!("Test message {j} in iteration {iteration}"),
                    }) {
                        println!("Failed to send a message: {e}");
                        force_exit_clone.store(true, Ordering::SeqCst);
                        break;
                    }

                    thread::sleep(Duration::from_millis(50));
                }

                thread::sleep(Duration::from_millis(1000));

                println!("Sending Disconnect");

                let mut state = test_state_controller.lock().unwrap();
                state.start_disconnection();
                drop(state);

                if let Err(e) = input_tx.send(IRCMessageIn::Disconnect) {
                    println!("Failed to send Disconnect: {e}");
                    force_exit_clone.store(true, Ordering::SeqCst);
                    break;
                }
                println!("(should be done disconnecting)");
            }

            if let Ok(state) = test_state_controller.try_lock() {
                if state.check_deadlock() {
                    println!("Warning: potential deadlock detected!");
                    println!(
                        "Last activity: {}s ago",
                        state.last_activity_time.elapsed().as_secs()
                    );

                    force_exit_clone.store(true, Ordering::SeqCst);
                    break;
                }
            }

            thread::sleep(Duration::from_millis(100));
        }

        println!("Test completed, closing the channels...");
        drop(input_tx);
        println!("Test controller thread exited");
    });

    controller_thread.join().unwrap();
    monitor_thread.join().unwrap();
    actor_thread.join().unwrap();

    let state = test_state.lock().unwrap();
    state.print_stats();
}

fn create_local_irc_config(i: usize) -> irc::client::data::Config {
    let username = format!("test_user_{i}");
    irc::client::data::Config {
        username: Some(username.clone()),
        nickname: Some(username.clone()),
        realname: Some(username.clone()),
        password: Some("password".to_owned()),

        server: Some("localhost".to_string()),
        use_tls: Some(false),

        channels: vec!["#test".to_string()],

        ..Default::default()
    }
}

fn create_message_channels() -> (
    UnboundedSender<IRCMessageIn>,
    UnboundedReceiver<IRCMessageIn>,
    UnboundedSender<AppMessageIn>,
    UnboundedReceiver<AppMessageIn>,
) {
    let (input_tx, input_rx) = mpsc::unbounded_channel::<IRCMessageIn>();
    let (output_tx, output_rx) = mpsc::unbounded_channel::<AppMessageIn>();
    (input_tx, input_rx, output_tx, output_rx)
}

/*
This must be run against an IRC server. Example configuration for ngIRCd:

[Global]
    Listen=127.0.0.1
    MotdPhrase = "Hello world!"
    Password = password

[Limits]
    MaxConnectionsIP = 0
    MaxJoins = 0
    MaxNickLength = 100
    PongTimeout = 120
    MaxConnections = 0

[Options]
    Ident = no
    Debug = yes
    DNS = no
    StrictUtf8 = no

[Channel]
    Name = #test
    Modes = tn

Run as:

    ngircd --config ngircd.conf --nodaemon
*/
#[test]
#[ignore = "heavyweight; must be run manually"]
fn test_deadlock_detection() {
    run_improved_deadlock_test();
}
