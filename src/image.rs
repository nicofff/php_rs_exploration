use std::collections::HashMap;
use ext_php_rs::prelude::*;

use crate::image_animated::AnimatedImage;
use crate::image_error::ImageError;
use crate::image_info::ImageInfo;
use crate::image_ops_trait::ImageOps;
use crate::image_static::StaticImage;

fn read_exif(path: &str) -> Option<HashMap<String, String>> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
    let mut map = HashMap::new();
    for field in exif.fields() {
        map.insert(format!("{}", field.tag), field.display_value().to_string());
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

pub(crate) enum ImageInner {
    Static(StaticImage),
    Animated(AnimatedImage),
}

impl ImageInner {
    fn as_ops(&self) -> &dyn ImageOps {
        match self {
            ImageInner::Static(s)   => s,
            ImageInner::Animated(a) => a,
        }
    }
    fn as_ops_mut(&mut self) -> &mut dyn ImageOps {
        match self {
            ImageInner::Static(s)   => s,
            ImageInner::Animated(a) => a,
        }
    }
}

#[php_class]
#[php(name = "RustImage\\Image")]
pub struct PhpImage {
    pub(crate) inner: ImageInner,
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
            let file_size = std::fs::metadata(&path)
                .map_err(|e| ImageError(format!("Failed to read file metadata '{}': {}", path, e)))?
                .len() as i64;
            if file_size > limit {
                return Err(ImageError(format!(
                    "File size {} bytes exceeds limit of {} bytes", file_size, limit
                )));
            }
        }

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

        let reader = image::ImageReader::open(&path)
            .map_err(|e| ImageError(format!("Failed to open image '{}': {}", path, e)))?
            .with_guessed_format()
            .map_err(|e| ImageError(format!("Failed to guess format for '{}': {}", path, e)))?;
        let format = reader.format();

        // Any format whose decoder implements AnimationDecoder goes to AnimatedImage.
        // Re-open the file for the decoder (reader already consumed it for format detection).
        let inner = match format {
            Some(image::ImageFormat::Gif) => {
                use image::codecs::gif::GifDecoder;
                use image::AnimationDecoder;
                let file = std::fs::File::open(&path)
                    .map_err(|e| ImageError(format!("Failed to open '{}': {}", path, e)))?;
                let frames = GifDecoder::new(std::io::BufReader::new(file))
                    .map_err(|e| ImageError(format!("Failed to create GIF decoder: {}", e)))?
                    .into_frames().collect_frames()
                    .map_err(|e| ImageError(format!("Failed to read GIF frames: {}", e)))?;
                ImageInner::Animated(AnimatedImage(frames))
            }
            Some(image::ImageFormat::WebP) => {
                use image::codecs::webp::WebPDecoder;
                use image::AnimationDecoder;
                let file = std::fs::File::open(&path)
                    .map_err(|e| ImageError(format!("Failed to open '{}': {}", path, e)))?;
                let decoder = WebPDecoder::new(std::io::BufReader::new(file))
                    .map_err(|e| ImageError(format!("Failed to create WebP decoder: {}", e)))?;
                if decoder.has_animation() {
                    let frames = decoder.into_frames().collect_frames()
                        .map_err(|e| ImageError(format!("Failed to read WebP frames: {}", e)))?;
                    ImageInner::Animated(AnimatedImage(frames))
                } else {
                    let img = image::ImageReader::open(&path)
                        .map_err(|e| ImageError(format!("Failed to open '{}': {}", path, e)))?
                        .with_guessed_format()
                        .map_err(|e| ImageError(format!("Failed to guess format: {}", e)))?
                        .decode()
                        .map_err(|e| ImageError(format!("Failed to decode image: {}", e)))?;
                    ImageInner::Static(StaticImage(img))
                }
            }
            _ => {
                let img = reader.decode()
                    .map_err(|e| ImageError(format!("Failed to decode image '{}': {}", path, e)))?;
                ImageInner::Static(StaticImage(img))
            }
        };

        // Post-decode dimension check
        let (w, h) = inner.as_ops().dimensions();
        if let Some(max_w) = max_width {
            if w as i64 > max_w {
                return Err(ImageError(format!("Image width {} exceeds limit of {}", w, max_w)));
            }
        }
        if let Some(max_h) = max_height {
            if h as i64 > max_h {
                return Err(ImageError(format!("Image height {} exceeds limit of {}", h, max_h)));
            }
        }

        Ok(Self { inner, output_format: None })
    }

    pub fn from_buffer(bytes: ext_php_rs::binary::Binary<u8>) -> Result<Self, ImageError> {
        let img = image::load_from_memory(bytes.as_ref())
            .map_err(|e| ImageError(format!("Failed to decode image from buffer: {}", e)))?;
        Ok(Self {
            inner: ImageInner::Static(StaticImage(img)),
            output_format: None,
        })
    }

    pub fn copy(&self) -> Self {
        let inner = match &self.inner {
            ImageInner::Static(s)   => ImageInner::Static(StaticImage(s.0.clone())),
            ImageInner::Animated(a) => ImageInner::Animated(AnimatedImage(a.0.clone())),
        };
        Self { inner, output_format: self.output_format }
    }

    pub fn info(path: String) -> Result<ImageInfo, ImageError> {
        let reader = image::ImageReader::open(&path)
            .map_err(|e| ImageError(format!("Failed to open '{}': {}", path, e)))?
            .with_guessed_format()
            .map_err(|e| ImageError(format!("Failed to guess format: {}", e)))?;
        let format_str = reader.format()
            .map(|f| format!("{:?}", f).to_lowercase())
            .unwrap_or_else(|| "unknown".to_string());
        let format_enum = reader.format();
        let (width, height) = reader.into_dimensions()
            .map_err(|e| ImageError(format!("Failed to read dimensions: {}", e)))?;

        // Check for animation by attempting AnimationDecoder on formats that support it.
        let is_animated = match format_enum {
            Some(image::ImageFormat::Gif) => {
                use image::codecs::gif::GifDecoder;
                use image::AnimationDecoder;
                if let Ok(file) = std::fs::File::open(&path) {
                    if let Ok(decoder) = GifDecoder::new(std::io::BufReader::new(file)) {
                        if let Ok(frames) = decoder.into_frames().collect_frames() {
                            frames.len() > 1
                        } else { false }
                    } else { false }
                } else { false }
            }
            Some(image::ImageFormat::WebP) => {
                use image::codecs::webp::WebPDecoder;
                if let Ok(file) = std::fs::File::open(&path) {
                    if let Ok(decoder) = WebPDecoder::new(std::io::BufReader::new(file)) {
                        decoder.has_animation()
                    } else { false }
                } else { false }
            }
            _ => false,
        };

        Ok(ImageInfo { width, height, format: format_str, has_alpha: false, is_animated, exif_data: read_exif(&path) })
    }

    #[php(defaults(fit = None))]
    pub fn resize(&mut self, width: i64, height: i64, fit: Option<String>) -> Result<(), ImageError> {
        if width <= 0 || height <= 0 {
            return Err(ImageError("Resize dimensions must be positive".into()));
        }
        let fit_mode = fit.as_deref().unwrap_or("contain");
        match fit_mode {
            "contain" | "cover" | "fill" => {}
            _ => return Err(ImageError(format!(
                "Unknown fit mode '{}'. Use contain, cover, or fill.", fit_mode
            ))),
        }
        self.inner.as_ops_mut().resize(width as u32, height as u32, fit_mode)
    }

    pub fn thumbnail(&mut self, width: i64, height: i64) -> Result<(), ImageError> {
        if width <= 0 || height <= 0 {
            return Err(ImageError("Thumbnail dimensions must be positive".into()));
        }
        self.inner.as_ops_mut().thumbnail(width as u32, height as u32);
        Ok(())
    }

    pub fn crop(&mut self, x: i64, y: i64, width: i64, height: i64) -> Result<(), ImageError> {
        if x < 0 || y < 0 || width <= 0 || height <= 0 {
            return Err(ImageError("Crop parameters must be non-negative and dimensions must be positive".into()));
        }
        self.inner.as_ops_mut().crop(x as u32, y as u32, width as u32, height as u32)
    }

    #[php(defaults(opacity = None))]
    pub fn overlay(&mut self, other: &PhpImage, x: i64, y: i64, opacity: Option<f64>) -> Result<(), ImageError> {
        let overlay_img = other.inner.as_ops().first_frame();
        self.inner.as_ops_mut().overlay(&overlay_img, x as i32, y as i32, opacity.unwrap_or(1.0) as f32);
        Ok(())
    }

    pub fn grayscale(&mut self) -> Result<(), ImageError> {
        self.inner.as_ops_mut().grayscale();
        Ok(())
    }

    #[php(defaults(quality = None))]
    pub fn to_jpeg(&mut self, quality: Option<i64>) -> Result<(), ImageError> {
        self.output_format = Some(OutputFormat::Jpeg(quality.unwrap_or(85).clamp(0, 100) as u8));
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
    fn encode_to_bytes(&self) -> Result<Vec<u8>, ImageError> {
        let format = self.output_format.unwrap_or(OutputFormat::Png);
        self.inner.as_ops().encode(format)
    }
}
