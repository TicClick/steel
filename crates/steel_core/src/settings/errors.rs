use std::fmt;

#[derive(Debug)]
pub enum SettingsError {
    IoError(String, std::io::Error),
    YamlError(String, serde_yaml::Error),
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingsError::IoError(details, err) => write!(f, "{}: {}", details, err),
            SettingsError::YamlError(details, err) => write!(f, "{}: {}", details, err),
        }
    }
}

impl std::error::Error for SettingsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SettingsError::IoError(_, err) => Some(err),
            SettingsError::YamlError(_, err) => Some(err),
        }
    }
}

impl From<std::io::Error> for SettingsError {
    fn from(err: std::io::Error) -> Self {
        SettingsError::IoError("I/O error while loading settings".into(), err)
    }
}

impl From<serde_yaml::Error> for SettingsError {
    fn from(err: serde_yaml::Error) -> Self {
        SettingsError::YamlError("YAML parsing error".into(), err)
    }
}
