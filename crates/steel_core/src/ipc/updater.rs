use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io;

#[derive(Debug)]
pub enum ArchiveType {
    Zip,
    TarGZip,
    Unknown(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: usize,
}

impl ReleaseAsset {
    pub fn archive_type(&self) -> ArchiveType {
        let path = std::path::Path::new(&self.browser_download_url);
        match path.extension() {
            None => ArchiveType::Unknown("".into()),
            Some(ext) => {
                if let Some(s) = ext.to_str() {
                    match s {
                        "gz" => ArchiveType::TarGZip,
                        "zip" => ArchiveType::Zip,
                        e => ArchiveType::Unknown(e.into()),
                    }
                } else {
                    ArchiveType::Unknown("".into())
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReleaseMetadataGitHub {
    pub tag_name: String,
    pub published_at: chrono::DateTime<chrono::Utc>,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReleaseMetadataGist {
    pub files: BTreeMap<String, FileMetadataGist>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileMetadataGist {
    pub filename: String,
    pub r#type: String,
    pub raw_url: String,
    pub size: u64,
}

impl ReleaseMetadataGitHub {
    pub fn platform_specific_asset(&self) -> Option<&ReleaseAsset> {
        let os_marker = if cfg!(target_os = "windows") {
            "-windows"
        } else if cfg!(target_os = "macos") {
            "-darwin"
        } else if cfg!(target_os = "linux") {
            "-linux"
        } else {
            ""
        };

        if os_marker.is_empty() {
            return None;
        }
        return self.assets.iter().find(|a| a.name.contains(os_marker));
    }

    pub fn size(&self) -> usize {
        match self.platform_specific_asset() {
            Some(a) => a.size,
            None => 0,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub enum State {
    #[default]
    Idle,
    FetchingMetadata,
    MetadataReady(ReleaseMetadataGitHub),
    FetchingRelease(usize, usize),
    ReleaseReady(ReleaseMetadataGitHub),
    UpdateError(String),
}

impl From<io::Error> for State {
    fn from(value: io::Error) -> Self {
        Self::UpdateError(value.to_string())
    }
}

impl From<ureq::Error> for State {
    fn from(value: ureq::Error) -> Self {
        Self::UpdateError(value.to_string())
    }
}

impl From<String> for State {
    fn from(value: String) -> Self {
        Self::UpdateError(value)
    }
}

impl From<Box<dyn std::any::Any + std::marker::Send>> for State {
    fn from(value: Box<dyn std::any::Any + std::marker::Send>) -> Self {
        Self::UpdateError(format!("{:?}", value))
    }
}

#[derive(Debug, Default, Clone)]
pub struct UpdateState {
    pub when: Option<chrono::DateTime<chrono::Local>>,
    pub state: State,
    pub url_test_result: Option<Result<(), String>>,
    pub force_update: bool,
    pub stop_evt: bool,
}
