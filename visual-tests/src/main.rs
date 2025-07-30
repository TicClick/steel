use rand::{rng, Rng};
use tokio::sync::mpsc::unbounded_channel;

use steel::core::chat::Message;
use steel::core::ipc::ui::UIMessageIn;
use steel::run_app;

pub fn main() {
    let (ui_queue_in, ui_queue_out) = unbounded_channel();

    ui_queue_in
        .send(UIMessageIn::ConnectionStatusChanged(
            steel::core::chat::ConnectionStatus::Connected,
        ))
        .unwrap();

    for i in 0..10 {
        ui_queue_in
            .send(UIMessageIn::NewChatRequested {
                target: format!("#test-{}", i),
                switch: true,
            })
            .unwrap();
    }

    for name in [
        "African clawed frog",
        "Bass",
        "Crane",
        "Dragonfish",
        "Eel",
        "Feral hog",
    ] {
        ui_queue_in
            .send(UIMessageIn::NewChatRequested {
                target: name.to_owned(),
                switch: true,
            })
            .unwrap();
    }

    for i in 0..25 {
        let len = rng().random_range(1..40);
        let msg: String = rng()
            .sample_iter(&rand::distr::Alphanumeric)
            .take(len)
            .map(char::from)
            .collect();

        ui_queue_in
            .send(UIMessageIn::NewMessageReceived {
                target: "#test-0".to_owned(),
                message: Message::new_text(format!("{}", i).as_str(), msg.as_str()),
            })
            .unwrap();
    }

    let mq = ui_queue_in.clone();
    std::thread::spawn(move || loop {
        mq.send(UIMessageIn::NewMessageReceived {
            target: "#test-0".to_owned(),
            message: Message::new_text("test", "testy"),
        })
        .unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
    });

    let app_thread = run_app(
        ui_queue_in,
        ui_queue_out,
        std::env::current_exe().ok()
    );
    app_thread.join().unwrap();
}
