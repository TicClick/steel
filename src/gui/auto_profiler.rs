use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct AutoProfiler {
    sink_id: puffin::FrameSinkId,
    frames: Arc<Mutex<Vec<Arc<puffin::FrameData>>>>,
}

impl AutoProfiler {
    pub fn new() -> Self {
        let frames: Arc<Mutex<Vec<Arc<puffin::FrameData>>>> = Arc::new(Mutex::new(Vec::new()));
        let frames_clone = frames.clone();
        let sink_id = puffin::GlobalProfiler::lock().add_sink(Box::new(move |frame| {
            frames_clone.lock().unwrap().push(frame);
        }));
        Self { sink_id, frames }
    }

    pub fn save(&self, path: &Path) {
        let frames = self.frames.lock().unwrap();
        if frames.is_empty() {
            eprintln!("[puffin] No frames collected, skipping save.");
            return;
        }
        match std::fs::File::create(path) {
            Err(e) => eprintln!("[puffin] Failed to create {}: {e}", path.display()),
            Ok(file) => {
                let mut writer = std::io::BufWriter::new(file);
                if let Err(e) = std::io::Write::write_all(&mut writer, b"PUF0") {
                    eprintln!("[puffin] Failed to write header: {e}");
                    return;
                }
                let mut written = 0usize;
                for frame in frames.iter() {
                    if let Err(e) = frame.write_into(None, &mut writer) {
                        eprintln!("[puffin] Failed to write frame: {e}");
                        return;
                    }
                    written += 1;
                }
                eprintln!("[puffin] Saved {written} frames to {}", path.display());
            }
        }
    }
}

impl Drop for AutoProfiler {
    fn drop(&mut self) {
        puffin::GlobalProfiler::lock().remove_sink(self.sink_id);
    }
}
