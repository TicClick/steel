use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Application {
    #[serde(default)]
    pub autoupdate: AutoUpdate,
    #[serde(default)]
    pub window: WindowGeometry,
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
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self {
            x: 600,
            y: 400,
            height: 600,
            width: 800,
        }
    }
}
