use rosu_v2::error::OsuError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SteelApplicationError {
    #[error("Invalid OAuth token")]
    InvalidOAuth,

    #[error("Failed to initialize osu! API client")]
    APIInitializationError {
        #[from]
        source: OsuError,
    },
}

pub type SteelApplicationResult<T> = Result<T, SteelApplicationError>;
