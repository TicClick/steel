pub mod chat;
pub mod irc;
pub mod settings;
pub mod sound;
pub mod updater;

// https://docs.rs/chrono/latest/chrono/format/strftime/index.html

const DEFAULT_TIME_FORMAT: &str = "%H:%M:%S";
const DEFAULT_DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const DATETIME_FORMAT_WITH_TZ: &str = "%Y-%m-%d %H:%M:%S (UTC %:z)";
pub const DEFAULT_DATE_FORMAT: &str = "%Y-%m-%d";
