use crate::core::settings::{BuiltInSound, Sound};

const BELL: &[u8] = include_bytes!("../../media/sounds/bell.mp3");
const DOUBLE_BELL: &[u8] = include_bytes!("../../media/sounds/double-bell.mp3");
const PARTY_HORN: &[u8] = include_bytes!("../../media/sounds/party-horn.mp3");
const PING: &[u8] = include_bytes!("../../media/sounds/ping.mp3");
const TICK: &[u8] = include_bytes!("../../media/sounds/tick.mp3");
const TWO_TONE: &[u8] = include_bytes!("../../media/sounds/two-tone.mp3");

pub struct SoundPlayer {
    _stream: Option<rodio::OutputStream>,
    stream_handle: Option<rodio::OutputStreamHandle>,
    sink: Option<rodio::Sink>,
    initialization_error: Option<rodio::StreamError>,
}

impl std::fmt::Debug for SoundPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SoundPlayer(functional={})", self.functional())
    }
}

impl Default for SoundPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundPlayer {
    pub fn new() -> Self {
        match rodio::OutputStream::try_default() {
            Ok((_stream, stream_handle)) => Self {
                _stream: Some(_stream),
                stream_handle: Some(stream_handle),
                sink: None,
                initialization_error: None,
            },
            Err(e) => {
                log::error!(
                    "failed to read the default audio device -- no sounds will play: {:?}",
                    e
                );
                Self {
                    _stream: None,
                    stream_handle: None,
                    sink: None,
                    initialization_error: Some(e),
                }
            }
        }
    }

    pub fn functional(&self) -> bool {
        self._stream.is_some() && self.stream_handle.is_some()
    }

    pub fn initialization_error(&self) -> &Option<rodio::StreamError> {
        &self.initialization_error
    }

    pub fn play(&mut self, sound: &Sound) {
        if let Some(sh) = &self.stream_handle {
            let sample = match sound {
                Sound::BuiltIn(builtin) => match builtin {
                    BuiltInSound::Bell => BELL,
                    BuiltInSound::DoubleBell => DOUBLE_BELL,
                    BuiltInSound::PartyHorn => PARTY_HORN,
                    BuiltInSound::Ping => PING,
                    BuiltInSound::Tick => TICK,
                    BuiltInSound::TwoTone => TWO_TONE,
                },
            };
            match sh.play_once(std::io::Cursor::new(sample)) {
                Ok(sink) => self.sink = Some(sink),
                Err(e) => {
                    log::error!("failed to play {:?}: {:?}", sound, e);
                }
            }
        }
    }
}
