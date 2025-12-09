use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedTokenState {
    // OAuth access token (including "Bearer " prefix)
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub access_expires_at: DateTime<Utc>,
    // Note: For osu! API, refresh tokens have the same lifetime as access tokens
    pub refresh_expires_at: DateTime<Utc>,
}

impl PersistedTokenState {
    pub fn new(
        access_token: String,
        refresh_token: Option<String>,
        expires_in_seconds: i64,
    ) -> Self {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(expires_in_seconds);

        Self {
            access_token,
            refresh_token,
            access_expires_at: expires_at,
            refresh_expires_at: expires_at,
        }
    }

    pub fn is_access_token_valid(&self) -> bool {
        let now = Utc::now();
        // Add a 5% safety margin - same as rosu-v2's internal logic
        let buffer = chrono::Duration::seconds(
            (self
                .access_expires_at
                .signed_duration_since(now)
                .num_seconds() as f64
                * 0.05) as i64,
        );
        now < (self.access_expires_at - buffer)
    }

    pub fn is_refresh_token_valid(&self) -> bool {
        Utc::now() < self.refresh_expires_at
    }

    pub fn has_valid_token(&self) -> bool {
        self.is_access_token_valid()
            || (self.refresh_token.is_some() && self.is_refresh_token_valid())
    }
}

pub fn get_token_storage_path() -> PathBuf {
    PathBuf::from("temporary-state.yaml")
}

pub fn save_token_state(state: &PersistedTokenState) -> Result<(), Box<dyn std::error::Error>> {
    let path = get_token_storage_path();
    let yaml = serde_yaml::to_string(state)?;

    log::info!("Saving token state to {path:?}");
    std::fs::write(&path, yaml)?;
    log::warn!("Token state saved to {path:?} - this file contains sensitive authentication data");
    Ok(())
}

pub fn load_token_state() -> Result<PersistedTokenState, Box<dyn std::error::Error>> {
    let path = get_token_storage_path();

    if !path.exists() {
        return Err("Token state file does not exist".into());
    }

    log::info!("Loading token state from {path:?}");
    let yaml = std::fs::read_to_string(&path)?;
    let state: PersistedTokenState = serde_yaml::from_str(&yaml)?;

    Ok(state)
}

pub fn clear_token_state() -> Result<(), Box<dyn std::error::Error>> {
    let path = get_token_storage_path();

    if path.exists() {
        log::info!("Deleting token state file: {path:?}");
        std::fs::remove_file(&path)?;
    }

    Ok(())
}
