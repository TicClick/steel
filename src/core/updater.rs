use std::collections::BTreeMap;
use std::io::{self, Read};
use std::sync::{Arc, Mutex};

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use steel_core::VersionString;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub const RECENT_RELEASES_METADATA_URL: &str =
    "https://api.github.com/repos/TicClick/steel/releases";
const GIST_METADATA_FILENAME: &str = "releases.json";

pub fn default_update_url() -> String {
    #[cfg(feature = "glass")]
    {
        glass::ROOT_RELEASES_URL.to_owned()
    }
    #[cfg(not(feature = "glass"))]
    {
        RECENT_RELEASES_METADATA_URL.to_owned()
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

#[derive(Debug)]
pub enum UpdateSource {
    Unknown,
    GitHub(String),
    Gist(String),
}

#[derive(Debug)]
pub enum BackendRequest {
    InitiateAutoupdate,
    SetAutoupdateStatus(bool),
    ChangeURL(String),
    FetchMetadata,
    FetchRelease,
    Quit,
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

    pub fn size(&self) -> usize {
        match self.platform_specific_asset() {
            Some(a) => a.size,
            None => 0,
        }
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

pub fn test_update_url(url: &str) -> Result<UpdateSource, String> {
    match ureq::request("GET", url).call() {
        Err(e) => Err(e.to_string()),
        Ok(payload) => {
            let v = payload.into_string().unwrap();
            match serde_json::from_str::<Vec<ReleaseMetadataGitHub>>(&v) {
                Ok(_) => Ok(UpdateSource::GitHub(url.to_owned())),
                Err(_) => match serde_json::from_str::<ReleaseMetadataGist>(&v) {
                    Ok(_) => Ok(UpdateSource::Gist(url.to_owned())),
                    Err(e) => Err(e.to_string()),
                },
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
        Self::new(UpdateSource::Unknown)
    }
}

impl Updater {
    pub fn new(src: UpdateSource) -> Self {
        let state = Arc::new(Mutex::new(UpdateState::default()));
        let (tx, rx) = channel(5);

        let state_ = state.clone();
        let backend_transmitter = tx.clone();
        let th = std::thread::spawn(move || {
            UpdaterBackend::new(src, state_, backend_transmitter, rx).run();
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

    pub fn enable_autoupdate(&self, enable: bool) {
        self.channel
            .blocking_send(BackendRequest::SetAutoupdateStatus(enable))
            .unwrap();
        if enable {
            self.channel
                .blocking_send(BackendRequest::InitiateAutoupdate)
                .unwrap();
        }
    }

    pub fn change_url(&self, url: &str) {
        self.channel
            .blocking_send(BackendRequest::ChangeURL(url.to_owned()))
            .unwrap();
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

    pub fn abort_update(&self) {
        self.state.lock().unwrap().stop_evt = true;
    }

    pub fn available_update(&self) -> Option<ReleaseMetadataGitHub> {
        match self.state().state {
            State::MetadataReady(r) => Some(r),
            _ => None,
        }
    }

    pub fn is_update_done(&self) -> bool {
        matches!(self.state().state, State::ReleaseReady(_))
    }
}

pub const AUTOUPDATE_INTERVAL_MINUTES: i64 = 10;

struct UpdaterBackend {
    src: UpdateSource,
    state: Arc<Mutex<UpdateState>>,
    self_channel: Sender<BackendRequest>,
    channel: Receiver<BackendRequest>,
    autoupdate: bool,
    last_autoupdate_run: Option<chrono::DateTime<chrono::Local>>,
    autoupdate_timer: Option<std::thread::JoinHandle<()>>,
}

impl UpdaterBackend {
    fn new(
        src: UpdateSource,
        state: Arc<Mutex<UpdateState>>,
        self_channel: Sender<BackendRequest>,
        channel: Receiver<BackendRequest>,
    ) -> Self {
        Self {
            src,
            state,
            self_channel,
            channel,
            autoupdate: false,
            last_autoupdate_run: None,
            autoupdate_timer: None,
        }
    }

    fn run(&mut self) {
        crate::core::os::cleanup_after_update();
        while let Some(msg) = self.channel.blocking_recv() {
            match msg {
                BackendRequest::Quit => break,
                BackendRequest::FetchMetadata => self.fetch_metadata(),
                BackendRequest::FetchRelease => self.fetch_release(),
                BackendRequest::InitiateAutoupdate => {
                    self.run_update_cycle();
                }
                BackendRequest::SetAutoupdateStatus(enabled) => {
                    self.autoupdate = enabled;
                }
                BackendRequest::ChangeURL(url) => self.change_url(url),
            }
        }
    }

    fn change_url(&mut self, url: String) {
        match &self.src {
            UpdateSource::Gist(s) | UpdateSource::GitHub(s) => {
                if s == &url {
                    return;
                }
            }
            UpdateSource::Unknown => (),
        }

        match test_update_url(&url) {
            Ok(src) => {
                log::debug!("updater: change url {:?} -> {:?}", self.src, src);
                self.src = src;
                let mut guard = self.state.lock().unwrap();
                guard.url_test_result = Some(Ok(()));
                guard.force_update = true;
            }
            Err(e) => {
                self.state.lock().unwrap().force_update = false;
                self.state.lock().unwrap().url_test_result = Some(Err(e));
            }
        }
        self.set_state(State::Idle);
    }

    fn run_update_cycle(&mut self) {
        let enough_time_slept = match self.last_autoupdate_run {
            None => true,
            Some(when) => {
                (chrono::Local::now() - when).num_minutes() >= AUTOUPDATE_INTERVAL_MINUTES
            }
        };
        if !(enough_time_slept && self.autoupdate) {
            return;
        }

        if let Some(th) = self.autoupdate_timer.take() {
            log::debug!("joining the previous autoupdate timer thread (should be really quick)..");
            if let Err(e) = th.join() {
                log::error!("previous autoupdate thread failed with error: {:?}", e);
            }
        }

        self.last_autoupdate_run = Some(chrono::Local::now());
        let tx = self.self_channel.clone();
        self.autoupdate_timer = Some(std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(
                60 * AUTOUPDATE_INTERVAL_MINUTES as u64,
            ));
            tx.blocking_send(BackendRequest::InitiateAutoupdate)
                .unwrap();
        }));

        if matches!(self.state.lock().unwrap().state, State::ReleaseReady(_)) {
            return;
        }

        self.fetch_metadata();
        let (state, force_update) = {
            let guard = self.state.lock().unwrap();
            (guard.state.clone(), guard.force_update)
        };
        if let State::MetadataReady(m) = state {
            if (force_update || crate::VERSION.semver() < m.tag_name.semver())
                && m.platform_specific_asset().is_some()
            {
                self.fetch_release();
            }
            self.state.lock().unwrap().force_update = false;
        }
    }

    fn set_state(&self, state: State) {
        if let State::UpdateError(ref text) = state {
            log::error!("failed to perform the update: {}", text);
        }
        let mut guard = self.state.lock().unwrap();
        *guard = UpdateState {
            state,
            stop_evt: guard.stop_evt,
            when: Some(chrono::Local::now()),
            url_test_result: guard.url_test_result.clone(),
            force_update: guard.force_update,
        };
    }

    fn fetch_metadata(&self) {
        log::debug!("updater: checking releases metadata");
        self.set_state(State::FetchingMetadata);
        match &self.src {
            UpdateSource::Unknown => {
                log::debug!("no update source set, idling");
                self.set_state(State::Idle);
            }
            UpdateSource::Gist(s) => self.fetch_metadata_gist(s),
            UpdateSource::GitHub(s) => self.fetch_metadata_github(s),
        }
    }

    // The metadata file should mirror GitHub API response from /repos/{user}/{repo}/releases
    fn fetch_metadata_gist(&self, url: &str) {
        match ureq::request("GET", url).call() {
            Ok(payload) => match payload.into_json::<ReleaseMetadataGist>() {
                Ok(files) => {
                    if let Some(metadata_file) = files.files.get(GIST_METADATA_FILENAME) {
                        self.fetch_metadata_github(&metadata_file.raw_url);
                    } else {
                        self.set_state(State::UpdateError(
                            "updater: failed to fetch preliminary metadata file from gist.github.com".into(),
                        ));
                    }
                }
                Err(e) => self.set_state(e.into()),
            },
            Err(e) => self.set_state(e.into()),
        }
    }

    fn fetch_metadata_github(&self, url: &str) {
        match ureq::request("GET", url).call() {
            Ok(payload) => match payload.into_json::<Vec<ReleaseMetadataGitHub>>() {
                Ok(mut releases) => {
                    // Descending order
                    releases
                        .sort_by(|a, b| a.tag_name.semver().cmp(&b.tag_name.semver()).reverse());
                    log::debug!("updater: latest release info -> {:?}", releases.first());
                    for release in releases {
                        if release.platform_specific_asset().is_some() {
                            log::debug!("latest relevant release: {:?}", release);
                            self.set_state(State::MetadataReady(release));
                            return;
                        }
                    }
                    self.set_state(State::UpdateError(
                        "no suitable release found for your platform".into(),
                    ));
                }
                Err(e) => self.set_state(e.into()),
            },
            Err(e) => self.set_state(e.into()),
        }
    }

    fn fetch_release(&self) {
        let state = self.state.lock().unwrap().clone().state;
        if let State::MetadataReady(m) = state {
            if let Some(a) = m.platform_specific_asset() {
                log::debug!(
                    "updater: fetching the new release from {}",
                    a.browser_download_url
                );
                self.set_state(State::FetchingRelease(0, 0));
                match ureq::request("GET", &a.browser_download_url).call() {
                    Ok(r) => {
                        let state = self.state.clone();
                        match std::thread::scope(|s| s.spawn(|| download(r, state)).join()) {
                            Ok(Ok(data)) => match self.prepare_release(data, a.archive_type()) {
                                Ok(()) => self.set_state(State::ReleaseReady(m)),
                                Err(e) => self.set_state(e.into()),
                            },
                            Ok(Err(e)) => self.set_state(e.into()),
                            Err(e) => self.set_state(e.into()),
                        }
                    }
                    Err(e) => self.set_state(e.into()),
                }
            }
        }
    }

    fn prepare_release(
        &self,
        data: Vec<u8>,
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
            "{}.old",
            target.file_name().unwrap().to_str().unwrap()
        ));
        log::info!(
            "updater: renaming old executable {:?} -> {:?}",
            target,
            backup
        );
        std::fs::rename(&target, &backup)?;

        let reader = Box::new(std::io::Cursor::new(data));
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

fn download(r: ureq::Response, state: Arc<Mutex<UpdateState>>) -> Result<Vec<u8>, std::io::Error> {
    let chunk_sz = 1 << 18; // 256K
    let total_bytes: usize = r.header("Content-Length").unwrap().parse().unwrap();
    state.lock().unwrap().state = State::FetchingRelease(0, total_bytes);
    let mut chunk = Vec::with_capacity(std::cmp::min(total_bytes, chunk_sz));
    let mut data = Vec::new();
    let mut stream = r.into_reader();
    loop {
        match stream.read_exact(&mut chunk) {
            Ok(_) => {
                let bytes_left = total_bytes - data.len() - chunk.len();
                data.append(&mut chunk);

                let mut state = state.lock().unwrap();
                if state.stop_evt {
                    log::info!("Update aborted by user");
                    state.stop_evt = false;
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        "Update aborted by user",
                    ));
                }
                state.state = State::FetchingRelease(data.len(), total_bytes);
                let next_chunk_sz = std::cmp::min(bytes_left, chunk_sz);
                if next_chunk_sz > 0 {
                    chunk.resize(next_chunk_sz, 0);
                } else {
                    return Ok(data);
                }
            }
            Err(e) => {
                log::error!("Failed to download the file in full: {}", e);
                return Err(e);
            }
        }
    }
}
