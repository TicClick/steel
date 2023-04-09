use std::fmt;

use chrono::{DateTime, Utc};

use super::DEFAULT_DATE_FORMAT;

pub enum EventCategory {
    IRC,
    Application,
}

pub enum EventType {
    ConnectionChanged,
    IRCError,
}

pub struct EventDetails {
    pub message: String,
}

impl fmt::Display for EventDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub enum EventSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Panic,
}

impl fmt::Display for EventSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Debug => "debug",
                Self::Info => "info",
                Self::Warning => "warning",
                Self::Error => "error",
                Self::Panic => "panic",
            }
        )
    }
}

pub struct Event {
    pub time: DateTime<Utc>,
    pub severity: EventSeverity,
    pub details: EventDetails,
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} <{}> {}",
            self.time.format(DEFAULT_DATE_FORMAT),
            self.severity,
            self.details
        )
    }
}

impl Event {
    pub fn new(severity: EventSeverity, details: EventDetails) -> Self {
        Self {
            time: Utc::now(),
            severity,
            details,
        }
    }
}

#[derive(Default)]
pub struct Logger {
    pub irc: Vec<Event>,
    pub app: Vec<Event>,
}

impl Logger {
    // TODO: save to file first and foremost
    pub fn log(&mut self, category: EventCategory, severity: EventSeverity, details: EventDetails) {
        let event = Event::new(severity, details);
        match category {
            EventCategory::IRC => self.irc.push(event),
            EventCategory::Application => self.app.push(event),
        }
    }

    pub fn log_irc(&mut self, severity: EventSeverity, details: EventDetails) {
        self.log(EventCategory::IRC, severity, details)
    }

    pub fn log_app(&mut self, severity: EventSeverity, details: EventDetails) {
        self.log(EventCategory::Application, severity, details)
    }
}
