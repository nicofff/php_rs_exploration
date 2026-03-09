use ext_php_rs::prelude::*;
use image::RgbaImage;

use crate::image_decode;
use crate::image_error::ImageError;
use crate::image_info::ImageInfo;

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
}
