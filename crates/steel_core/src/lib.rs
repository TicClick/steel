pub mod chat;
pub mod ipc;
pub mod settings;

// https://docs.rs/chrono/latest/chrono/format/strftime/index.html

pub const DEFAULT_TIME_FORMAT: &str = "%H:%M:%S";
pub const DEFAULT_DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
pub const DATETIME_FORMAT_WITH_TZ: &str = "%Y-%m-%d %H:%M:%S (UTC %:z)";
pub const DEFAULT_DATE_FORMAT: &str = "%Y-%m-%d";
