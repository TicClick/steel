use rand::{thread_rng, Rng};
use tokio::sync::mpsc::unbounded_channel;

use steel::core::chat::{ChatState, Message};
use steel::core::ipc::ui::UIMessageIn;
use steel::run_app;

pub fn main() {
    let (ui_queue_in, ui_queue_out) = unbounded_channel();

    ui_queue_in
        .send(UIMessageIn::ConnectionStatusChanged(
            steel::core::chat::ConnectionStatus::Connected,
        ))
        .unwrap();

    ui_queue_in
        .send(UIMessageIn::NewChatRequested(
            "#test".to_owned(),
            ChatState::Joined,
            true,
        ))
        .unwrap();

    for i in 0..25 {
        let len = thread_rng().gen_range(1..40);
        let msg: String = thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(len)
            .map(char::from)
            .collect();

        ui_queue_in
            .send(UIMessageIn::NewMessageReceived {
                target: "#test".to_owned(),
                message: Message::new_text(format!("{}", i).as_str(), msg.as_str()),
            })
            .unwrap();
    }

    let app_thread = run_app(ui_queue_in, ui_queue_out);
    app_thread.join().unwrap();
}
