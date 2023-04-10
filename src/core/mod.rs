pub mod chat;
pub mod irc;
pub mod logger;
pub mod settings;

// https://docs.rs/chrono/latest/chrono/format/strftime/index.html

const DEFAULT_TIME_FORMAT: &str = "%H:%M:%S";
const DEFAULT_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
