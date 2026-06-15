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
use steel_core::ipc::server::{AppMessageIn, ConnectionDetails};
use tokio::sync::broadcast;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::RwLock as AsyncRwLock;

use crate::core::error::SteelApplicationError;
use crate::core::http::jwt::AccessTokenTiming;
use crate::core::http::token_manager::TokenManager;
use crate::core::http::token_refresh::{TokenRefreshConfig, TokenRefreshError};
use crate::core::http::token_storage;
use crate::core::http::{send_progress, APISettings};
use steel_core::string_utils::UsernameString;

const TOKEN_REFRESH_RETRY_SECS: u64 = 5 * 60;

#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

struct ClientInner {
    api: AsyncRwLock<Osu>,

    tokens: TokenManager,
    ws_base_uri: String,
    output: UnboundedSender<AppMessageIn>,

    user_cache: RwLock<HashMap<u32, User>>,
    username_cache: RwLock<HashMap<String, u32>>,
    channel_cache: RwLock<HashMap<ChatChannelId, ChatChannel>>,
    channel_name_cache: RwLock<HashMap<String, ChatChannelId>>,

    shutdown_tx: broadcast::Sender<()>,

    own_user_id: RwLock<Option<u32>>,
    own_username: RwLock<Option<String>>,
}

impl Client {
    async fn new(
        settings: &APISettings,
        output: UnboundedSender<AppMessageIn>,
        tokens: TokenManager,
    ) -> Result<Self, SteelApplicationError> {
        let state = tokens.snapshot();
        let token = Token::new(
            &state.access_token,
            state.refresh_token.clone().map(|s| s.into_boxed_str()),
        );
        let api = Self::build_api(tokens.config(), token).await?;

        let (shutdown_tx, _) = broadcast::channel(1);

        let client = Self {
            inner: Arc::new(ClientInner {
                api: AsyncRwLock::new(api),
                tokens,
                ws_base_uri: settings.ws_base_uri.clone(),
                output,
                user_cache: RwLock::new(HashMap::new()),
                username_cache: RwLock::new(HashMap::new()),
                channel_cache: RwLock::new(HashMap::new()),
                channel_name_cache: RwLock::new(HashMap::new()),
                shutdown_tx,
                own_user_id: RwLock::new(None),
                own_username: RwLock::new(None),
            }),
        };

        client.spawn_token_refresh_task();

        Ok(client)
    }

    // work around rosu-v2 to direct the token refresh procedure towards the jump server
    async fn build_api(
        config: &TokenRefreshConfig,
        token: Token,
    ) -> Result<Osu, rosu_v2::error::OsuError> {
        rosu_v2::OsuBuilder::new()
            .client_id(config.client_id)
            .client_secret(config.client_secret.clone())
            .with_token(token, None)
            .build()
            .await
    }

    pub async fn from_stored_token(
        settings: &APISettings,
        output: UnboundedSender<AppMessageIn>,
    ) -> Result<Self, SteelApplicationError> {
        let stored =
            token_storage::load_token_state().map_err(|_| SteelApplicationError::InvalidOAuth)?;

        if !stored.has_valid_token() {
            return Err(SteelApplicationError::InvalidOAuth);
        }

        let rotation_threshold = chrono::Duration::days(settings.token_rotation_days as i64);
        let tokens = TokenManager::new(settings.token_refresh_config(), rotation_threshold, stored);

        let needs_rotation = tokens.refresh_token_needs_rotation();
        if tokens.is_access_token_valid() && !needs_rotation {
            send_progress(&output, "using the stored access token");
        } else {
            if needs_rotation {
                log::info!("The refresh token is about to expire, rotating the token pair");
                send_progress(
                    &output,
                    "renewing the session (the refresh token expires soon)",
                );
            } else {
                log::info!("Stored access token has expired, refreshing it before connecting");
                send_progress(&output, "the access token has expired, refreshing it");
            }

            tokens.refresh().await.map_err(|e| {
                log::error!("Failed to refresh tokens: {e}");
                if matches!(e, TokenRefreshError::Rejected(_)) {
                    // The refresh token has been revoked or is otherwise unusable --
                    // start from a clean slate, as if the user has never logged in.
                    if let Err(e) = token_storage::clear_token_state() {
                        log::error!("Failed to clear token state: {e}");
                    }
                }
                SteelApplicationError::TokenRefreshFailed(e.to_string())
            })?;

            send_progress(&output, "received a new token pair");
        }

        Self::new(settings, output, tokens).await
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

        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    log::error!("Failed to create a runtime for the token refresh task: {e}");
                    return;
                }
            };

            runtime.block_on(async move {
                let mut retry_delay: Option<std::time::Duration> = None;

                loop {
                    let wait = retry_delay.take().unwrap_or_else(|| {
                        inner
                            .tokens
                            .next_refresh_due()
                            .signed_duration_since(Utc::now())
                            .to_std()
                            .unwrap_or(std::time::Duration::ZERO)
                            .max(std::time::Duration::from_secs(1))
                    });

                    tokio::select! {
                        _ = tokio::time::sleep(wait) => {
                            if Utc::now() < inner.tokens.next_refresh_due() {
                                continue;
                            }

                            if let Err(e) = Self::refresh_and_apply(&inner).await {
                                log::error!("Scheduled token refresh failed: {e}");
                                retry_delay = Some(std::time::Duration::from_secs(TOKEN_REFRESH_RETRY_SECS));
                            }
                        }
                        _ = shutdown_rx.recv() => {
                            log::info!("Token refresh task: shutdown signal received");
                            break;
                        }
                    }
                }

                log::info!("Token refresh task stopped");
            });
        });
    }

    pub async fn refresh_token_now(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Self::refresh_and_apply(&self.inner).await
    }

    async fn refresh_and_apply(
        inner: &Arc<ClientInner>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let new_state = inner.tokens.refresh().await?;

        let token = Token::new(
            &new_state.access_token,
            new_state.refresh_token.clone().map(|s| s.into_boxed_str()),
        );
        let api = Self::build_api(inner.tokens.config(), token).await?;
        *inner.api.write().await = api;

        inner
            .output
            .send(AppMessageIn::connection_details_changed(
                ConnectionDetails::API {
                    server: inner.ws_base_uri.clone(),
                    token_expires_at: new_state.access_expires_at,
                    refresh_token_expires_at: new_state
                        .refresh_token
                        .as_ref()
                        .map(|_| new_state.refresh_expires_at),
                },
            ))
            .unwrap_or_else(|e| log::error!("Failed to send connection details: {e}"));

        Ok(())
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

    pub async fn chat_channels(&self) -> Result<Vec<ChatChannel>, rosu_v2::error::OsuError> {
        self.inner.api.read().await.chat_channels().await
    }

    pub async fn chat_keepalive(&self) -> Result<(), rosu_v2::error::OsuError> {
        self.inner.api.read().await.chat_keepalive().await?;
        Ok(())
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
        self.inner.tokens.access_token_expires_at()
    }

    pub fn get_refresh_token_expires_at(&self) -> Option<DateTime<Utc>> {
        self.inner.tokens.refresh_token_expires_at()
    }

    pub fn access_token_timing(&self) -> Option<AccessTokenTiming> {
        self.inner.tokens.access_token_timing()
    }

    pub fn is_refresh_token_usable(&self) -> bool {
        self.inner.tokens.is_refresh_token_usable()
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
