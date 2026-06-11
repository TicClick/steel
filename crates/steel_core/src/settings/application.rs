use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Application {
    #[serde(default)]
    pub autoupdate: AutoUpdate,
    #[serde(default)]
    pub window: WindowGeometry,
    #[serde(default)]
    pub detached_chat_windows: BTreeMap<String, DetachedWindowGeometry>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AutoUpdate {
    pub enabled: bool,
    #[serde(default)]
    pub url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    #[serde(default)]
    pub maximized: bool,
    #[serde(default)]
    pub sidebar_width: Option<f32>,
    #[serde(default)]
    pub channels_panel_height: Option<f32>,
    #[serde(default)]
    pub private_chats_panel_height: Option<f32>,
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self {
            x: 600,
            y: 400,
            height: 600,
            width: 800,
            maximized: false,
            sidebar_width: None,
            channels_panel_height: None,
            private_chats_panel_height: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetachedWindowGeometry {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: i32,
    pub height: i32,
}

impl Default for DetachedWindowGeometry {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            width: 600,
            height: 400,
        }
    }
}
