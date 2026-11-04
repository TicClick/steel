use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub fn app_bundle_path() -> Option<&'static Path> {
    static BUNDLE: OnceLock<Option<PathBuf>> = OnceLock::new();
    BUNDLE
        .get_or_init(|| {
            if !cfg!(target_os = "macos") {
                return None;
            }
            let exe = std::env::current_exe().ok()?.canonicalize().ok()?;
            find_app_bundle(&exe)
        })
        .as_deref()
}

pub const APP_ARCHIVE_SUFFIX: &str = ".app.tar.gz";

pub fn app_archive_name() -> String {
    format!("steel-macos-{}{APP_ARCHIVE_SUFFIX}", std::env::consts::ARCH)
}

fn find_app_bundle(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;
    let is_bundle = macos.file_name()? == "MacOS"
        && contents.file_name()? == "Contents"
        && bundle.extension()? == "app";
    is_bundle.then(|| bundle.to_path_buf())
}

#[derive(Debug)]
pub enum ArchiveType {
    Zip,
    TarGZip,
    AppBundle,
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
        if self.name.ends_with(APP_ARCHIVE_SUFFIX) {
            return ArchiveType::AppBundle;
        }
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
        if cfg!(target_os = "macos") {
            let name = app_archive_name();
            if let Some(asset) = self.assets.iter().find(|a| a.name == name) {
                return Some(asset);
            }
            if app_bundle_path().is_some() {
                return None;
            }
        }

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

        let mut preferred_architectures = Vec::new();
        if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
            preferred_architectures.push("aarch64-");
        }
        if cfg!(target_arch = "x86_64") {
            preferred_architectures.push("x86_64-");
        }

        log::debug!(
            "platform-specific asset: looking for os={os_marker:?}, arch={preferred_architectures:?}"
        );

        let compatible: Vec<&ReleaseAsset> = self
            .assets
            .iter()
            .filter(|a| a.name.contains(os_marker))
            .collect();
        if !compatible.is_empty() {
            for arch in preferred_architectures {
                for asset in &compatible {
                    if asset.name.contains(arch) {
                        return Some(asset);
                    }
                }
            }
        }
        None
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
    FetchingRelease(usize, Option<usize>),
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
        Self::UpdateError(format!("{value:?}"))
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
