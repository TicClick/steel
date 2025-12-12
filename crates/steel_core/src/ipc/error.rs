use thiserror::Error;

#[derive(Debug, Error)]
pub enum IpcError {
    #[error("Channel closed: {context}")]
    ChannelClosed { context: String },
}

pub type IpcResult<T> = Result<T, IpcError>;
