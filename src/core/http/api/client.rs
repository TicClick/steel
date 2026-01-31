use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use rosu_v2::{
    model::chat::{ChatChannel, ChatChannelId},
    prelude::{Token, User},
    request::UserId,
    Osu,
};
use tokio::sync::broadcast;
use tokio::sync::RwLock as AsyncRwLock;

use crate::core::http::token_storage;
use crate::core::{error::SteelApplicationError, http::token_storage::API_TOKEN_LIFETIME_SECS};
use steel_core::string_utils::UsernameString;

#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

struct ClientInner {
    api: AsyncRwLock<Osu>,

    user_cache: RwLock<HashMap<u32, User>>,
    username_cache: RwLock<HashMap<String, u32>>,
    channel_cache: RwLock<HashMap<ChatChannelId, ChatChannel>>,
    channel_name_cache: RwLock<HashMap<String, ChatChannelId>>,

    current_token: RwLock<String>,
    token_expires_at: RwLock<DateTime<Utc>>,

    shutdown_tx: broadcast::Sender<()>,

    own_user_id: RwLock<Option<u32>>,
    own_username: RwLock<Option<String>>,
}

impl Client {
    pub async fn new(
        client_id: u64,
        client_secret: String,
        token: Token,
        expires_in: Option<i64>,
    ) -> Result<Self, SteelApplicationError> {
        let api = rosu_v2::OsuBuilder::new()
            .client_id(client_id)
            .client_secret(client_secret.clone())
            .with_token(token, expires_in)
            .build()
            .await?;

        let access_token = api
            .token()
            .access()
            .ok_or(SteelApplicationError::InvalidOAuth)?
            .to_string();

        let exp_secs = expires_in.unwrap_or(API_TOKEN_LIFETIME_SECS);
        let token_expires_at = Utc::now() + chrono::Duration::seconds(exp_secs);

        let (shutdown_tx, _) = broadcast::channel(1);

        let client = Self {
            inner: Arc::new(ClientInner {
                api: AsyncRwLock::new(api),
                user_cache: RwLock::new(HashMap::new()),
                username_cache: RwLock::new(HashMap::new()),
                channel_cache: RwLock::new(HashMap::new()),
                channel_name_cache: RwLock::new(HashMap::new()),
                current_token: RwLock::new(access_token),
                token_expires_at: RwLock::new(token_expires_at),
                shutdown_tx,
                own_user_id: RwLock::new(None),
                own_username: RwLock::new(None),
            }),
        };

        client.spawn_token_refresh_task();

        Ok(client)
    }
    pub async fn from_stored_token(
        client_id: u64,
        client_secret: String,
    ) -> Result<Self, SteelApplicationError> {
        let token_state =
            token_storage::load_token_state().map_err(|_| SteelApplicationError::InvalidOAuth)?;

        if !token_state.has_valid_token() {
            return Err(SteelApplicationError::InvalidOAuth);
        }

        let refresh_token = token_state
            .refresh_token
            .clone()
            .map(|s| s.into_boxed_str());

        let token = Token::new(&token_state.access_token, refresh_token);

        let expires_in = if token_state.is_access_token_valid() {
            let now = Utc::now();
            Some(
                token_state
                    .access_expires_at
                    .signed_duration_since(now)
                    .num_seconds(),
            )
        } else {
            None
        };

        Self::new(client_id, client_secret, token, expires_in).await
    }

    pub fn get_cached_user(&self, user_id: u32) -> Option<User> {
        self.inner.user_cache.read().get(&user_id).cloned()
    }

    pub fn insert_user(&self, user: &User) {
        let user_id = user.user_id;
        let username = user.username.to_string().normalize();

        self.inner.user_cache.write().insert(user_id, user.clone());
        self.inner.username_cache.write().insert(username, user_id);
    }

    pub fn get_cached_userid(&self, username: &str) -> Option<u32> {
        self.inner
            .username_cache
            .read()
            .get(&username.normalize())
            .cloned()
    }

    pub async fn get_or_fetch_user(
        &self,
        username: &str,
    ) -> Result<User, rosu_v2::error::OsuError> {
        if let Some(user_id) = self.get_cached_userid(username) {
            if let Some(user) = self.get_cached_user(user_id) {
                return Ok(user);
            }
        }

        let user: User = self
            .inner
            .api
            .read()
            .await
            .user(UserId::Name(format!("@{}", username).into()))
            .await?
            .into();

        self.insert_user(&user);

        Ok(user)
    }

    pub fn get_cached_channel(&self, channel_id: ChatChannelId) -> Option<ChatChannel> {
        self.inner.channel_cache.read().get(&channel_id).cloned()
    }

    pub fn insert_channel(&self, channel: &ChatChannel) {
        let channel_id = channel.channel_id;
        let channel_name = channel.name.normalize();

        self.inner
            .channel_cache
            .write()
            .insert(channel_id, channel.clone());
        self.inner
            .channel_name_cache
            .write()
            .insert(channel_name, channel_id);
    }

    pub fn get_cached_channel_by_name(&self, channel_name: &str) -> Option<ChatChannel> {
        let name = channel_name.normalize();
        let channel_id = self.inner.channel_name_cache.read().get(&name).cloned()?;
        self.get_cached_channel(channel_id)
    }

    pub fn clear_caches(&self) {
        self.inner.user_cache.write().clear();
        self.inner.username_cache.write().clear();
        self.inner.channel_cache.write().clear();
        self.inner.channel_name_cache.write().clear();
    }

    fn spawn_token_refresh_task(&self) {
        let inner = Arc::clone(&self.inner);
        let mut shutdown_rx = self.inner.shutdown_tx.subscribe();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        Self::check_and_save_token(&inner).await;
                    }
                    _ = shutdown_rx.recv() => {
                        log::info!("Token refresh task: shutdown signal received");
                        Self::check_and_save_token(&inner).await;
                        break;
                    }
                }
            }

            log::info!("Token refresh task stopped");
        });
    }

    async fn check_and_save_token(inner: &Arc<ClientInner>) {
        log::info!("Token refresh task: running");
        let (new_access_token, new_refresh_token, stored_token) = {
            let api = inner.api.read().await;
            let access = api.token().access().map(|s| s.to_string());
            let refresh = api.token().refresh().map(|s| s.to_string());
            let stored = inner.current_token.read().clone();
            (access, refresh, stored)
        };

        if let Some(ref access_token) = new_access_token {
            if stored_token != *access_token {
                log::info!("Token has been refreshed by rosu_v2, saving to disk");

                match token_storage::create_and_save_new_state(
                    access_token,
                    new_refresh_token.as_deref(),
                ) {
                    Ok(new_state) => {
                        *inner.current_token.write() = access_token.clone();
                        *inner.token_expires_at.write() = new_state.access_expires_at;
                        log::info!(
                            "Token saved successfully, expires at {}",
                            new_state.access_expires_at
                        );
                    }
                    Err(e) => {
                        log::error!("Failed to save refreshed token: {}", e);
                    }
                }
            } else {
                log::debug!("Token unchanged during periodic check");
            }
        } else {
            log::warn!("API client has no access token during refresh check");
        }
    }

    pub async fn own_data(&self) -> Result<User, rosu_v2::error::OsuError> {
        let user = self.inner.api.read().await.own_data().await?;

        *self.inner.own_user_id.write() = Some(user.user_id);
        *self.inner.own_username.write() = Some(user.username.clone().into_string());

        let user: User = user.into();

        self.insert_user(&user);

        Ok(user)
    }

    pub async fn get_or_fetch_channel(
        &self,
        channel_id: ChatChannelId,
    ) -> Result<ChatChannel, rosu_v2::error::OsuError> {
        if let Some(channel) = self.get_cached_channel(channel_id) {
            return Ok(channel);
        }

        let channel_data = self.inner.api.read().await.chat_channel(channel_id).await?;

        self.insert_channel(&channel_data.channel);
        Ok(channel_data.channel)
    }

    pub async fn chat_send_message(
        &self,
        channel_id: ChatChannelId,
        content: String,
        is_action: bool,
    ) -> Result<(), rosu_v2::error::OsuError> {
        self.inner
            .api
            .read()
            .await
            .chat_send_message(channel_id, content, is_action)
            .await?;
        Ok(())
    }

    pub async fn chat_create_private_channel(
        &self,
        user_id: u32,
        content: String,
        is_action: bool,
    ) -> Result<ChatChannel, rosu_v2::error::OsuError> {
        let result = self
            .inner
            .api
            .read()
            .await
            .chat_create_private_channel(user_id, content, is_action)
            .await?;

        self.insert_channel(&result.channel);

        Ok(result.channel)
    }

    pub async fn chat_join_channel(
        &self,
        channel_id: ChatChannelId,
        user_id: u32,
    ) -> Result<rosu_v2::model::chat::ChatChannel, rosu_v2::error::OsuError> {
        let channel = self
            .inner
            .api
            .read()
            .await
            .chat_join_channel(channel_id, user_id)
            .await?;
        self.insert_channel(&channel);

        Ok(channel)
    }

    pub async fn chat_leave_channel(
        &self,
        channel_id: ChatChannelId,
        user_id: u32,
    ) -> Result<(), rosu_v2::error::OsuError> {
        self.inner
            .api
            .read()
            .await
            .chat_leave_channel(channel_id, user_id)
            .await
    }

    pub async fn chat_keepalive(&self) -> Result<(), rosu_v2::error::OsuError> {
        self.inner.api.read().await.chat_keepalive().await?;
        Ok(())
    }

    pub async fn get_current_token(&self) -> Token {
        let api = self.inner.api.read().await;
        let token = api.token();

        let access = token.access().map(|s| s.to_string().into_boxed_str());
        let refresh = token.refresh().map(|s| s.to_string().into_boxed_str());

        Token::new(access.as_deref().unwrap_or(""), refresh)
    }

    pub async fn get_access_token(&self) -> Option<String> {
        self.inner
            .api
            .read()
            .await
            .token()
            .access()
            .map(|s| s.to_string())
    }

    pub fn get_token_expires_at(&self) -> DateTime<Utc> {
        *self.inner.token_expires_at.read()
    }

    pub fn is_token_valid(&self) -> bool {
        let expires_at = *self.inner.token_expires_at.read();
        let now = Utc::now();
        let buffer = chrono::Duration::seconds(
            (expires_at.signed_duration_since(now).num_seconds() as f64 * 0.05) as i64,
        );
        now < (expires_at - buffer)
    }

    pub fn get_own_user_id(&self) -> Option<u32> {
        *self.inner.own_user_id.read()
    }

    pub fn get_own_username(&self) -> Option<String> {
        self.inner.own_username.read().clone()
    }

    pub fn set_own_user(&self, username: String, user_id: u32) {
        *self.inner.own_username.write() = Some(username);
        *self.inner.own_user_id.write() = Some(user_id);
    }

    pub fn shutdown(&self) {
        log::info!("Client shutdown requested");
        let _ = self.inner.shutdown_tx.send(());
    }

    pub fn is_shutdown_requested(&self) -> bool {
        self.inner.shutdown_tx.receiver_count() == 0
    }
}
