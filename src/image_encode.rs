use gif::{Encoder as GifEncoder, Frame as GifFrame, Repeat};

use crate::image::Frame;
use crate::image_error::ImageError;

pub fn encode_gif_animated(frames: &[Frame]) -> Result<Vec<u8>, ImageError> {
    if frames.is_empty() {
        return Err(ImageError("No frames to encode".into()));
    }

    let width = frames[0].buffer.width() as u16;
    let height = frames[0].buffer.height() as u16;

    let mut buf = Vec::new();
    {
        let mut encoder = GifEncoder::new(&mut buf, width, height, &[])
            .map_err(|e| ImageError(format!("Failed to create GIF encoder: {}", e)))?;
        encoder.set_repeat(Repeat::Infinite)
            .map_err(|e| ImageError(format!("Failed to set GIF repeat: {}", e)))?;

        for frame in frames {
            let mut rgba = frame.buffer.as_raw().clone();
            let mut gif_frame = GifFrame::from_rgba_speed(width, height, &mut rgba, 10);
            gif_frame.delay = (frame.delay_ms / 10) as u16; // convert ms to centiseconds
            encoder.write_frame(&gif_frame)
                .map_err(|e| ImageError(format!("Failed to write GIF frame: {}", e)))?;
        }
    }

    Ok(buf)
}

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
