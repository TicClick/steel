use std::time::Duration;

use chrono::DurationRound;
use steel_core::{ipc::server::AppMessageIn, DEFAULT_DATE_FORMAT};
use tokio::sync::mpsc::UnboundedSender;

use crate::actor::ActorHandle;

pub(super) struct DateAnnouncer {
    _thread: std::thread::JoinHandle<()>,
}

impl ActorHandle for DateAnnouncer {}

impl DateAnnouncer {
    pub(super) fn new(app_queue: UnboundedSender<AppMessageIn>) -> Self {
        Self {
            _thread: std::thread::spawn(|| announcer(app_queue)),
        }
    }
}

fn announcer(app_queue: UnboundedSender<AppMessageIn>) {
    let mut then = chrono::Local::now();
    loop {
        let now = chrono::Local::now();
        if then.date_naive() < now.date_naive() {
            then = now;
            let round_date = now.duration_trunc(chrono::Duration::days(1)).unwrap();
            let message = format!(
                "A new day is born ({})",
                round_date.format(DEFAULT_DATE_FORMAT),
            );
            app_queue
                .send(AppMessageIn::DateChanged(round_date, message))
                .unwrap();
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
