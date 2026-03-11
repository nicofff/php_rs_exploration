use crate::image_error::ImageError;

pub fn encode_webp(rgba: &[u8], width: u32, height: u32, quality: u8) -> Result<Vec<u8>, ImageError> {
    let encoder = webp::Encoder::from_rgba(rgba, width, height);
    let memory = encoder.encode(quality as f32);
    Ok(memory.to_vec())
}

pub fn encode_webp_animated(frames: &[image::Frame], quality: u8) -> Result<Vec<u8>, ImageError> {
    if frames.is_empty() {
        return Err(ImageError("No frames to encode".into()));
    }

    let width = frames[0].buffer().width();
    let height = frames[0].buffer().height();

    let mut config = webp::WebPConfig::new()
        .map_err(|_| ImageError("Failed to create WebP config".into()))?;
    config.quality = quality as f32;

    let mut encoder = webp::AnimEncoder::new(width, height, &config);
    encoder.set_loop_count(0);

    let mut timestamp_ms: i32 = 0;
    for frame in frames {
        let ts = timestamp_ms;
        let (numer, denom) = frame.delay().numer_denom_ms();
        let delay_ms = if denom == 0 { 0 } else { numer / denom };
        timestamp_ms += delay_ms as i32;
        let anim_frame = webp::AnimFrame::from_rgba(frame.buffer().as_raw(), width, height, ts);
        encoder.add_frame(anim_frame);
    }

    let data = encoder.encode();
    Ok(data.to_vec())
}

pub fn encode_avif(rgba: &[u8], width: u32, height: u32, quality: u8) -> Result<Vec<u8>, ImageError> {
    use image::codecs::avif::AvifEncoder;
    use image::ImageEncoder;

    let mut buf = Vec::new();
    let encoder = AvifEncoder::new_with_speed_quality(&mut buf, 6, quality);
    encoder
        .write_image(rgba, width, height, image::ExtendedColorType::Rgba8)
        .map_err(|e| ImageError(format!("AVIF encoding failed: {}", e)))?;
    Ok(buf)
}
