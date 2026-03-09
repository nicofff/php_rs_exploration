use crate::image_error::ImageError;
use crate::image::Frame;

pub fn decode_static_from_path(path: &str) -> Result<Vec<Frame>, ImageError> {
    let img = image::open(path)
        .map_err(|e| ImageError(format!("Failed to decode image '{}': {}", path, e)))?;
    Ok(vec![Frame {
        buffer: img.to_rgba8(),
        delay_ms: 0,
    }])
}

pub fn decode_static_from_buffer(bytes: &[u8]) -> Result<Vec<Frame>, ImageError> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| ImageError(format!("Failed to decode image from buffer: {}", e)))?;
    Ok(vec![Frame {
        buffer: img.to_rgba8(),
        delay_ms: 0,
    }])
}
