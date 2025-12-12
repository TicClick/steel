use thiserror::Error;

#[derive(Debug, Error)]
pub enum SteelApplicationError {
    #[error("Invalid OAuth token")]
    InvalidOAuth,
}

pub type SteelApplicationResult<T> = Result<T, SteelApplicationError>;
