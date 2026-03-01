use serde::{Deserialize, Serialize};
use std::fmt::Display;

pub struct NotificationParams {
    pub is_private_message: bool,
    pub is_message_highlighted: bool,
    pub is_window_focused: bool,
    pub is_sound_configured: bool,
    pub is_system_message: bool,
    pub is_own_message: bool,
}

pub struct NotificationOutcome {
    pub flash_window: bool,
    pub play_sound: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Notifications {
    pub highlights: Highlights,
    pub notification_events: NotificationEvents,
    pub sound_only_when_unfocused: bool,
    pub enable_notification_timeout: bool,
    pub notification_timeout_seconds: u32,
    pub notification_style: NotificationStyle,
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            highlights: Highlights::default(),
            notification_events: NotificationEvents::default(),
            sound_only_when_unfocused: false,
            enable_notification_timeout: false,
            notification_timeout_seconds: 10,
            notification_style: NotificationStyle::default(),
        }
    }
}

impl Notifications {
    pub fn evaluate(&self, params: &NotificationParams) -> NotificationOutcome {
        if params.is_system_message || params.is_own_message {
            return NotificationOutcome {
                flash_window: false,
                play_sound: false,
            };
        }

        let is_notifications_enabled = (params.is_message_highlighted
            && self.notification_events.highlights)
            || (params.is_private_message && self.notification_events.private_messages);

        NotificationOutcome {
            flash_window: is_notifications_enabled && !params.is_window_focused,
            play_sound: params.is_sound_configured
                && is_notifications_enabled
                && (!self.sound_only_when_unfocused || !params.is_window_focused),
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
    Custom(std::path::PathBuf),
}

impl Display for Sound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::BuiltIn(s) => format!("built-in ({s})"),
                Self::Custom(path) => path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("(unknown file)")
                    .to_string(),
            }
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NotificationEvents {
    pub highlights: bool,
    pub private_messages: bool,
}

impl Default for NotificationEvents {
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
    #[cfg_attr(not(target_os = "linux"), default)]
    Intensive,
    #[cfg_attr(target_os = "linux", default)]
    Moderate,
}

impl Display for NotificationStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            if cfg!(target_os = "windows") {
                match self {
                    Self::Intensive => "flash window",
                    Self::Moderate => "flash taskbar icon",
                }
            } else if cfg!(target_os = "macos") {
                match self {
                    Self::Intensive => "jump many times in dock",
                    Self::Moderate => "jump once in dock",
                }
            } else if cfg!(target_os = "linux") {
                match self {
                    Self::Intensive => "(unsupported)",
                    Self::Moderate => "flash taskbar icon",
                }
            } else {
                match self {
                    Self::Intensive => "flash window (unsupported)",
                    Self::Moderate => "flash taskbar icon (unsupported)",
                }
            }
        )
    }
}
