#[derive(thiserror::Error, Debug)]
pub enum IRCError {
    #[error("fatal IRC error: {0}")]
    FatalError(String),
    #[error("IRC error {code:?}: {content}")]
    ServerError {
        code: irc_proto::Response,
        chat: Option<String>,
        content: String,
    },
}
