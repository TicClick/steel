use tokio::sync::mpsc::{Receiver, Sender};
pub trait Actor<T, U> {
    fn new(input: Receiver<T>, output: Sender<U>) -> Self;
    fn handle_message(&mut self, message: T);
    fn run(&mut self);
}

pub trait ActorHandle {}
