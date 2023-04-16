pub mod actor;
pub mod app;
pub mod core;
pub mod gui;

const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

pub trait VersionString {
    fn semver(&self) -> (u8, u8, u8);
}

impl VersionString for &str {
    fn semver(&self) -> (u8, u8, u8) {
        if !self.starts_with('v') {
            return (0, 0, 0);
        }

        let mut parts: Vec<u8> = self[1..]
            .split('.')
            .filter_map(|i| i.parse::<u8>().ok())
            .collect();
        while parts.len() < 3 {
            parts.push(0);
        }
        (parts[0], parts[1], parts[2])
    }
}

impl VersionString for String {
    fn semver(&self) -> (u8, u8, u8) {
        self.as_str().semver()
    }
}

pub fn png_to_rgba(bytes: &[u8]) -> Result<(Vec<u8>, (u32, u32)), png::DecodingError> {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    match reader.next_frame(&mut buf) {
        Ok(_) => Ok((buf, reader.info().size())),
        Err(e) => Err(e),
    }
}
