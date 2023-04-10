use std::io::{self, Read};
use std::sync::{Arc, Mutex};

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{channel, Receiver, Sender};

#[derive(Debug, Default, Clone)]
pub enum UpdateState {
    #[default]
    Idle,
    FetchingMetadata,
    MetadataReady(ReleaseMetadata),
    FetchingRelease,
    ReleaseReady(ReleaseMetadata),
    UpdateError(String),
}

impl From<io::Error> for UpdateState {
    fn from(value: io::Error) -> Self {
        Self::UpdateError(value.to_string())
    }
}

impl From<ureq::Error> for UpdateState {
    fn from(value: ureq::Error) -> Self {
        Self::UpdateError(value.to_string())
    }
}

impl From<String> for UpdateState {
    fn from(value: String) -> Self {
        Self::UpdateError(value)
    }
}

#[derive(Debug)]
pub enum BackendRequest {
    FetchMetadata,
    FetchRelease,
    Quit,
}

const RELEASE_METADATA_URL: &str = "https://api.github.com/repos/TicClick/steel/releases/latest";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReleaseMetadata {
    pub tag_name: String,
    pub published_at: chrono::DateTime<chrono::Utc>,
    pub assets: Vec<ReleaseAsset>,
}

impl ReleaseMetadata {
    pub fn platform_specific_asset(&self) -> Option<&ReleaseAsset> {
        let marker = if cfg!(windows) {
            "-windows"
        } else if cfg!(macos) {
            "-darwin"
        } else if cfg!(unix) {
            "-linux"
        } else {
            ""
        };

        if marker.is_empty() {
            return None;
        }
        return self.assets.iter().find(|a| a.name.contains(marker));
    }
}

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
    pub size: u32,
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

#[derive(Debug)]
pub struct Updater {
    state: Arc<Mutex<UpdateState>>,
    th: std::thread::JoinHandle<()>,
    channel: Sender<BackendRequest>,
}

impl Default for Updater {
    fn default() -> Self {
        Self::new()
    }
}

impl Updater {
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(UpdateState::default()));
        let (tx, rx) = channel(5);

        let state_ = state.clone();
        let th = std::thread::spawn(move || {
            UpdaterBackend::new(state_, rx).run();
        });
        Self {
            state,
            th,
            channel: tx,
        }
    }

    pub fn state(&self) -> UpdateState {
        (*self.state.lock().unwrap()).clone()
    }

    pub fn check_version(&self) {
        self.channel
            .blocking_send(BackendRequest::FetchMetadata)
            .unwrap();
    }

    pub fn download_new_version(&self) {
        self.channel
            .blocking_send(BackendRequest::FetchRelease)
            .unwrap();
    }

    pub fn stop(self) {
        self.channel.blocking_send(BackendRequest::Quit).unwrap();
        self.th.join().unwrap();
    }

    pub fn available_update(&self) -> Option<ReleaseMetadata> {
        match self.state() {
            UpdateState::MetadataReady(r) => Some(r),
            _ => None,
        }
    }

    pub fn is_update_done(&self) -> bool {
        matches!(self.state(), UpdateState::ReleaseReady(_))
    }
}

struct UpdaterBackend {
    state: Arc<Mutex<UpdateState>>,
    channel: Receiver<BackendRequest>,
}

impl UpdaterBackend {
    fn new(state: Arc<Mutex<UpdateState>>, channel: Receiver<BackendRequest>) -> Self {
        Self { state, channel }
    }

    fn cleanup_after_last_update(&self) {
        if let Ok(executable) = std::env::current_exe() {
            let mut old_backup = executable.clone();
            old_backup.set_file_name(format!(
                "{}.bak",
                executable.file_name().unwrap().to_str().unwrap()
            ));
            if !old_backup.exists() {
                return;
            }
            if let Err(e) = std::fs::remove_file(&old_backup) {
                log::warn!(
                    "failed to remove old executable ({:?}) which was left after SUCCESSFUL update: {:?}",
                    old_backup,
                    e
                );
            } else {
                log::debug!(
                    "removed old executable ({:?}) which was left after SUCCESSFUL update",
                    old_backup
                );
            }
        }
    }

    fn run(&mut self) {
        self.cleanup_after_last_update();
        while let Some(msg) = self.channel.blocking_recv() {
            match msg {
                BackendRequest::Quit => break,
                BackendRequest::FetchMetadata => self.fetch_metadata(),
                BackendRequest::FetchRelease => self.fetch_release(),
            }
        }
    }

    fn set_state(&self, state: UpdateState) {
        if let UpdateState::UpdateError(ref text) = state {
            log::error!("failed to perform the update: {}", text);
        }
        *self.state.lock().unwrap() = state;
    }

    fn fetch_metadata(&self) {
        log::info!("updater: checking {}", RELEASE_METADATA_URL);
        *self.state.lock().unwrap() = UpdateState::FetchingMetadata;
        match ureq::request("GET", RELEASE_METADATA_URL).call() {
            Ok(payload) => match payload.into_json() {
                Ok(p) => {
                    log::info!("updater: latest release info -> {:?}", p);
                    self.set_state(UpdateState::MetadataReady(p))
                }
                Err(e) => self.set_state(e.into()),
            },
            Err(e) => self.set_state(e.into()),
        }
    }

    fn fetch_release(&self) {
        let state = self.state.lock().unwrap().clone();
        if let UpdateState::MetadataReady(m) = state {
            match m.platform_specific_asset() {
                None => self.set_state(
                    "There's no package for your platform in the latest release"
                        .to_owned()
                        .into(),
                ),
                Some(a) => {
                    log::info!(
                        "updater: fetching the new release from {}",
                        a.browser_download_url
                    );
                    self.set_state(UpdateState::FetchingRelease);
                    match ureq::request("GET", &a.browser_download_url).call() {
                        Ok(p) => match self.prepare_release(p.into_reader(), a.archive_type()) {
                            Ok(()) => self.set_state(UpdateState::ReleaseReady(m)),
                            Err(e) => self.set_state(e.into()),
                        },
                        Err(e) => self.set_state(e.into()),
                    }
                }
            }
        }
    }

    fn prepare_release(
        &self,
        reader: Box<dyn std::io::Read + Send + Sync + 'static>,
        archive_type: ArchiveType,
    ) -> Result<(), std::io::Error> {
        log::info!("updater: archive type determined as {:?}", archive_type);
        if let ArchiveType::Unknown(ext) = archive_type {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "unknown archive extension {ext} -- you might be able to deal with it yourself"
                ),
            ));
        }

        let target = std::env::current_exe()?;
        let mut backup = target.clone();
        backup.set_file_name(format!(
            "{}.bak",
            target.file_name().unwrap().to_str().unwrap()
        ));
        log::info!(
            "updater: renaming old executable {:?} -> {:?}",
            target,
            backup
        );
        std::fs::rename(&target, &backup)?;

        let extraction_result = match archive_type {
            ArchiveType::TarGZip => self.extract_gzip(reader, &target),
            ArchiveType::Zip => self.extract_zip(reader, &target),
            ArchiveType::Unknown(_) => Ok(()), // handled above already
        };
        if extraction_result.is_err() {
            log::error!("{:?}", extraction_result);
            if let Err(e) = std::fs::rename(&backup, &target) {
                log::error!(
                    "failed to restore the executable after an unsuccessful update: {:?}",
                    e
                );
            };
        }
        extraction_result
    }

    fn extract_gzip(
        &self,
        reader: Box<dyn std::io::Read + Send + Sync + 'static>,
        target: &std::path::Path,
    ) -> Result<(), io::Error> {
        let gz_decoder = GzDecoder::new(reader);
        let mut archive = tar::Archive::new(gz_decoder);
        archive.unpack(target.parent().unwrap())
    }

    fn extract_zip(
        &self,
        mut reader: Box<dyn std::io::Read + Send + Sync + 'static>,
        target: &std::path::PathBuf,
    ) -> Result<(), io::Error> {
        // GitHub releases have a single file inside, the executable itself.
        match zip::read::read_zipfile_from_stream(&mut reader) {
            Ok(Some(mut file)) => {
                let mut buf = Vec::new();
                match file.read_to_end(&mut buf) {
                    Err(e) => Err(e),
                    Ok(_) => std::fs::write(target, buf),
                }
            }
            Ok(None) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "empty zip archive".to_string(),
            )),
            Err(e) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("failed to decode zip stream: {:?}", e),
            )),
        }
    }
}
