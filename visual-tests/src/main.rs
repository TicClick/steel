use clap::Parser;
use rand::{rng, Rng};
use tokio::sync::mpsc::unbounded_channel;

use steel::core::chat::Message;
use steel::core::ipc::ui::UIMessageIn;
use steel::run_app;

const PRIVATE_CHAT_NAMES: &[&str] = &[
    "African clawed frog",
    "Bass",
    "Crane",
    "Dragonfish",
    "Eel",
    "Feral hog",
    "Goldfish",
    "Hedge hawk",
    "Iguana",
    "Jellyfish",
];

fn generate_random_message(max_len: usize) -> String {
    let len = rng().random_range(1..=max_len);
    rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

fn build_all_targets(channels: usize, private_chats: usize) -> Vec<String> {
    let mut all_targets = Vec::new();
    for i in 0..channels {
        all_targets.push(format!("#test-{i}"));
    }
    for name in PRIVATE_CHAT_NAMES.iter().take(private_chats) {
        all_targets.push(name.to_string());
    }
    all_targets
}

fn get_target_for_mode(mode: &Option<MessageMode>, target: &Option<String>, channels: usize, private_chats: usize) -> String {
    match mode {
        Some(MessageMode::Random) => {
            let all_targets = build_all_targets(channels, private_chats);
            let target_idx = rng().random_range(0..all_targets.len());
            all_targets[target_idx].clone()
        },
        _ => target.as_ref().map(|s| s.clone()).unwrap_or_else(|| "#test-0".to_string()),
    }
}

fn send_messages(ui_queue: &tokio::sync::mpsc::UnboundedSender<UIMessageIn>, target: &str, count: usize, max_len: usize, _sender_name: &str) {
    for i in 0..count {
        let msg = generate_random_message(max_len);
        ui_queue
            .send(UIMessageIn::NewMessageReceived {
                target: target.to_string(),
                message: Message::new_text(format!("{i}").as_str(), msg.as_str()),
            })
            .unwrap();
    }
}

#[derive(Parser, Debug)]
#[command(name = "visual-tests")]
struct Args {
    /// Number of test channels to create
    #[arg(long, default_value_t = 10)]
    ch: usize,

    /// Number of messages to send per channel
    #[arg(long, default_value_t = 25)]
    count: usize,

    /// Number of private chats to create
    #[arg(long, default_value_t = 3)]
    dm: usize,

    /// Maximum length of random messages
    #[arg(long, default_value_t = 40)]
    len: usize,

    /// Message sending mode: none, random, or target specific chat
    #[arg(long)]
    mode: Option<MessageMode>,

    /// Target chat for messages (when using target mode)
    #[arg(long)]
    target: Option<String>,

    /// Enable continuous message sending
    #[arg(long)]
    go: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum MessageMode {
    None,
    Random,
    Target,
}

pub fn main() {
    let args = Args::parse();
    let (ui_queue_in, ui_queue_out) = unbounded_channel();

    ui_queue_in
        .send(UIMessageIn::ConnectionStatusChanged(
            steel::core::chat::ConnectionStatus::Connected,
        ))
        .unwrap();

    for i in 0..args.ch {
        ui_queue_in
            .send(UIMessageIn::NewChatRequested {
                target: format!("#test-{i}"),
                switch: true,
            })
            .unwrap();
    }

    for name in PRIVATE_CHAT_NAMES.iter().take(args.dm) {
        ui_queue_in
            .send(UIMessageIn::NewChatRequested {
                target: name.to_string(),
                switch: true,
            })
            .unwrap();
    }

    if let Some(ref mode) = args.mode {
        match mode {
            MessageMode::None => {},
            MessageMode::Random => {
                let all_targets = build_all_targets(args.ch, args.dm);
                for i in 0..args.count {
                    let msg = generate_random_message(args.len);
                    let target_idx = rng().random_range(0..all_targets.len());
                    ui_queue_in
                        .send(UIMessageIn::NewMessageReceived {
                            target: all_targets[target_idx].clone(),
                            message: Message::new_text(format!("{i}").as_str(), msg.as_str()),
                        })
                        .unwrap();
                }
            },
            MessageMode::Target => {
                let target = args.target.as_ref().map(|s| s.clone()).unwrap_or_else(|| "#test-0".to_string());
                send_messages(&ui_queue_in, &target, args.count, args.len, "batch");
            },
        }
    }

    if args.go {
        let mq = ui_queue_in.clone();
        let max_len = args.len;
        let message_mode = args.mode.clone();
        let channels = args.ch;
        let private_chats = args.dm;
        let target = args.target.clone();
        
        std::thread::spawn(move || {
            loop {
                let msg = generate_random_message(max_len);
                let send_target = get_target_for_mode(&message_mode, &target, channels, private_chats);

                mq.send(UIMessageIn::NewMessageReceived {
                    target: send_target,
                    message: Message::new_text("continuous", msg.as_str()),
                })
                .unwrap();
                
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });
    }

    let app_thread = run_app(ui_queue_in, ui_queue_out, std::env::current_exe().ok());
    app_thread.join().unwrap();
}
