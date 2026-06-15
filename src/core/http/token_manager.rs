use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;

use crate::core::http::jwt::{self, AccessTokenTiming};
use crate::core::http::token_refresh::{self, TokenRefreshConfig, TokenRefreshError};
use crate::core::http::token_storage::{self, PersistedTokenState, API_TOKEN_LIFETIME_SECS};

const ACCESS_TOKEN_REFRESH_MARGIN_SECS: i64 = 30 * 60;

pub struct TokenManager {
    config: TokenRefreshConfig,
    rotation_threshold: Duration,
    state: RwLock<PersistedTokenState>,
}

impl TokenManager {
    pub fn new(
        config: TokenRefreshConfig,
        rotation_threshold: Duration,
        state: PersistedTokenState,
    ) -> Self {
        Self {
            config,
            rotation_threshold,
            state: RwLock::new(state),
        }
    }

    pub fn config(&self) -> &TokenRefreshConfig {
        &self.config
    }

    pub fn snapshot(&self) -> PersistedTokenState {
        self.state.read().clone()
    }

    pub fn access_token(&self) -> String {
        self.state.read().access_token.clone()
    }

    pub fn access_token_timing(&self) -> Option<AccessTokenTiming> {
        jwt::access_token_timing(&self.state.read().access_token)
    }

    pub fn access_token_expires_at(&self) -> DateTime<Utc> {
        self.state.read().access_expires_at
    }

    pub fn refresh_token_expires_at(&self) -> Option<DateTime<Utc>> {
        let state = self.state.read();
        state
            .refresh_token
            .as_ref()
            .map(|_| state.refresh_expires_at)
    }

    pub fn is_access_token_valid(&self) -> bool {
        self.state.read().is_access_token_valid()
    }

    pub fn is_refresh_token_usable(&self) -> bool {
        let state = self.state.read();
        state.refresh_token.is_some() && state.is_refresh_token_valid()
    }

    pub fn refresh_token_needs_rotation(&self) -> bool {
        let state = self.state.read();
        state.refresh_token.is_some()
            && state.is_refresh_token_valid()
            && state.refresh_expires_at - Utc::now() < self.rotation_threshold
    }

    pub fn next_refresh_due(&self) -> DateTime<Utc> {
        let state = self.state.read();
        let access_due =
            state.access_expires_at - Duration::seconds(ACCESS_TOKEN_REFRESH_MARGIN_SECS);
        let rotation_due = state.refresh_expires_at - self.rotation_threshold;
        access_due.min(rotation_due)
    }

    pub async fn refresh(&self) -> Result<PersistedTokenState, TokenRefreshError> {
        let Some(refresh_token) = self.state.read().refresh_token.clone() else {
            return Err(TokenRefreshError::Rejected(
                "no refresh token available".to_owned(),
            ));
        };

        let config = self.config.clone();
        let request_token = refresh_token.clone();
        let refreshed = tokio::task::spawn_blocking(move || {
            token_refresh::refresh_tokens(&config, &request_token)
        })
        .await
        .map_err(|e| TokenRefreshError::RequestFailed(format!("refresh task panicked: {e}")))??;

        let new_state = PersistedTokenState::new(
            refreshed.access_token,
            refreshed.refresh_token.or(Some(refresh_token)),
            refreshed.expires_in.unwrap_or(API_TOKEN_LIFETIME_SECS),
        );

        *self.state.write() = new_state.clone();
        if let Err(e) = token_storage::save_token_state(&new_state) {
            log::error!("Failed to save refreshed token state: {e}");
        }

        Ok(new_state)
    }
}
