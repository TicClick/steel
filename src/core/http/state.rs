use crate::core::chat_cache::ChatCache;
use crate::core::http::token_storage::PersistedTokenState;
use chrono::{DateTime, Utc};
use std::sync::Arc;

pub struct HTTPState {
    pub cache: ChatCache,
    pub own_username: Option<String>,
    pub own_user_id: Option<u32>,
    pub api: Option<Arc<rosu_v2::Osu>>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub refresh_token_expires_at: Option<DateTime<Utc>>,
    pub shutdown_requested: bool,
}

impl HTTPState {
    pub fn new() -> Self {
        Self {
            cache: ChatCache::new(),
            own_username: None,
            own_user_id: None,
            api: None,
            token_expires_at: None,
            refresh_token_expires_at: None,
            shutdown_requested: false,
        }
    }

    pub fn request_shutdown(&mut self) {
        self.shutdown_requested = true;
    }

    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested
    }

    pub fn set_own_user(&mut self, username: String, user_id: u32) {
        self.own_username = Some(username);
        self.own_user_id = Some(user_id);
    }

    pub fn set_api_client(&mut self, api: Arc<rosu_v2::Osu>) {
        self.api = Some(api);
    }

    pub fn set_token_expiry(&mut self, token_state: &PersistedTokenState) {
        self.token_expires_at = Some(token_state.access_expires_at);
        self.refresh_token_expires_at = Some(token_state.refresh_expires_at);
    }

    pub fn is_token_valid(&self) -> bool {
        if let Some(expires_at) = self.token_expires_at {
            let now = Utc::now();
            // Add 5% buffer for safety
            let buffer = chrono::Duration::seconds(
                (expires_at.signed_duration_since(now).num_seconds() as f64 * 0.05) as i64,
            );
            now < (expires_at - buffer)
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.own_username = None;
        self.own_user_id = None;
        self.api = None;
        self.token_expires_at = None;
        self.refresh_token_expires_at = None;
        self.shutdown_requested = false;
    }
}

impl Default for HTTPState {
    fn default() -> Self {
        Self::new()
    }
}
