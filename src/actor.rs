use async_trait::async_trait;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[async_trait]
pub trait Actor<T, U> {
    fn new(input: UnboundedReceiver<T>, output: UnboundedSender<U>) -> Self;
    async fn handle_message(&mut self, message: T);
    async fn run(&mut self);
}

pub trait ActorHandle {}
