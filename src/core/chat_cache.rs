use rosu_v2::{
    model::chat::{ChatChannel, ChatChannelId},
    prelude::User,
};
use std::collections::HashMap;

use steel_core::string_utils::UsernameString;

#[derive(Debug, Default)]
pub struct ChatCache {
    user_cache: HashMap<u32, User>,
    username_cache: HashMap<String, u32>,
    channel_cache: HashMap<ChatChannelId, ChatChannel>,
}

impl ChatCache {
    pub fn new() -> Self {
        Self {
            user_cache: HashMap::new(),
            username_cache: HashMap::new(),
            channel_cache: HashMap::new(),
        }
    }

    pub fn get_user(&self, user_id: u32) -> Option<&User> {
        self.user_cache.get(&user_id)
    }

    pub fn insert_user(&mut self, user: User) {
        self.username_cache
            .insert(user.username.to_string().normalize(), user.user_id);
        self.user_cache.insert(user.user_id, user);
    }

    pub fn insert_users(&mut self, users: Vec<User>) {
        self.username_cache.extend(
            users
                .iter()
                .cloned()
                .map(|u| (u.username.to_string().normalize(), u.user_id)),
        );
        self.user_cache
            .extend(users.into_iter().map(|u| (u.user_id, u)));
    }

    pub fn get_username(&self, user_id: u32) -> Option<String> {
        self.user_cache
            .get(&user_id)
            .map(|u| u.username.clone().into_string())
    }

    pub fn get_user_by_username(&self, username: &str) -> Option<u32> {
        self.username_cache.get(&username.normalize()).cloned()
    }

    pub fn get_channel(&self, channel_id: ChatChannelId) -> Option<&ChatChannel> {
        self.channel_cache.get(&channel_id)
    }

    pub fn find_channel(&self, channel_name: &str) -> Option<&ChatChannel> {
        let name = channel_name.to_lowercase();
        self.channel_cache.values().find(|ch| ch.name == name)
    }

    pub fn insert_channel(&mut self, channel: ChatChannel) {
        self.channel_cache.insert(channel.channel_id, channel);
    }

    pub fn insert_channels(&mut self, channels: Vec<ChatChannel>) {
        self.channel_cache
            .extend(channels.into_iter().map(|ch| (ch.channel_id, ch)));
    }

    pub fn get_channel_name(&self, channel_id: ChatChannelId) -> Option<String> {
        self.channel_cache.get(&channel_id).map(|c| c.name.clone())
    }

    pub fn clear(&mut self) {
        self.user_cache.clear();
        self.channel_cache.clear();
    }
}
