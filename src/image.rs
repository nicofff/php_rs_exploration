use std::collections::HashMap;
use ext_php_rs::prelude::*;
use image::imageops::FilterType;

use crate::image_decode;
use crate::image_error::ImageError;
use crate::image_info::ImageInfo;
use crate::image_ops;

fn read_exif(path: &str) -> Option<HashMap<String, String>> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;

    let mut map = HashMap::new();
    for field in exif.fields() {
        let tag = format!("{}", field.tag);
        let value = field.display_value().to_string();
        map.insert(tag, value);
    }
    Some(map)
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum OutputFormat {
    Jpeg(u8),
    Png,
    Gif,
    Webp(u8),
    Avif(u8),
}

#[php_class]
#[php(name = "RustImage\\Image")]
pub struct PhpImage {
    pub(crate) frames: Vec<image::Frame>,
    pub(crate) output_format: Option<OutputFormat>,
}

#[php_impl]
impl PhpImage {
    #[php(defaults(max_width = None, max_height = None, max_bytes = None))]
    pub fn open(
        path: String,
        max_width: Option<i64>,
        max_height: Option<i64>,
        max_bytes: Option<i64>,
    ) -> Result<Self, ImageError> {
        if let Some(limit) = max_bytes {
            let metadata = std::fs::metadata(&path)
                .map_err(|e| ImageError(format!("Failed to read file metadata '{}': {}", path, e)))?;
            let file_size = metadata.len() as i64;
            if file_size > limit {
                return Err(ImageError(format!(
                    "File size {} bytes exceeds limit of {} bytes",
                    file_size, limit
                )));
            }
        }

        // Pre-decode dimension check (reads header only)
        if max_width.is_some() || max_height.is_some() {
            if let Ok(reader) = image::ImageReader::open(&path) {
                if let Ok(reader) = reader.with_guessed_format() {
                    if let Ok((w, h)) = reader.into_dimensions() {
                        let mw = max_width.map(|v| v as u32).unwrap_or(u32::MAX);
                        let mh = max_height.map(|v| v as u32).unwrap_or(u32::MAX);
                        if w > mw || h > mh {
                            return Err(ImageError(format!(
                                "Image dimensions {}x{} exceed limit {}x{}", w, h, mw, mh
                            )));
                        }
                    }
                }
            }
        }

        let frames = if image_decode::is_gif(&path) {
            use image::codecs::gif::GifDecoder;
            use image::AnimationDecoder;
            use std::io::BufReader;
            use std::fs::File;

            let file = File::open(&path)
                .map_err(|e| ImageError(format!("Failed to open GIF '{}': {}", path, e)))?;
            let reader = BufReader::new(file);
            let decoder = GifDecoder::new(reader)
                .map_err(|e| ImageError(format!("Failed to decode GIF '{}': {}", path, e)))?;
            decoder.into_frames().collect_frames()
                .map_err(|e| ImageError(format!("Failed to read GIF frames '{}': {}", path, e)))?
        } else {
            let img = image::ImageReader::open(&path)
                .map_err(|e| ImageError(format!("Failed to open image '{}': {}", path, e)))?
                .with_guessed_format()
                .map_err(|e| ImageError(format!("Failed to guess format for '{}': {}", path, e)))?
                .decode()
                .map_err(|e| ImageError(format!("Failed to decode image '{}': {}", path, e)))?;
            vec![image::Frame::new(img.to_rgba8())]
        };

        // Check first frame dimensions against limits if provided
        if let Some(frame) = frames.first() {
            let (w, h) = frame.buffer().dimensions();
            if let Some(max_w) = max_width {
                if w as i64 > max_w {
                    return Err(ImageError(format!(
                        "Image width {} exceeds limit of {}",
                        w, max_w
                    )));
                }
            }
            if let Some(max_h) = max_height {
                if h as i64 > max_h {
                    return Err(ImageError(format!(
                        "Image height {} exceeds limit of {}",
                        h, max_h
                    )));
                }
            }
        }

        Ok(Self { frames, output_format: None })
    }

    pub fn from_buffer(bytes: ext_php_rs::binary::Binary<u8>) -> Result<Self, ImageError> {
        let img = image::load_from_memory(bytes.as_ref())
            .map_err(|e| ImageError(format!("Failed to decode image from buffer: {}", e)))?;
        let frames = vec![image::Frame::new(img.to_rgba8())];
        Ok(Self { frames, output_format: None })
    }

    /// Returns a copy of this image with independent pixel data.
    /// Useful for generating multiple thumbnails from a single decode.
    pub fn copy(&self) -> Self {
        Self {
            frames: self.frames.clone(),
            output_format: self.output_format,
        }
    }

    pub fn info(path: String) -> Result<ImageInfo, ImageError> {
        let reader = image::ImageReader::open(&path)
            .map_err(|e| ImageError(format!("Failed to open '{}': {}", path, e)))?
            .with_guessed_format()
            .map_err(|e| ImageError(format!("Failed to guess format: {}", e)))?;
        let format = reader.format()
            .map(|f| format!("{:?}", f).to_lowercase())
            .unwrap_or_else(|| "unknown".to_string());
        let (width, height) = reader.into_dimensions()
            .map_err(|e| ImageError(format!("Failed to read dimensions: {}", e)))?;

        let is_animated = if image_decode::is_gif(&path) {
            use image::codecs::gif::GifDecoder;
            use image::AnimationDecoder;
            use std::io::BufReader;
            use std::fs::File;

            if let Ok(file) = File::open(&path) {
                let reader = BufReader::new(file);
                if let Ok(decoder) = GifDecoder::new(reader) {
                    if let Ok(frames) = decoder.into_frames().collect_frames() {
                        frames.len() > 1
                    } else { false }
                } else { false }
            } else { false }
        } else {
            false
        };

        let exif_data = read_exif(&path);

        Ok(ImageInfo {
            width,
            height,
            format,
            has_alpha: false,
            is_animated,
            exif_data,
        })
    }

    #[php(defaults(fit = None))]
    pub fn resize(&mut self, width: i64, height: i64, fit: Option<String>) -> Result<(), ImageError> {
        if width <= 0 || height <= 0 {
            return Err(ImageError("Resize dimensions must be positive".into()));
        }
        let (w, h) = (width as u32, height as u32);
        let fit_mode = fit.as_deref().unwrap_or("contain");

        match fit_mode {
            "contain" | "cover" | "fill" => {}
            _ => return Err(ImageError(format!(
                "Unknown fit mode '{}'. Use contain, cover, or fill.", fit_mode
            ))),
        }

        self.frames = self.frames.iter().map(|frame| {
            let img = image::DynamicImage::ImageRgba8(frame.buffer().clone());
            let resized = match fit_mode {
                "fill" => img.resize_exact(w, h, FilterType::Lanczos3),
                "cover" => img.resize_to_fill(w, h, FilterType::Lanczos3),
                _ => img.resize(w, h, FilterType::Lanczos3),
            };
            image::Frame::from_parts(resized.to_rgba8(), 0, 0, frame.delay())
        }).collect();

        Ok(())
    }

    pub fn thumbnail(&mut self, width: i64, height: i64) -> Result<(), ImageError> {
        if width <= 0 || height <= 0 {
            return Err(ImageError("Thumbnail dimensions must be positive".into()));
        }
        let (w, h) = (width as u32, height as u32);

        self.frames = self.frames.iter().map(|frame| {
            let img = image::DynamicImage::ImageRgba8(frame.buffer().clone());
            let resized = img.resize(w, h, FilterType::Triangle);
            image::Frame::from_parts(resized.to_rgba8(), 0, 0, frame.delay())
        }).collect();

        Ok(())
    }

    pub fn crop(&mut self, x: i64, y: i64, width: i64, height: i64) -> Result<(), ImageError> {
        if x < 0 || y < 0 || width <= 0 || height <= 0 {
            return Err(ImageError("Crop parameters must be non-negative and dimensions must be positive".into()));
        }
        let (cx, cy, cw, ch) = (x as u32, y as u32, width as u32, height as u32);

        if let Some(frame) = self.frames.first() {
            let (fw, fh) = frame.buffer().dimensions();
            if cx + cw > fw || cy + ch > fh {
                return Err(ImageError(format!(
                    "Crop region {}x{} at ({},{}) exceeds image bounds {}x{}",
                    cw, ch, cx, cy, fw, fh
                )));
            }
        }

        self.frames = self.frames.iter().map(|frame| {
            let img = image::DynamicImage::ImageRgba8(frame.buffer().clone());
            let cropped = img.crop_imm(cx, cy, cw, ch);
            image::Frame::from_parts(cropped.to_rgba8(), 0, 0, frame.delay())
        }).collect();

        Ok(())
    }

    #[php(defaults(opacity = None))]
    pub fn overlay(&mut self, other: &PhpImage, x: i64, y: i64, opacity: Option<f64>) -> Result<(), ImageError> {
        let overlay_frame = other.frames.first()
            .ok_or_else(|| ImageError("Overlay image has no data".into()))?;
        let opacity = opacity.unwrap_or(1.0) as f32;
        let overlay_img = image::DynamicImage::ImageRgba8(overlay_frame.buffer().clone());

        self.frames = self.frames.iter().map(|frame| {
            image_ops::overlay_frame(frame, &overlay_img, x as i32, y as i32, opacity)
        }).collect();

        Ok(())
    }

    pub fn grayscale(&mut self) -> Result<(), ImageError> {
        self.frames = self.frames.iter().map(|frame| {
            let img = image::DynamicImage::ImageRgba8(frame.buffer().clone());
            let gray = img.grayscale().to_rgba8();
            image::Frame::from_parts(gray, 0, 0, frame.delay())
        }).collect();
        Ok(())
    }

    #[php(defaults(quality = None))]
    pub fn to_jpeg(&mut self, quality: Option<i64>) -> Result<(), ImageError> {
        let q = quality.unwrap_or(85).clamp(0, 100) as u8;
        self.output_format = Some(OutputFormat::Jpeg(q));
        Ok(())
    }

    pub fn to_png(&mut self) -> Result<(), ImageError> {
        self.output_format = Some(OutputFormat::Png);
        Ok(())
    }

    #[php(defaults(quality = None))]
    pub fn to_webp(&mut self, quality: Option<i64>) -> Result<(), ImageError> {
        self.output_format = Some(OutputFormat::Webp(quality.unwrap_or(80).clamp(0, 100) as u8));
        Ok(())
    }

    #[php(defaults(quality = None))]
    pub fn to_avif(&mut self, quality: Option<i64>) -> Result<(), ImageError> {
        self.output_format = Some(OutputFormat::Avif(quality.unwrap_or(60).clamp(0, 100) as u8));
        Ok(())
    }

    pub fn to_gif(&mut self) -> Result<(), ImageError> {
        self.output_format = Some(OutputFormat::Gif);
        Ok(())
    }

    pub fn to_buffer(&self) -> Result<ext_php_rs::binary::Binary<u8>, ImageError> {
        let bytes = self.encode_to_bytes()?;
        Ok(bytes.into())
    }

    pub fn save(&self, path: String) -> Result<(), ImageError> {
        let bytes = self.encode_to_bytes()?;
        std::fs::write(&path, &bytes)
            .map_err(|e| ImageError(format!("Failed to save to '{}': {}", path, e)))
    }

}

impl PhpImage {
    fn first_image(&self) -> Result<image::DynamicImage, ImageError> {
        let frame = self.frames.first()
            .ok_or_else(|| ImageError("No image data".into()))?;
        Ok(image::DynamicImage::ImageRgba8(frame.buffer().clone()))
    }

    fn encode_to_bytes(&self) -> Result<Vec<u8>, ImageError> {
        let format = self.output_format.unwrap_or(OutputFormat::Png);

        match format {
            OutputFormat::Jpeg(quality) => {
                let img = self.first_image()?;
                let mut buf = Vec::new();
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                    std::io::Cursor::new(&mut buf), quality
                );
                img.to_rgb8().write_with_encoder(encoder)
                    .map_err(|e| ImageError(format!("JPEG encoding failed: {}", e)))?;
                Ok(buf)
            }
            OutputFormat::Png => {
                let img = self.first_image()?;
                let mut buf = Vec::new();
                img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                    .map_err(|e| ImageError(format!("PNG encoding failed: {}", e)))?;
                Ok(buf)
            }
            OutputFormat::Webp(quality) => {
                if self.frames.len() > 1 {
                    crate::image_encode::encode_webp_animated(&self.frames, quality)
                } else {
                    let frame = self.frames.first()
                        .ok_or_else(|| ImageError("No image data".into()))?;
                    crate::image_encode::encode_webp(frame, quality)
                }
            }
            OutputFormat::Avif(quality) => {
                let frame = self.frames.first()
                    .ok_or_else(|| ImageError("No image data".into()))?;
                crate::image_encode::encode_avif(frame, quality)
            }
            OutputFormat::Gif => {
                use image::codecs::gif::{GifEncoder, Repeat};
                let mut buf = Vec::new();
                {
                    let mut encoder = GifEncoder::new(&mut buf);
                    encoder.set_repeat(Repeat::Infinite)
                        .map_err(|e| ImageError(format!("Failed to set GIF repeat: {}", e)))?;
                    encoder.encode_frames(self.frames.iter().cloned())
                        .map_err(|e| ImageError(format!("GIF encoding failed: {}", e)))?;
                }
                Ok(buf)
            }
        }
    }
}
