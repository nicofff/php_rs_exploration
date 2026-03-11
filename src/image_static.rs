use image::{DynamicImage, GenericImageView, imageops::FilterType};

use crate::image_error::ImageError;
use crate::image::OutputFormat;
use crate::image_ops::overlay_rgba;
use crate::image_ops_trait::ImageOps;

pub(crate) struct StaticImage(pub(crate) DynamicImage);

impl ImageOps for StaticImage {
    fn resize(&mut self, width: u32, height: u32, fit: &str) -> Result<(), ImageError> {
        self.0 = match fit {
            "fill"  => self.0.resize_exact(width, height, FilterType::Lanczos3),
            "cover" => self.0.resize_to_fill(width, height, FilterType::Lanczos3),
            _       => self.0.resize(width, height, FilterType::Lanczos3),
        };
        Ok(())
    }

    fn thumbnail(&mut self, width: u32, height: u32) {
        self.0 = self.0.resize(width, height, FilterType::Triangle);
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
