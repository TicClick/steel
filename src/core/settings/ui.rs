use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::core::settings::colour::Colour;

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ChatColours {
    pub own: Colour,
    pub users: BTreeMap<String, Colour>,
}

impl Default for ChatColours {
    fn default() -> Self {
        Self {
            own: Colour::from_rgb(200, 255, 250),
            users: BTreeMap::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct UI {
    pub theme: ThemeMode,
    pub colours: ChatColours,
}
