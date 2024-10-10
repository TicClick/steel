use std::collections::{hash_map::Entry, HashMap};
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::Write as IOWrite;
use std::path::{Path, PathBuf};

use steel_core::chat::{Message, MessageType};
use steel_core::DEFAULT_DATETIME_FORMAT;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::actor::ActorHandle;

pub enum LoggingRequest {
    LogMessage { chat_name: String, message: Message },
    CloseLog { chat_name: String },
    ChangeLogFormat { log_line_format: String },
    ChangeLoggingDirectory { logging_directory: String },
    ShutdownLogger,
}

pub struct ChatLoggerHandle {
    channel: UnboundedSender<LoggingRequest>,
    log_system_messages: bool,
}

impl ActorHandle for ChatLoggerHandle {}

impl ChatLoggerHandle {
    pub fn new(log_directory: &str, log_line_format: &str) -> Self {
        let (tx, rx) = unbounded_channel();
        let mut actor = ChatLoggerBackend::new(log_directory, log_line_format, rx);
        std::thread::spawn(move || {
            actor.run();
        });
        Self {
            channel: tx,
            log_system_messages: true,
        }
    }

    pub fn log_system_messages(&mut self, log: bool) {
        self.log_system_messages = log;
    }

    pub fn log(&self, chat_name: &str, message: &Message) {
        if matches!(message.r#type, MessageType::System) && !self.log_system_messages {
            return;
        }

        let _ = self.channel.send(LoggingRequest::LogMessage {
            chat_name: chat_name.to_owned(),
            message: message.to_owned(),
        });
    }

    pub fn close_log(&self, chat_name: String) {
        let _ = self.channel.send(LoggingRequest::CloseLog { chat_name });
    }

    pub fn change_log_format(&self, log_line_format: String) {
        let _ = self
            .channel
            .send(LoggingRequest::ChangeLogFormat { log_line_format });
    }

    pub fn change_logging_directory(&self, logging_directory: String) {
        let _ = self
            .channel
            .send(LoggingRequest::ChangeLoggingDirectory { logging_directory });
    }

    pub fn shutdown(&self) {
        let _ = self.channel.send(LoggingRequest::ShutdownLogger);
    }
}

struct ChatLoggerBackend {
    log_directory: PathBuf,
    log_line_format: String,
    log_system_line_format: String,
    channel: UnboundedReceiver<LoggingRequest>,
    files: HashMap<PathBuf, File>,
}

impl ChatLoggerBackend {
    fn new(
        log_directory: &str,
        log_line_format: &str,
        channel: UnboundedReceiver<LoggingRequest>,
    ) -> Self {
        Self {
            log_directory: Path::new(log_directory).to_path_buf(),
            log_line_format: log_line_format.to_owned(),
            log_system_line_format: to_log_system_line_format(log_line_format),
            channel,
            files: HashMap::new(),
        }
    }

    fn run(&mut self) {
        while let Some(evt) = self.channel.blocking_recv() {
            match evt {
                LoggingRequest::LogMessage { chat_name, message } => {
                    if self.log(chat_name, message).is_err() {
                        return;
                    }
                }
                LoggingRequest::ChangeLogFormat { log_line_format } => {
                    self.log_system_line_format = to_log_system_line_format(&log_line_format);
                    self.log_line_format = log_line_format;
                }
                LoggingRequest::ChangeLoggingDirectory { logging_directory } => {
                    log::info!(
                        "Chat logging directory has been changed: {:?} -> {}",
                        self.log_directory,
                        logging_directory
                    );
                    self.log_directory = Path::new(&logging_directory).to_path_buf();
                    self.files.clear();
                }
                LoggingRequest::CloseLog { chat_name } => self.close(chat_name),
                LoggingRequest::ShutdownLogger => return,
            }
        }
    }

    fn chat_path(&self, chat_name: &str) -> PathBuf {
        self.log_directory
            .join(chat_name.to_lowercase())
            .with_extension("log")
    }

    fn log(&mut self, chat_name: String, message: Message) -> std::io::Result<()> {
        if self.files.is_empty() {
            if let Err(e) = std::fs::create_dir_all(&self.log_directory) {
                log::error!(
                    "Failed to create the directory for storing chat logs: {}",
                    e
                );
                return Err(e);
            }
        }

        let target_path = self.chat_path(&chat_name);
        let (is_new_file, mut f) = match self.files.entry(target_path.clone()) {
            Entry::Occupied(e) => (false, e.into_mut()),
            Entry::Vacant(e) => {
                match std::fs::OpenOptions::new()
                    .read(true)
                    .create(true)
                    .append(true)
                    .open(target_path)
                {
                    Ok(handle) => (true, e.insert(handle)),
                    Err(e) => {
                        log::error!(
                            "Failed to open or create the chat log for {}: {}",
                            chat_name,
                            e
                        );
                        return Err(e);
                    }
                }
            }
        };

        if is_new_file {
            if let Err(e) = writeln!(&mut f, "\n") {
                log::error!(
                    "Failed to start a new logging session for {}: {}",
                    chat_name,
                    e
                );
                return Err(e);
            }
        }

        let log_line_format = match message.r#type {
            MessageType::System => &self.log_system_line_format,
            _ => &self.log_line_format,
        };
        let formatted_message = format_message_for_logging(log_line_format, &message);
        if let Err(e) = writeln!(&mut f, "{}", formatted_message) {
            log::error!("Failed to append a chat log line for {}: {}", chat_name, e);
            return Err(e);
        }

        Ok(())
    }

    fn close(&mut self, chat_name: String) {
        let target_path = self.chat_path(&chat_name);
        if let Entry::Occupied(e) = self.files.entry(target_path) {
            e.remove_entry();
        }
    }
}

pub fn format_message_for_logging(log_line_format: &str, message: &Message) -> String {
    let mut result = String::new();
    let mut placeholder = String::new();
    let mut in_placeholder = false;

    for c in log_line_format.chars() {
        match c {
            '{' => {
                in_placeholder = true;
                placeholder.clear();
            }
            '}' => {
                if in_placeholder {
                    result.push_str(&resolve_placeholder(&placeholder, message));
                    in_placeholder = false;
                } else {
                    result.push(c);
                }
            }
            _ => {
                if in_placeholder {
                    placeholder.push(c);
                } else {
                    result.push(c);
                }
            }
        }
    }

    result
}

fn resolve_placeholder(placeholder: &str, message: &Message) -> String {
    if let Some(date_format) = placeholder.strip_prefix("date:") {
        let mut buf = String::new();
        match write!(&mut buf, "{}", message.time.format(date_format)) {
            Ok(_) => buf,
            Err(_) => format!("{{date:{}}}", date_format),
        }
    } else {
        match placeholder {
            "username" => message.username.clone(),
            "text" => message.text.clone(),
            _ => String::from("{unknown}"),
        }
    }
}

fn to_log_system_line_format(log_line_format: &str) -> String {
    if let Some(start_pos) = log_line_format.find("{date:") {
        if let Some(pos) = &log_line_format[start_pos..].find('}') {
            let end_pos = start_pos + *pos;
            let date_format = log_line_format[start_pos..end_pos + 1].to_owned();
            return format!("{} * {{text}}", date_format);
        }
    }
    format!("{{date:{}}} * {{text}}", DEFAULT_DATETIME_FORMAT)
}
