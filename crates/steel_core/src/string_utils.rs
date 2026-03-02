use std::fmt;

pub trait UsernameString {
    fn normalize(&self) -> String;
    fn as_username_key(&self) -> UsernameKey;
}

impl UsernameString for &str {
    fn normalize(&self) -> String {
        self.to_lowercase().replace(' ', "_")
    }

    fn as_username_key(&self) -> UsernameKey {
        UsernameKey::new(self)
    }
}

impl UsernameString for String {
    fn normalize(&self) -> String {
        self.as_str().normalize()
    }

    fn as_username_key(&self) -> UsernameKey {
        self.as_str().as_username_key()
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct UsernameKey(String);

impl UsernameKey {
    pub fn new(username: &str) -> Self {
        Self(username.normalize())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_same_username(&self, username: &str) -> bool {
        self.0 == username.normalize()
    }
}

impl fmt::Display for UsernameKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for UsernameKey {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for UsernameKey {
    fn from(s: String) -> Self {
        Self::new(&s)
    }
}

impl std::borrow::Borrow<str> for UsernameKey {
    fn borrow(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct ChatKey(String);

impl ChatKey {
    pub fn new(name: &str) -> Self {
        Self(name.normalize())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ChatKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ChatKey {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for ChatKey {
    fn from(s: String) -> Self {
        Self::new(&s)
    }
}

impl std::borrow::Borrow<str> for ChatKey {
    fn borrow(&self) -> &str {
        &self.0
    }
}
