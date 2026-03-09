use ext_php_rs::prelude::*;
use image::RgbaImage;

use crate::image_decode;
use crate::image_error::ImageError;
use crate::image_info::ImageInfo;
use crate::image_ops;

pub(crate) struct Frame {
    pub(crate) buffer: RgbaImage,
    pub(crate) delay_ms: u32,
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
    pub frames: Vec<Frame>,
    pub output_format: Option<OutputFormat>,
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
        // Check file size against max_bytes if provided
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

        let frames = image_decode::decode_static_from_path(&path)?;

        // Check first frame dimensions against limits if provided
        if let Some(frame) = frames.first() {
            let (w, h) = frame.buffer.dimensions();
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
        let frames = crate::image_decode::decode_static_from_buffer(bytes.as_ref())?;
        Ok(Self { frames, output_format: None })
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

        Ok(ImageInfo {
            width,
            height,
            format,
            has_alpha: false,
            is_animated: false,
        })
    }

    #[php(defaults(fit = None))]
    pub fn resize(&mut self, width: i64, height: i64, fit: Option<String>) -> Result<(), ImageError> {
        use fast_image_resize::{FilterType, ResizeAlg};

        if width <= 0 || height <= 0 {
            return Err(ImageError("Resize dimensions must be positive".into()));
        }
        let target_w = width as u32;
        let target_h = height as u32;
        let fit_mode = fit.as_deref().unwrap_or("contain");
        let algorithm = ResizeAlg::Convolution(FilterType::Lanczos3);

        let mut new_frames = Vec::with_capacity(self.frames.len());
        for frame in &self.frames {
            let (src_w, src_h) = frame.buffer.dimensions();
            let resized = match fit_mode {
                "contain" => {
                    let (new_w, new_h) = image_ops::fit_contain(src_w, src_h, target_w, target_h);
                    image_ops::resize_frame(frame, new_w, new_h, algorithm)?
                }
                "cover" => {
                    let (scaled_w, scaled_h, crop_x, crop_y) =
                        image_ops::fit_cover(src_w, src_h, target_w, target_h);
                    let scaled = image_ops::resize_frame(frame, scaled_w, scaled_h, algorithm)?;
                    image_ops::crop_frame(&scaled, crop_x, crop_y, target_w, target_h)?
                }
                "fill" => {
                    image_ops::resize_frame(frame, target_w, target_h, algorithm)?
                }
                _ => {
                    return Err(ImageError(format!("Unknown fit mode '{}'. Use contain, cover, or fill.", fit_mode)));
                }
            };
            new_frames.push(resized);
        }
        self.frames = new_frames;
        Ok(())
    }

    pub fn thumbnail(&mut self, width: i64, height: i64) -> Result<(), ImageError> {
        use fast_image_resize::{FilterType, ResizeAlg};

        if width <= 0 || height <= 0 {
            return Err(ImageError("Thumbnail dimensions must be positive".into()));
        }
        let target_w = width as u32;
        let target_h = height as u32;
        let algorithm = ResizeAlg::Interpolation(FilterType::Bilinear);

        let mut new_frames = Vec::with_capacity(self.frames.len());
        for frame in &self.frames {
            let (src_w, src_h) = frame.buffer.dimensions();
            let (new_w, new_h) = image_ops::fit_contain(src_w, src_h, target_w, target_h);
            let resized = image_ops::resize_frame(frame, new_w, new_h, algorithm)?;
            new_frames.push(resized);
        }
        self.frames = new_frames;
        Ok(())
    }

    pub fn crop(&mut self, x: i64, y: i64, width: i64, height: i64) -> Result<(), ImageError> {
        if x < 0 || y < 0 || width <= 0 || height <= 0 {
            return Err(ImageError("Crop parameters must be non-negative and dimensions must be positive".into()));
        }
        let mut new_frames = Vec::with_capacity(self.frames.len());
        for frame in &self.frames {
            let cropped = image_ops::crop_frame(frame, x as u32, y as u32, width as u32, height as u32)?;
            new_frames.push(cropped);
        }
        self.frames = new_frames;
        Ok(())
    }

    pub fn grayscale(&mut self) -> Result<(), ImageError> {
        let mut new_frames = Vec::with_capacity(self.frames.len());
        for frame in &self.frames {
            new_frames.push(image_ops::grayscale_frame(frame));
        }
        self.frames = new_frames;
        Ok(())
    }

    #[php(defaults(quality = None))]
    pub fn to_jpeg(&mut self, quality: Option<i64>) -> Result<(), ImageError> {
        let q = quality.unwrap_or(85) as u8;
        self.output_format = Some(OutputFormat::Jpeg(q));
        Ok(())
    }

    pub fn to_png(&mut self) -> Result<(), ImageError> {
        self.output_format = Some(OutputFormat::Png);
        Ok(())
    }

    #[php(defaults(quality = None))]
    pub fn to_webp(&mut self, quality: Option<i64>) -> Result<(), ImageError> {
        self.output_format = Some(OutputFormat::Webp(quality.unwrap_or(80) as u8));
        Ok(())
    }

    #[php(defaults(quality = None))]
    pub fn to_avif(&mut self, quality: Option<i64>) -> Result<(), ImageError> {
        self.output_format = Some(OutputFormat::Avif(quality.unwrap_or(60) as u8));
        Ok(())
    }

    pub fn save(&self, path: String) -> Result<(), ImageError> {
        let frame = self.frames.first()
            .ok_or_else(|| ImageError("No frames to save".into()))?;

        match self.output_format {
            Some(OutputFormat::Jpeg(quality)) => {
                // Convert RGBA to RGB for JPEG
                let (w, h) = frame.buffer.dimensions();
                let mut rgb_buf = Vec::with_capacity((w * h * 3) as usize);
                for pixel in frame.buffer.pixels() {
                    rgb_buf.push(pixel[0]);
                    rgb_buf.push(pixel[1]);
                    rgb_buf.push(pixel[2]);
                }
                let rgb_image = image::RgbImage::from_raw(w, h, rgb_buf)
                    .ok_or_else(|| ImageError("Failed to create RGB image".into()))?;

                let file = std::fs::File::create(&path)
                    .map_err(|e| ImageError(format!("Failed to create file '{}': {}", path, e)))?;
                let mut buf_writer = std::io::BufWriter::new(file);
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf_writer, quality);
                rgb_image.write_with_encoder(encoder)
                    .map_err(|e| ImageError(format!("JPEG encoding failed: {}", e)))?;
            }
            Some(OutputFormat::Webp(quality)) => {
                let data = crate::image_encode::encode_webp(frame, quality)?;
                std::fs::write(&path, &data)
                    .map_err(|e| ImageError(format!("Failed to save WebP '{}': {}", path, e)))?;
            }
            Some(OutputFormat::Avif(quality)) => {
                let data = crate::image_encode::encode_avif(frame, quality)?;
                std::fs::write(&path, &data)
                    .map_err(|e| ImageError(format!("Failed to save AVIF '{}': {}", path, e)))?;
            }
            Some(OutputFormat::Png) | None => {
                frame.buffer.save(&path)
                    .map_err(|e| ImageError(format!("Failed to save image: {}", e)))?;
            }
            _ => {
                return Err(ImageError("Unsupported output format for save".into()));
            }
        }
        Ok(())
    }
}
