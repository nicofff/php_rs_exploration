use image::{DynamicImage, imageops::FilterType};

use crate::image_error::ImageError;
use crate::image::OutputFormat;
use crate::image_ops::overlay_rgba;
use crate::image_ops_trait::ImageOps;

pub(crate) struct AnimatedImage(pub(crate) Vec<image::Frame>);

impl ImageOps for AnimatedImage {
    fn resize(&mut self, width: u32, height: u32, fit: &str) -> Result<(), ImageError> {
        self.0 = self.0.iter().map(|frame| {
            let img = DynamicImage::ImageRgba8(frame.buffer().clone());
            let resized = match fit {
                "fill"  => img.resize_exact(width, height, FilterType::Lanczos3),
                "cover" => img.resize_to_fill(width, height, FilterType::Lanczos3),
                _       => img.resize(width, height, FilterType::Lanczos3),
            };
            image::Frame::from_parts(resized.to_rgba8(), 0, 0, frame.delay())
        }).collect();
        Ok(())
    }

    fn thumbnail(&mut self, width: u32, height: u32) {
        self.0 = self.0.iter().map(|frame| {
            let img = DynamicImage::ImageRgba8(frame.buffer().clone());
            let resized = img.resize(width, height, FilterType::Triangle);
            image::Frame::from_parts(resized.to_rgba8(), 0, 0, frame.delay())
        }).collect();
    }

    fn crop(&mut self, x: u32, y: u32, width: u32, height: u32) -> Result<(), ImageError> {
        if let Some(frame) = self.0.first() {
            let (fw, fh) = frame.buffer().dimensions();
            if x + width > fw || y + height > fh {
                return Err(ImageError(format!(
                    "Crop region {}x{} at ({},{}) exceeds image bounds {}x{}",
                    width, height, x, y, fw, fh
                )));
            }
        }
        self.0 = self.0.iter().map(|frame| {
            let img = DynamicImage::ImageRgba8(frame.buffer().clone());
            let cropped = img.crop_imm(x, y, width, height);
            image::Frame::from_parts(cropped.to_rgba8(), 0, 0, frame.delay())
        }).collect();
        Ok(())
    }

    fn grayscale(&mut self) {
        self.0 = self.0.iter().map(|frame| {
            let img = DynamicImage::ImageRgba8(frame.buffer().clone());
            let gray = img.grayscale().to_rgba8();
            image::Frame::from_parts(gray, 0, 0, frame.delay())
        }).collect();
    }

    fn overlay(&mut self, overlay: &DynamicImage, x: i32, y: i32, opacity: f32) {
        self.0 = self.0.iter().map(|frame| {
            let mut rgba = frame.buffer().clone();
            overlay_rgba(&mut rgba, overlay, x, y, opacity);
            image::Frame::from_parts(rgba, frame.left(), frame.top(), frame.delay())
        }).collect();
    }

    fn dimensions(&self) -> (u32, u32) {
        self.0.first()
            .map(|f| f.buffer().dimensions())
            .unwrap_or((0, 0))
    }

    fn encode(&self, format: OutputFormat) -> Result<Vec<u8>, ImageError> {
        match format {
            OutputFormat::Jpeg(quality) => {
                let frame = self.0.first()
                    .ok_or_else(|| ImageError("No frames".into()))?;
                let mut buf = Vec::new();
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                    std::io::Cursor::new(&mut buf), quality,
                );
                frame.buffer().write_with_encoder(encoder)
                    .map_err(|e| ImageError(format!("JPEG encoding failed: {}", e)))?;
                Ok(buf)
            }
            OutputFormat::Png => {
                let frame = self.0.first()
                    .ok_or_else(|| ImageError("No frames".into()))?;
                let img = DynamicImage::ImageRgba8(frame.buffer().clone());
                let mut buf = Vec::new();
                img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                    .map_err(|e| ImageError(format!("PNG encoding failed: {}", e)))?;
                Ok(buf)
            }
            OutputFormat::Webp(quality) => {
                if self.0.len() > 1 {
                    crate::image_encode::encode_webp_animated(&self.0, quality)
                } else {
                    let frame = self.0.first()
                        .ok_or_else(|| ImageError("No frames".into()))?;
                    let buf = frame.buffer();
                    crate::image_encode::encode_webp(buf.as_raw(), buf.width(), buf.height(), quality)
                }
            }
            OutputFormat::Avif(quality) => {
                let frame = self.0.first()
                    .ok_or_else(|| ImageError("No frames".into()))?;
                let buf = frame.buffer();
                crate::image_encode::encode_avif(buf.as_raw(), buf.width(), buf.height(), quality)
            }
            OutputFormat::Gif => {
                use image::codecs::gif::{GifEncoder, Repeat};
                let mut buf = Vec::new();
                {
                    let mut encoder = GifEncoder::new(&mut buf);
                    encoder.set_repeat(Repeat::Infinite)
                        .map_err(|e| ImageError(format!("Failed to set GIF repeat: {}", e)))?;
                    encoder.encode_frames(self.0.iter().cloned())
                        .map_err(|e| ImageError(format!("GIF encoding failed: {}", e)))?;
                }
                Ok(buf)
            }
        }
    }

    fn first_frame(&self) -> DynamicImage {
        self.0.first()
            .map(|f| DynamicImage::ImageRgba8(f.buffer().clone()))
            .unwrap_or_else(|| DynamicImage::new_rgba8(1, 1))
    }
}
