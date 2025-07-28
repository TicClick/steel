use std::io::{self, Read};
use std::sync::{Arc, Mutex};

use flate2::read::GzDecoder;
use steel_core::ipc::server::AppMessageIn;
use steel_core::ipc::updater::*;
use steel_core::settings::application::AutoUpdate;
use steel_core::VersionString;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

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

#[derive(Debug)]
pub enum UpdateSource {
    GitHub(String),
    Gist(String),
}

impl From<String> for UpdateSource {
    fn from(value: String) -> Self {
        if value.starts_with("https://api.github.com/repos/") {
            Self::GitHub(value)
        } else {
            Self::Gist(value)
        }
    }
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

fn set_state(
    state: &Arc<Mutex<UpdateState>>,
    current_step: State,
    core: &UnboundedSender<AppMessageIn>,
) {
    if let State::UpdateError(ref text) = current_step {
        log::error!("failed to perform the update: {}", text);
    }

    let new_state = {
        let mut guard = state.lock().unwrap();
        let new_state = UpdateState {
            state: current_step,
            stop_evt: guard.stop_evt,
            when: Some(chrono::Local::now()),
            url_test_result: guard.url_test_result.clone(),
            force_update: guard.force_update,
        };
        *guard = new_state.clone();
        new_state
    };

    core.send(AppMessageIn::UpdateStateChanged(new_state))
        .unwrap();
}

#[derive(Debug)]
pub struct Updater {
    state: Arc<Mutex<UpdateState>>,
    pub settings: AutoUpdate,
    th: std::thread::JoinHandle<()>,
    backend_channel: UnboundedSender<BackendRequest>,
}

impl Updater {
    pub fn new(core: UnboundedSender<AppMessageIn>, settings: AutoUpdate) -> Self {
        let state = Arc::new(Mutex::new(UpdateState::default()));
        let (tx, rx) = unbounded_channel();

        let state_ = state.clone();
        let backend_transmitter = tx.clone();

        let s = settings.clone();
        let th = std::thread::spawn(move || {
            UpdaterBackend::new(s, state_, backend_transmitter, rx, core).run();
        });
        Self {
            state,
            settings,
            th,
            backend_channel: tx,
        }
    }

    pub fn state(&self) -> UpdateState {
        (*self.state.lock().unwrap()).clone()
    }

    pub fn change_settings(&mut self, new_settings: AutoUpdate) {
        if new_settings.url != self.settings.url {
            self.change_url(&new_settings.url);
        }
        if new_settings.enabled != self.settings.enabled {
            self.toggle_autoupdate(new_settings.enabled);
        }
        self.settings = new_settings;
    }

    pub fn force_check_after_url_change(&self) {
        self.backend_channel
            .send(BackendRequest::InitiateAutoupdate)
            .unwrap();
    }

    pub fn toggle_autoupdate(&self, enabled: bool) {
        self.backend_channel
            .send(BackendRequest::SetAutoupdateStatus(enabled))
            .unwrap();
        if enabled {
            self.backend_channel
                .send(BackendRequest::InitiateAutoupdate)
                .unwrap();
        }
    }

    pub fn change_url(&self, url: &str) {
        self.backend_channel
            .send(BackendRequest::ChangeURL(url.to_owned()))
            .unwrap();
    }

    pub fn check_version(&self) {
        self.backend_channel
            .send(BackendRequest::FetchMetadata)
            .unwrap();
    }

    pub fn download_new_version(&self) {
        self.backend_channel
            .send(BackendRequest::FetchRelease)
            .unwrap();
    }

    pub fn stop(self) {
        self.backend_channel.send(BackendRequest::Quit).unwrap();
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
    self_channel: UnboundedSender<BackendRequest>,
    channel: UnboundedReceiver<BackendRequest>,
    core: UnboundedSender<AppMessageIn>,
    autoupdate: bool,
    last_autoupdate_run: Option<chrono::DateTime<chrono::Local>>,
    autoupdate_timer: Option<std::thread::JoinHandle<()>>,
}

impl UpdaterBackend {
    fn new(
        settings: AutoUpdate,
        state: Arc<Mutex<UpdateState>>,
        self_channel: UnboundedSender<BackendRequest>,
        channel: UnboundedReceiver<BackendRequest>,
        core: UnboundedSender<AppMessageIn>,
    ) -> Self {
        let src: UpdateSource = settings.url.clone().into();
        let should_force_update = settings.url != default_update_url();

        if should_force_update {
            let mut guard = state.lock().unwrap();
            guard.force_update = true;
            guard.url_test_result = match test_update_url(&settings.url) {
                Ok(_) => Some(Ok(())),
                Err(e) => Some(Err(e)),
            };
        }

        Self {
            src,
            state,
            self_channel,
            channel,
            core,
            autoupdate: settings.enabled,
            last_autoupdate_run: None,
            autoupdate_timer: None,
        }
    }

    fn run(&mut self) {
        crate::core::os::cleanup_after_update();
        if self.autoupdate {
            self.self_channel
                .send(BackendRequest::InitiateAutoupdate)
                .unwrap();
        }
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
        let url_unchanged = match &self.src {
            UpdateSource::Gist(s) | UpdateSource::GitHub(s) => s == &url,
        };

        if url_unchanged {
            {
                let mut guard = self.state.lock().unwrap();
                guard.url_test_result = Some(Ok(()));
            }
            set_state(&self.state, State::Idle, &self.core);
            return;
        }

        match test_update_url(&url) {
            Ok(src) => {
                log::debug!("updater: change url {:?} -> {:?}", self.src, src);
                self.src = src;
                {
                    let mut guard = self.state.lock().unwrap();
                    guard.url_test_result = Some(Ok(()));
                    guard.force_update = true;
                }
                set_state(&self.state, State::Idle, &self.core);
                // Automatically trigger update check after successful URL change
                self.run_update_cycle();
            }
            Err(e) => {
                {
                    let mut guard = self.state.lock().unwrap();
                    guard.force_update = false;
                    guard.url_test_result = Some(Err(e));
                }
                set_state(&self.state, State::Idle, &self.core);
            }
        }
    }

    fn run_update_cycle(&mut self) {
        // Always attempt metadata fetch if force_update is set (e.g., after URL change)
        let force_update = self.state.lock().unwrap().force_update;

        let enough_time_slept = match self.last_autoupdate_run {
            None => true,
            Some(when) => {
                (chrono::Local::now() - when).num_minutes() >= AUTOUPDATE_INTERVAL_MINUTES
            }
        };

        if !(force_update || (enough_time_slept && self.autoupdate)) {
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
            tx.send(BackendRequest::InitiateAutoupdate).unwrap();
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

    fn fetch_metadata(&self) {
        log::debug!("updater: checking releases metadata");
        set_state(&self.state, State::FetchingMetadata, &self.core);
        match &self.src {
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
                        set_state(
                            &self.state,
                            State::UpdateError("updater: failed to fetch preliminary metadata file from gist.github.com".into()),
                            &self.core
                        );
                    }
                }
                Err(e) => set_state(&self.state, e.into(), &self.core),
            },
            Err(e) => set_state(&self.state, e.into(), &self.core),
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
                            self.state.lock().unwrap().url_test_result = Some(Ok(()));
                            set_state(&self.state, State::MetadataReady(release), &self.core);
                            return;
                        }
                    }
                    set_state(
                        &self.state,
                        State::UpdateError("no suitable release found for your platform".into()),
                        &self.core,
                    );
                }
                Err(e) => set_state(&self.state, e.into(), &self.core),
            },
            Err(e) => set_state(&self.state, e.into(), &self.core),
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
                set_state(&self.state, State::FetchingRelease(0, None), &self.core);

                match ureq::request("GET", &a.browser_download_url).call() {
                    Ok(r) => {
                        let state = self.state.clone();
                        let core = self.core.clone();
                        match std::thread::scope(|s| s.spawn(|| download(r, state, core)).join()) {
                            Ok(Ok(data)) => match self.prepare_release(data, a.archive_type()) {
                                Ok(()) => {
                                    set_state(&self.state, State::ReleaseReady(m), &self.core)
                                }
                                Err(e) => set_state(&self.state, e.into(), &self.core),
                            },
                            Ok(Err(e)) => set_state(&self.state, e.into(), &self.core),
                            Err(e) => set_state(&self.state, e.into(), &self.core),
                        }
                    }
                    Err(e) => set_state(&self.state, e.into(), &self.core),
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

        // The original executable has already been renamed during the previous update round, and now we have fetched
        // a binary from another release stream.
        if target.exists() {
            std::fs::rename(&target, &backup)?;
        }

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

fn download(
    r: ureq::Response,
    state: Arc<Mutex<UpdateState>>,
    core: UnboundedSender<AppMessageIn>,
) -> Result<Vec<u8>, std::io::Error> {
    let chunk_sz = 1 << 18; // 256K
    let total_bytes: Option<usize> = r
        .header("Content-Length")
        .and_then(|value| value.parse().ok());

    let initial_state = State::FetchingRelease(0, total_bytes);
    set_state(&state, initial_state, &core);

    let mut chunk = Vec::with_capacity(chunk_sz);
    let mut data = Vec::new();
    let mut stream = r.into_reader();

    loop {
        match stream.read_exact(&mut chunk) {
            Ok(_) => {
                let bytes_left = match total_bytes {
                    Some(total) => total - data.len() - chunk.len(),
                    None => chunk_sz,
                };
                data.append(&mut chunk);

                // Change and send the state manually to avoid locking the mutex twice in a row unnecessarily.
                let new_state = {
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
                    state.clone()
                };
                core.send(AppMessageIn::UpdateStateChanged(new_state))
                    .unwrap();

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
