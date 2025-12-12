use rosu_v2::{
    model::chat::{ChatChannel, ChatChannelId},
    prelude::User,
    request::UserId,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use steel_core::string_utils::UsernameString;

pub struct ChatCache {
    user_cache: RwLock<HashMap<u32, User>>,
    username_cache: RwLock<HashMap<String, u32>>,
    channel_cache: RwLock<HashMap<ChatChannelId, ChatChannel>>,
    api: RwLock<Option<Arc<rosu_v2::Osu>>>,
}

impl ChatCache {
    pub fn new() -> Self {
        Self {
            user_cache: RwLock::new(HashMap::new()),
            username_cache: RwLock::new(HashMap::new()),
            channel_cache: RwLock::new(HashMap::new()),
            api: RwLock::new(None),
        }
    }

    pub fn set_api(&self, api: Arc<rosu_v2::Osu>) {
        if let Ok(mut api_lock) = self.api.write() {
            *api_lock = Some(api);
        }
    }

    pub fn get_user(&self, user_id: u32) -> Option<User> {
        self.user_cache
            .read()
            .ok()
            .and_then(|cache| cache.get(&user_id).cloned())
    }

    pub fn insert_user(&self, user: User) {
        let user_id = user.user_id;
        let username = user.username.to_string().normalize();

        if let Ok(mut user_cache) = self.user_cache.write() {
            user_cache.insert(user_id, user);
        }
        if let Ok(mut username_cache) = self.username_cache.write() {
            username_cache.insert(username, user_id);
        }
    }

    pub fn insert_users(&self, users: Vec<User>) {
        let username_pairs: Vec<_> = users
            .iter()
            .map(|u| (u.username.to_string().normalize(), u.user_id))
            .collect();

        if let Ok(mut username_cache) = self.username_cache.write() {
            username_cache.extend(username_pairs);
        }
        if let Ok(mut user_cache) = self.user_cache.write() {
            user_cache.extend(users.into_iter().map(|u| (u.user_id, u)));
        }
    }

    pub fn get_username(&self, user_id: u32) -> Option<String> {
        self.user_cache.read().ok().and_then(|cache| {
            cache
                .get(&user_id)
                .map(|u| u.username.clone().into_string())
        })
    }

    pub fn get_user_by_username(&self, username: &str) -> Option<u32> {
        self.username_cache
            .read()
            .ok()
            .and_then(|cache| cache.get(&username.normalize()).cloned())
    }

    pub fn get_channel(&self, channel_id: ChatChannelId) -> Option<ChatChannel> {
        self.channel_cache
            .read()
            .ok()
            .and_then(|cache| cache.get(&channel_id).cloned())
    }

    pub fn find_channel(&self, channel_name: &str) -> Option<ChatChannel> {
        let name = channel_name.to_lowercase();
        self.channel_cache
            .read()
            .ok()
            .and_then(|cache| cache.values().find(|ch| ch.name == name).cloned())
    }

    pub fn insert_channel(&self, channel: ChatChannel) {
        if let Ok(mut channel_cache) = self.channel_cache.write() {
            channel_cache.insert(channel.channel_id, channel);
        }
    }

    pub fn insert_channels(&self, channels: Vec<ChatChannel>) {
        if let Ok(mut channel_cache) = self.channel_cache.write() {
            channel_cache.extend(channels.into_iter().map(|ch| (ch.channel_id, ch)));
        }
    }

    pub fn get_channel_name(&self, channel_id: ChatChannelId) -> Option<String> {
        self.channel_cache
            .read()
            .ok()
            .and_then(|cache| cache.get(&channel_id).map(|c| c.name.clone()))
    }

    pub fn clear(&self) {
        if let Ok(mut user_cache) = self.user_cache.write() {
            user_cache.clear();
        }
        if let Ok(mut username_cache) = self.username_cache.write() {
            username_cache.clear();
        }
        if let Ok(mut channel_cache) = self.channel_cache.write() {
            channel_cache.clear();
        }
    }

    pub async fn get_or_fetch_user_by_username(&self, username: &str) -> Result<u32, String> {
        if let Some(uid) = self.get_user_by_username(username) {
            return Ok(uid);
        }

        let api = self
            .api
            .read()
            .map_err(|e| format!("Failed to acquire API lock: {e}"))?
            .clone()
            .ok_or_else(|| "API not initialized".to_string())?;

        let user = api
            .user(UserId::Name(format!("@{username}").into()))
            .await
            .map_err(|e| format!("Failed to fetch user from API: {e}"))?;

        let user_id = user.user_id;
        self.insert_user(user.into());

        Ok(user_id)
    }

    pub async fn get_or_fetch_channel(
        &self,
        channel_id: ChatChannelId,
    ) -> Result<ChatChannel, String> {
        if let Some(channel) = self.get_channel(channel_id) {
            return Ok(channel);
        }

        let api = self
            .api
            .read()
            .map_err(|e| format!("Failed to acquire API lock: {e}"))?
            .clone()
            .ok_or_else(|| "API not initialized".to_string())?;

        let chat_info = api
            .chat_channel(channel_id)
            .await
            .map_err(|e| format!("Failed to fetch channel from API: {e}"))?;

        let channel = chat_info.channel;
        self.insert_channel(channel.clone());

        Ok(channel)
    }
}

impl Default for ChatCache {
    fn default() -> Self {
        Self::new()
    }
}
