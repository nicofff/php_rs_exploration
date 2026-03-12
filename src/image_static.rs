use image::{DynamicImage, GenericImageView};
use fast_image_resize::IntoImageView;

use crate::image_error::ImageError;
use crate::image::OutputFormat;
use crate::image_ops::overlay_rgba;
use crate::image_ops_trait::ImageOps;

pub(crate) struct StaticImage(pub(crate) DynamicImage);

/// Convert a fast_image_resize Image back to DynamicImage, preserving pixel format.
fn fir_to_dynamic(img: fast_image_resize::images::Image, w: u32, h: u32) -> DynamicImage {
    use fast_image_resize::PixelType;
    let pixel_type = img.pixel_type();
    let bytes = img.into_vec();
    match pixel_type {
        PixelType::U8x4 => DynamicImage::ImageRgba8(image::RgbaImage::from_raw(w, h, bytes).unwrap()),
        PixelType::U8x3 => DynamicImage::ImageRgb8(image::RgbImage::from_raw(w, h, bytes).unwrap()),
        PixelType::U8   => DynamicImage::ImageLuma8(image::GrayImage::from_raw(w, h, bytes).unwrap()),
        PixelType::U8x2 => DynamicImage::ImageLumaA8(image::GrayAlphaImage::from_raw(w, h, bytes).unwrap()),
        _ => DynamicImage::ImageRgba8(image::RgbaImage::from_raw(w, h, bytes).unwrap()),
    }
}

impl ImageOps for StaticImage {
    fn resize(&mut self, width: u32, height: u32, fit: &str) -> Result<(), ImageError> {
        use fast_image_resize as fr;
        use fr::images::Image;
        use fr::ResizeOptions;

        let pixel_type = self.0.pixel_type().unwrap();
        let (sw, sh) = (self.0.width(), self.0.height());

        self.0 = match fit {
            "fill" => {
                let mut dst = Image::new(width, height, pixel_type);
                fr::Resizer::new().resize(&self.0, &mut dst, None)
                    .map_err(|e| ImageError(format!("Resize failed: {}", e)))?;
                fir_to_dynamic(dst, width, height)
            }
            "cover" => {
                // Compute the source crop region that fills the target at uniform scale.
                let scale = f64::max(width as f64 / sw as f64, height as f64 / sh as f64);
                let crop_w = width as f64 / scale;
                let crop_h = height as f64 / scale;
                let crop_left = (sw as f64 - crop_w) / 2.0;
                let crop_top = (sh as f64 - crop_h) / 2.0;

                let mut dst = Image::new(width, height, pixel_type);
                fr::Resizer::new().resize(
                    &self.0, &mut dst,
                    &ResizeOptions::new().crop(crop_left, crop_top, crop_w, crop_h),
                ).map_err(|e| ImageError(format!("Resize failed: {}", e)))?;
                fir_to_dynamic(dst, width, height)
            }
            _ => { // "contain"
                let scale = f64::min(width as f64 / sw as f64, height as f64 / sh as f64);
                let nw = ((sw as f64 * scale).round() as u32).max(1);
                let nh = ((sh as f64 * scale).round() as u32).max(1);
                let mut dst = Image::new(nw, nh, pixel_type);
                fr::Resizer::new().resize(&self.0, &mut dst, None)
                    .map_err(|e| ImageError(format!("Resize failed: {}", e)))?;
                fir_to_dynamic(dst, nw, nh)
            }
        };
        Ok(())
    }

    fn thumbnail(&mut self, width: u32, height: u32) {
        use fast_image_resize as fr;
        use fr::images::Image;
        use fr::ResizeOptions;

        let (sw, sh) = (self.0.width(), self.0.height());
        let scale = f64::min(width as f64 / sw as f64, height as f64 / sh as f64);
        let nw = ((sw as f64 * scale).round() as u32).max(1);
        let nh = ((sh as f64 * scale).round() as u32).max(1);
        let mut dst = Image::new(nw, nh, self.0.pixel_type().unwrap());
        fr::Resizer::new()
            .resize(
                &self.0, &mut dst,
                &ResizeOptions::new().resize_alg(fr::ResizeAlg::Convolution(fr::FilterType::Bilinear)),
            )
            .expect("thumbnail succeeded");
        self.0 = fir_to_dynamic(dst, nw, nh);
    }

    fn crop(&mut self, x: u32, y: u32, width: u32, height: u32) -> Result<(), ImageError> {
        let (iw, ih) = self.0.dimensions();
        if x + width > iw || y + height > ih {
            return Err(ImageError(format!(
                "Crop region {}x{} at ({},{}) exceeds image bounds {}x{}",
                width, height, x, y, iw, ih
            )));
        }
        self.0 = self.0.crop_imm(x, y, width, height);
        Ok(())
    }

    fn grayscale(&mut self) {
        self.0 = self.0.grayscale();
    }

    fn overlay(&mut self, overlay: &DynamicImage, x: i32, y: i32, opacity: f32) {
        let mut rgba = self.0.to_rgba8();
        overlay_rgba(&mut rgba, overlay, x, y, opacity);
        self.0 = DynamicImage::ImageRgba8(rgba);
    }

    fn dimensions(&self) -> (u32, u32) {
        self.0.dimensions()
    }

    fn encode(&self, format: OutputFormat) -> Result<Vec<u8>, ImageError> {
        match format {
            OutputFormat::Jpeg(quality) => {
                let mut buf = Vec::new();
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                    std::io::Cursor::new(&mut buf), quality,
                );
                self.0.to_rgb8().write_with_encoder(encoder)
                    .map_err(|e| ImageError(format!("JPEG encoding failed: {}", e)))?;
                Ok(buf)
            }
            OutputFormat::Png => {
                let mut buf = Vec::new();
                self.0.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                    .map_err(|e| ImageError(format!("PNG encoding failed: {}", e)))?;
                Ok(buf)
            }
            OutputFormat::Webp(quality) => {
                let rgba = self.0.to_rgba8();
                crate::image_encode::encode_webp(rgba.as_raw(), rgba.width(), rgba.height(), quality)
            }
            OutputFormat::Avif(quality) => {
                let rgba = self.0.to_rgba8();
                crate::image_encode::encode_avif(rgba.as_raw(), rgba.width(), rgba.height(), quality)
            }
            OutputFormat::Gif => {
                use image::codecs::gif::{GifEncoder, Repeat};
                let mut buf = Vec::new();
                {
                    let mut encoder = GifEncoder::new(&mut buf);
                    encoder.set_repeat(Repeat::Infinite)
                        .map_err(|e| ImageError(format!("Failed to set GIF repeat: {}", e)))?;
                    encoder.encode_frames(std::iter::once(image::Frame::new(self.0.to_rgba8())))
                        .map_err(|e| ImageError(format!("GIF encoding failed: {}", e)))?;
                }
                Ok(buf)
            }
        }
    }

    fn first_frame(&self) -> DynamicImage {
        self.0.clone()
    }
}
