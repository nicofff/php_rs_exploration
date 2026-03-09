use fast_image_resize::{images::Image as FirImage, ResizeAlg, ResizeOptions, Resizer, PixelType};
use image::RgbaImage;

use crate::image_error::ImageError;
use crate::image::Frame;

pub fn resize_frame(frame: &Frame, new_width: u32, new_height: u32, algorithm: ResizeAlg) -> Result<Frame, ImageError> {
    if new_width == 0 || new_height == 0 {
        return Err(ImageError("Resize dimensions must be > 0".into()));
    }

    let src_image = FirImage::from_vec_u8(
        frame.buffer.width(),
        frame.buffer.height(),
        frame.buffer.as_raw().clone(),
        PixelType::U8x4,
    ).map_err(|e| ImageError(format!("Failed to create source image: {}", e)))?;

    let mut dst_image = FirImage::new(new_width, new_height, PixelType::U8x4);

    let mut resizer = Resizer::new();
    let options = ResizeOptions::new().resize_alg(algorithm);
    resizer.resize(&src_image, &mut dst_image, &options)
        .map_err(|e| ImageError(format!("Resize failed: {}", e)))?;

    let dst_buf = dst_image.into_vec();
    let rgba = RgbaImage::from_raw(new_width, new_height, dst_buf)
        .ok_or_else(|| ImageError("Failed to construct output image".into()))?;

    Ok(Frame {
        buffer: rgba,
        delay_ms: frame.delay_ms,
    })
}

pub fn fit_contain(src_w: u32, src_h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    let ratio_w = max_w as f64 / src_w as f64;
    let ratio_h = max_h as f64 / src_h as f64;
    let ratio = ratio_w.min(ratio_h);
    let new_w = (src_w as f64 * ratio).round() as u32;
    let new_h = (src_h as f64 * ratio).round() as u32;
    (new_w.max(1), new_h.max(1))
}

pub fn fit_cover(src_w: u32, src_h: u32, target_w: u32, target_h: u32) -> (u32, u32, u32, u32) {
    let ratio_w = target_w as f64 / src_w as f64;
    let ratio_h = target_h as f64 / src_h as f64;
    let ratio = ratio_w.max(ratio_h);
    let scaled_w = (src_w as f64 * ratio).round() as u32;
    let scaled_h = (src_h as f64 * ratio).round() as u32;
    let crop_x = (scaled_w - target_w) / 2;
    let crop_y = (scaled_h - target_h) / 2;
    (scaled_w, scaled_h, crop_x, crop_y)
}

pub fn crop_frame(frame: &Frame, x: u32, y: u32, width: u32, height: u32) -> Result<Frame, ImageError> {
    let src_w = frame.buffer.width();
    let src_h = frame.buffer.height();

    if x + width > src_w || y + height > src_h {
        return Err(ImageError(format!(
            "Crop region {}x{} at ({},{}) exceeds image bounds {}x{}",
            width, height, x, y, src_w, src_h
        )));
    }

    let cropped = image::imageops::crop_imm(&frame.buffer, x, y, width, height).to_image();
    Ok(Frame {
        buffer: cropped,
        delay_ms: frame.delay_ms,
    })
}
