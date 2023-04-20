pub mod actor;
pub mod app;
pub mod core;
pub mod gui;

const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

pub fn png_to_rgba(bytes: &[u8]) -> Result<(Vec<u8>, (u32, u32)), png::DecodingError> {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    match reader.next_frame(&mut buf) {
        Ok(_) => Ok((buf, reader.info().size())),
        Err(e) => Err(e),
    }
}
