use ext_php_rs::prelude::*;
use image::RgbaImage;

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
    pub fn open(path: String) -> Result<Self, ImageError> {
        let img = image::open(&path)
            .map_err(|e| ImageError(format!("Failed to open image '{}': {}", path, e)))?;
        let rgba = img.to_rgba8();
        Ok(Self {
            frames: vec![Frame { buffer: rgba, delay_ms: 0 }],
            output_format: None,
        })
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
