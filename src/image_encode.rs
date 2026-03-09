use crate::image::Frame;
use crate::image_error::ImageError;

pub fn encode_webp(frame: &Frame, quality: u8) -> Result<Vec<u8>, ImageError> {
    let encoder = webp::Encoder::from_rgba(
        frame.buffer.as_raw(),
        frame.buffer.width(),
        frame.buffer.height(),
    );
    let memory = encoder.encode(quality as f32);
    Ok(memory.to_vec())
}
