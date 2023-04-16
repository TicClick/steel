use serde::{Deserialize, Serialize};
use std::fmt::Display;

use crate::core::settings::colour::Colour;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Notifications {
    pub highlights: Highlights,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Highlights {
    pub colour: Colour,
    pub words: Vec<String>,
    pub sound: Option<Sound>,
}

impl Default for Highlights {
    fn default() -> Self {
        Self {
            colour: Colour::from_rgb(250, 200, 255),
            words: Vec::default(),
            sound: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BuiltInSound {
    #[default]
    Bell,
    DoubleBell,
    PartyHorn,
    Ping,
    Tick,
    TwoTone,
}

impl Display for BuiltInSound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Bell => "bell",
                Self::DoubleBell => "double bell",
                Self::PartyHorn => "party horn",
                Self::Ping => "ping",
                Self::Tick => "tick",
                Self::TwoTone => "two-tone",
            }
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Sound {
    BuiltIn(BuiltInSound),
}

impl Display for Sound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::BuiltIn(s) => format!("built-in ({})", s),
            }
        )
    }
}
