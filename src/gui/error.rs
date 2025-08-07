use std::fmt;

#[derive(Debug)]
pub enum GuiError {
    MessageNotFound {
        message_id: usize,
        chat_name: String,
    },
    ChatNotFound {
        chat_name: String,
    },
}

impl fmt::Display for GuiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GuiError::MessageNotFound {
                message_id,
                chat_name,
            } => {
                write!(f, "Failed to find message #{message_id} in chat {chat_name} -- it is probably already closed")
            }
            GuiError::ChatNotFound { chat_name } => {
                write!(f, "Chat '{chat_name}' not found")
            }
        }
    }
}

impl std::error::Error for GuiError {}

impl GuiError {
    pub fn message_not_found(message_id: usize, chat_name: impl Into<String>) -> Self {
        Self::MessageNotFound {
            message_id,
            chat_name: chat_name.into(),
        }
    }

    pub fn chat_not_found(chat_name: impl Into<String>) -> Self {
        Self::ChatNotFound {
            chat_name: chat_name.into(),
        }
    }
}
