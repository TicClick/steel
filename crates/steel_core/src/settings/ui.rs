use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::settings::colour::Colour;

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
    pub highlight: Colour,
    #[serde(default)]
    pub read_tabs: Colour,
    #[serde(default)]
    pub unread_tabs: Colour,
    pub default_users: Colour,
    pub custom_users: BTreeMap<String, Colour>,
}

impl Default for ChatColours {
    /// `egui::style::Widgets::dark()`
    fn default() -> Self {
        Self {
            own: Colour::from_rgb(250, 214, 60),
            highlight: Colour::from_rgb(250, 214, 60),
            read_tabs: Colour::from_rgb(120, 120, 120),
            unread_tabs: Colour::from_rgb(255, 255, 255),
            default_users: Colour::from_rgb(180, 180, 180),
            custom_users: BTreeMap::default(),
        }
    }
}

impl ChatColours {
    pub fn dark() -> Self {
        Self::default()
    }

    /// `egui::style::Widgets::light()`
    pub fn light() -> Self {
        Self {
            own: Colour::from_rgb(0, 132, 200),
            highlight: Colour::from_rgb(200, 77, 77),
            read_tabs: Colour::from_rgb(120, 120, 120),
            unread_tabs: Colour::from_rgb(0, 0, 0),
            default_users: Colour::from_rgb(60, 60, 60),
            ..Default::default()
        }
    }

    pub fn username_colour(&self, username: &str) -> &Colour {
        match self.custom_users.get(username) {
            Some(colour) => colour,
            None => &self.default_users,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct UI {
    pub theme: ThemeMode,
    pub light_colours: ChatColours,
    pub dark_colours: ChatColours,
}

impl Default for UI {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Dark,
            light_colours: ChatColours::light(),
            dark_colours: ChatColours::dark(),
        }
    }
}

impl UI {
    pub fn colours_mut(&mut self) -> &mut ChatColours {
        match self.theme {
            ThemeMode::Dark => &mut self.dark_colours,
            ThemeMode::Light => &mut self.light_colours,
        }
    }

    pub fn colours(&self) -> &ChatColours {
        match self.theme {
            ThemeMode::Dark => &self.dark_colours,
            ThemeMode::Light => &self.light_colours,
        }
    }
}
