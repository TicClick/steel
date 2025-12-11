pub mod chat;
pub mod error;
pub mod ipc;
pub mod settings;
pub mod string_utils;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// https://docs.rs/chrono/latest/chrono/format/strftime/index.html
pub const DEFAULT_TIME_FORMAT: &str = "%H:%M:%S";
pub const DEFAULT_DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
pub const DATETIME_FORMAT_WITH_TZ: &str = "%Y-%m-%d %H:%M:%S (UTC %:z)";
pub const DEFAULT_DATE_FORMAT: &str = "%Y-%m-%d";

pub trait VersionString {
    fn semver(&self) -> (u8, u8, u8);
}

impl VersionString for &str {
    fn semver(&self) -> (u8, u8, u8) {
        let mut version_string = &self[..];
        if version_string.starts_with('v') {
            version_string = &version_string[1..];
        }

        let mut parts: Vec<u8> = version_string
            .split('.')
            .filter_map(|i| i.parse::<u8>().ok())
            .collect();
        while parts.len() < 3 {
            parts.push(0);
        }
        (parts[0], parts[1], parts[2])
    }
}

impl VersionString for String {
    fn semver(&self) -> (u8, u8, u8) {
        self.as_str().semver()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TextStyle {
    Bold,
    Italics,
    Underline,
    Strikethrough,
    Monospace,

    Highlight(ecolor::Color32),
    Coloured(ecolor::Color32),
    SearchResult(ecolor::Color32),
}
