use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Notifications {
    pub highlights: Highlights,
    pub taskbar_flash_events: TaskbarFlashEvents,
    pub sound_only_when_unfocused: bool,
    pub enable_flash_timeout: bool,
    pub flash_timeout_seconds: u32,
    pub notification_style: NotificationStyle,
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            highlights: Highlights::default(),
            taskbar_flash_events: TaskbarFlashEvents::default(),
            sound_only_when_unfocused: false,
            enable_flash_timeout: false,
            flash_timeout_seconds: 10,
            notification_style: NotificationStyle::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Highlights {
    pub words: Vec<String>,
    pub sound: Option<Sound>,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaskbarFlashEvents {
    pub highlights: bool,
    pub private_messages: bool,
}

impl Default for TaskbarFlashEvents {
    fn default() -> Self {
        Self {
            highlights: true,
            private_messages: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NotificationStyle {
    #[default]
    WindowAndTaskbar,
    TaskbarOnly,
}

impl Display for NotificationStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::WindowAndTaskbar => "window + taskbar",
                Self::TaskbarOnly => "taskbar",
            }
        )
    }
}
