use image::DynamicImage;
use crate::image_error::ImageError;
use crate::image::OutputFormat;

pub(crate) trait ImageOps {
    /// Resize. `fit` is one of "contain", "cover", "fill" — validated by caller.
    fn resize(&mut self, width: u32, height: u32, fit: &str) -> Result<(), ImageError>;

    /// Resize preserving aspect ratio with Triangle filter.
    fn thumbnail(&mut self, width: u32, height: u32);

    /// Crop. Returns error if region exceeds bounds.
    fn crop(&mut self, x: u32, y: u32, width: u32, height: u32) -> Result<(), ImageError>;

    /// Convert to grayscale (output is still RGBA).
    fn grayscale(&mut self);

    /// Composite `overlay` on top. `overlay` is always a `DynamicImage` extracted from the other PhpImage.
    fn overlay(&mut self, overlay: &DynamicImage, x: i32, y: i32, opacity: f32);

    /// Dimensions of the first/only frame.
    fn dimensions(&self) -> (u32, u32);

    /// Encode to bytes in the given format.
    fn encode(&self, format: OutputFormat) -> Result<Vec<u8>, ImageError>;

    /// Return the first/only frame as DynamicImage (used to extract overlay source).
    fn first_frame(&self) -> DynamicImage;
}
