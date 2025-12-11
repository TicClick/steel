use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChatError {
    #[error("fatal chat error: {0}")]
    FatalError(String),
    #[error("chat error: {content}")]
    ServerError {
        chat: Option<String>,
        content: String,
    },
}
