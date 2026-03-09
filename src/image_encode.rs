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

pub fn encode_avif(frame: &Frame, quality: u8) -> Result<Vec<u8>, ImageError> {
    use image::codecs::avif::AvifEncoder;
    use image::ImageEncoder;

    let (w, h) = frame.buffer.dimensions();
    let mut buf = Vec::new();
    let encoder = AvifEncoder::new_with_speed_quality(&mut buf, 6, quality);
    encoder
        .write_image(
            frame.buffer.as_raw(),
            w,
            h,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| ImageError(format!("AVIF encoding failed: {}", e)))?;
    Ok(buf)
}
