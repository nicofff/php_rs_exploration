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

pub fn grayscale_frame(frame: &Frame) -> Frame {
    let gray = image::DynamicImage::ImageRgba8(frame.buffer.clone()).grayscale().to_rgba8();
    Frame {
        buffer: gray,
        delay_ms: frame.delay_ms,
    }
}

pub fn overlay_frame(base: &Frame, overlay: &Frame, x: i32, y: i32, opacity: f32) -> Frame {
    let mut result = base.buffer.clone();
    let ow = overlay.buffer.width() as i32;
    let oh = overlay.buffer.height() as i32;
    let bw = base.buffer.width() as i32;
    let bh = base.buffer.height() as i32;

    for oy in 0..oh {
        for ox in 0..ow {
            let bx = x + ox;
            let by = y + oy;
            if bx >= 0 && bx < bw && by >= 0 && by < bh {
                let src = overlay.buffer.get_pixel(ox as u32, oy as u32);
                let dst = result.get_pixel(bx as u32, by as u32);

                let src_a = (src.0[3] as f32 / 255.0) * opacity;
                let dst_a = dst.0[3] as f32 / 255.0;
                let out_a = src_a + dst_a * (1.0 - src_a);

                let blend = |s: u8, d: u8| -> u8 {
                    if out_a == 0.0 { return 0; }
                    ((s as f32 * src_a + d as f32 * dst_a * (1.0 - src_a)) / out_a) as u8
                };

                result.put_pixel(bx as u32, by as u32, image::Rgba([
                    blend(src.0[0], dst.0[0]),
                    blend(src.0[1], dst.0[1]),
                    blend(src.0[2], dst.0[2]),
                    (out_a * 255.0) as u8,
                ]));
            }
        }
    }

    Frame {
        buffer: result,
        delay_ms: base.delay_ms,
    }
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
