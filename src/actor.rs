use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
pub trait Actor<T, U> {
    fn new(input: UnboundedReceiver<T>, output: UnboundedSender<U>) -> Self;
    fn handle_message(&mut self, message: T);
    fn run(&mut self);
}

pub trait ActorHandle {}
