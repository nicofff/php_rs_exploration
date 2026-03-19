use std::collections::HashMap;
use std::io::Cursor;
use ext_php_rs::prelude::*;

use ril::{
    encode::{Encoder, EncoderMetadata},
    encodings::jpeg::{JpegEncoder, JpegEncoderOptions},
    encodings::webp::{WebPEncoderOptions, WebPMuxEncoder, WebPStaticEncoder},
    Image, ImageFormat, ImageSequence, L, OverlayMode, Paste, ResizeAlgorithm, Rgba,
};

use crate::image_error::ImageError;
use crate::image_info::ImageInfo;
use crate::rgb::PhpRgb;

// ── ril 0.10 API verification ────────────────────────────────────────────────
// Source: ~/.cargo/registry/src/.../ril-0.10.3/src/image.rs
//
//   Image::crop(x1, y1, x2, y2)
//     → YES. Takes two corner coordinates (top-left and bottom-right), NOT
//       (x, y, width, height). To crop by origin + size, callers must compute
//       x2 = x + w, y2 = y + h before calling.
//
//   Image::flip_vertical / flip
//     → Image::flip(&mut self)  — flips vertically (about the x-axis).
//       Internally: mirror() + rotate_180().
//
//   Image::flip_horizontal / mirror
//     → Image::mirror(&mut self) — flips horizontally (about the y-axis).
//
//   Image::rotate(degrees: i32)
//     → YES, but LIMITED. Only supports 0 / 90 / 180 / 270 degrees (clockwise).
//       Any other value panics with unimplemented!(). Discrete helpers also
//       available: rotate_90(), rotate_180(), rotate_270().
//       For auto_rotate (EXIF orientation), map the orientation tag value to one
//       of these four calls; no arbitrary-angle fallback is needed since EXIF
//       only uses 90-degree steps.
// ────────────────────────────────────────────────────────────────────────────

/// Parse EXIF in one pass, returning the full tag map and orientation together.
fn read_exif_full(path: &str) -> (Option<HashMap<String, String>>, Option<u32>) {
    let Ok(file) = std::fs::File::open(path) else { return (None, None) };
    let mut reader = std::io::BufReader::new(file);
    let Ok(exif) = exif::Reader::new().read_from_container(&mut reader) else { return (None, None) };
    let mut map = HashMap::new();
    let mut orientation = None;
    for field in exif.fields() {
        if field.tag == exif::Tag::Orientation {
            orientation = field.value.get_uint(0);
        }
        map.insert(format!("{}", field.tag), field.display_value().to_string());
    }
    (Some(map), orientation)
}

fn read_exif_full_from_bytes(data: &[u8]) -> (Option<HashMap<String, String>>, Option<u32>) {
    let mut reader = std::io::BufReader::new(Cursor::new(data));
    let Ok(exif) = exif::Reader::new().read_from_container(&mut reader) else { return (None, None) };
    let mut map = HashMap::new();
    let mut orientation = None;
    for field in exif.fields() {
        if field.tag == exif::Tag::Orientation {
            orientation = field.value.get_uint(0);
        }
        map.insert(format!("{}", field.tag), field.display_value().to_string());
    }
    (Some(map), orientation)
}

/// Compute output dimensions for resize modes.
fn compute_fit(src_w: u32, src_h: u32, target_w: u32, target_h: u32, fit: &str) -> (u32, u32) {
    match fit {
        "fill" => (target_w, target_h),
        "cover" => {
            let scale = f64::max(target_w as f64 / src_w as f64, target_h as f64 / src_h as f64);
            let w = ((src_w as f64 * scale).round() as u32).max(1);
            let h = ((src_h as f64 * scale).round() as u32).max(1);
            (w, h)
        }
        _ => {
            // contain: fit within target, preserving aspect ratio
            let scale = f64::min(target_w as f64 / src_w as f64, target_h as f64 / src_h as f64);
            let w = ((src_w as f64 * scale).round() as u32).max(1);
            let h = ((src_h as f64 * scale).round() as u32).max(1);
            (w, h)
        }
    }
}

/// Returns true when the format may contain multiple frames (GIF or WebP).
fn format_may_animate(fmt: ImageFormat) -> bool {
    matches!(fmt, ImageFormat::Gif | ImageFormat::WebP)
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum OutputFormat {
    Jpeg(u8),
    Png,
    Gif,
    Webp(u8),
}

pub(crate) enum ImageInner {
    Static(Image<Rgba>),
    Animated(ImageSequence<Rgba>),
}

#[php_class]
#[php(name = "RustImage\\Image")]
pub struct PhpImage {
    pub(crate) inner: ImageInner,
    pub(crate) output_format: Option<OutputFormat>,
    pub(crate) exif_data: Option<HashMap<String, String>>,
    pub(crate) orientation: Option<u32>,
}

#[php_impl]
impl PhpImage {
    pub fn open(path: String) -> Result<Self, ImageError> {
        let inner = if format_may_animate(ImageFormat::from_path(&path).unwrap_or_default()) {
            let seq = ImageSequence::<Rgba>::open(&path)
                .map_err(|e| ImageError(format!("Failed to open sequence '{}': {}", path, e)))?
                .into_sequence()
                .map_err(|e| ImageError(format!("Failed to decode frames '{}': {}", path, e)))?;
            if seq.len() > 1 {
                ImageInner::Animated(seq)
            } else {
                ImageInner::Static(
                    Image::<Rgba>::open(&path)
                        .map_err(|e| ImageError(format!("Failed to open '{}': {}", path, e)))?,
                )
            }
        } else {
            ImageInner::Static(
                Image::<Rgba>::open(&path)
                    .map_err(|e| ImageError(format!("Failed to open '{}': {}", path, e)))?,
            )
        };
        let (exif_data, orientation) = read_exif_full(&path);
        Ok(Self {
            inner,
            output_format: None,
            exif_data,
            orientation,
        })
    }

    pub fn from_buffer(bytes: ext_php_rs::binary::Binary<u8>) -> Result<Self, ImageError> {
        let data: &[u8] = bytes.as_ref();
        let inner = if format_may_animate(ImageFormat::infer_encoding(data)) {
            let seq = ImageSequence::<Rgba>::from_bytes_inferred(data)
                .map_err(|e| ImageError(format!("Failed to decode sequence from buffer: {}", e)))?
                .into_sequence()
                .map_err(|e| ImageError(format!("Failed to collect frames from buffer: {}", e)))?;
            if seq.len() > 1 {
                ImageInner::Animated(seq)
            } else {
                ImageInner::Static(
                    Image::<Rgba>::from_bytes_inferred(data)
                        .map_err(|e| ImageError(format!("Failed to decode image from buffer: {}", e)))?,
                )
            }
        } else {
            ImageInner::Static(
                Image::<Rgba>::from_bytes_inferred(data)
                    .map_err(|e| ImageError(format!("Failed to decode image from buffer: {}", e)))?,
            )
        };
        let (exif_data, orientation) = read_exif_full_from_bytes(data);
        Ok(Self {
            inner,
            output_format: None,
            exif_data,
            orientation,
        })
    }

    pub fn info(path: String) -> Result<ImageInfo, ImageError> {
        let img = Image::<Rgba>::open(&path)
            .map_err(|e| ImageError(format!("Failed to open '{}': {}", path, e)))?;
        let (width, height) = img.dimensions();

        let format = ImageFormat::from_path(&path)
            .map(|f| format!("{f}"))
            .unwrap_or_else(|_| "unknown".to_string());

        let is_animated = if format_may_animate(ImageFormat::from_path(&path).unwrap_or_default()) {
            ImageSequence::<Rgba>::open(&path)
                .and_then(|iter| iter.into_sequence())
                .map(|seq| seq.len() > 1)
                .unwrap_or(false)
        } else {
            false
        };

        Ok(ImageInfo {
            width,
            height,
            format,
            is_animated,
            exif_data: read_exif_full(&path).0,
        })
    }

    #[php(defaults(fit = None))]
    pub fn resize(&mut self, width: i64, height: i64, fit: Option<String>) -> Result<(), ImageError> {
        if width <= 0 || height <= 0 {
            return Err(ImageError("Resize dimensions must be positive".into()));
        }
        let fit_str = fit.as_deref().unwrap_or("contain");
        match fit_str {
            "contain" | "cover" | "fill" => {}
            _ => return Err(ImageError(format!(
                "Unknown fit mode '{}'. Use contain, cover, or fill.", fit_str
            ))),
        }

        let tw = width as u32;
        let th = height as u32;

        match &mut self.inner {
            ImageInner::Static(img) => {
                let (sw, sh) = img.dimensions();
                let (nw, nh) = compute_fit(sw, sh, tw, th, fit_str);
                img.resize(nw, nh, ResizeAlgorithm::Lanczos3);
            }
            ImageInner::Animated(seq) => {
                for frame in seq.iter_mut() {
                    let (sw, sh) = frame.image().dimensions();
                    let (nw, nh) = compute_fit(sw, sh, tw, th, fit_str);
                    frame.image_mut().resize(nw, nh, ResizeAlgorithm::Lanczos3);
                }
            }
        }
        Ok(())
    }

    #[php(defaults(opacity = None))]
    pub fn overlay(&mut self, other: &PhpImage, x: i64, y: i64, opacity: Option<f64>) -> Result<(), ImageError> {
        let opacity = (opacity.unwrap_or(1.0) as f32).clamp(0.0, 1.0);

        let mut overlay_img: Image<Rgba> = match &other.inner {
            ImageInner::Static(img) => img.clone(),
            ImageInner::Animated(seq) => seq.iter().next()
                .map(|f| f.image().clone())
                .ok_or_else(|| ImageError("Overlay source has no frames".into()))?,
        };

        // Scale every pixel's alpha by the opacity factor.
        if opacity < 1.0 {
            let opacity_u8 = (opacity * 255.0 + 0.5) as u32;
            overlay_img = overlay_img.map_alpha_pixels(|a| L(((a.0 as u32 * opacity_u8 + 127) / 255) as u8));
        }

        // Paste requires u32 coordinates; crop the off-screen portion when x/y are negative.
        let crop_x = if x < 0 { (-x) as u32 } else { 0 };
        let crop_y = if y < 0 { (-y) as u32 } else { 0 };
        if crop_x >= overlay_img.width() || crop_y >= overlay_img.height() {
            return Ok(()); // overlay is entirely off-screen
        }
        if crop_x > 0 || crop_y > 0 {
            let (ow, oh) = overlay_img.dimensions();
            overlay_img = crop_image(&overlay_img, crop_x, crop_y, ow - crop_x, oh - crop_y);
        }
        let paste_x = if x >= 0 { x as u32 } else { 0 };
        let paste_y = if y >= 0 { y as u32 } else { 0 };

        let paste_op = Paste::new(&overlay_img)
            .with_position(paste_x, paste_y)
            .with_overlay_mode(OverlayMode::Merge);

        match &mut self.inner {
            ImageInner::Static(base) => {
                base.draw(&paste_op);
            }
            ImageInner::Animated(seq) => {
                for frame in seq.iter_mut() {
                    frame.image_mut().draw(&paste_op);
                }
            }
        }
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

    pub fn create(width: i64, height: i64, color: &PhpRgb) -> Result<Self, ImageError> {
        if width <= 0 || height <= 0 {
            return Err(ImageError("create: width and height must be positive".into()));
        }
        if width > u32::MAX as i64 || height > u32::MAX as i64 {
            return Err(ImageError("create: dimensions exceed u32::MAX".into()));
        }
        Ok(Self {
            inner: ImageInner::Static(Image::new(width as u32, height as u32, color.to_rgba())),
            output_format: None,
            exif_data: None,
            orientation: None,
        })
    }

    pub fn flip(&mut self) -> Result<(), ImageError> {
        match &mut self.inner {
            ImageInner::Static(img) => { img.flip(); }
            ImageInner::Animated(seq) => {
                for frame in seq.iter_mut() {
                    frame.image_mut().flip();
                }
            }
        }
        Ok(())
    }

    pub fn mirror(&mut self) -> Result<(), ImageError> {
        match &mut self.inner {
            ImageInner::Static(img) => { img.mirror(); }
            ImageInner::Animated(seq) => {
                for frame in seq.iter_mut() {
                    frame.image_mut().mirror();
                }
            }
        }
        Ok(())
    }

    pub fn auto_rotate(&mut self) -> Result<(), ImageError> {
        let orientation = match self.orientation {
            None | Some(1) => return Ok(()),
            Some(v) if v > 8 => return Ok(()),
            Some(v) => v,
        };

        match orientation {
            2 => self.mirror()?,
            3 => apply_rotation(&mut self.inner, 180),
            4 => self.flip()?,
            5 => {
                apply_rotation(&mut self.inner, 90);
                self.mirror()?;
            }
            6 => apply_rotation(&mut self.inner, 90),
            7 => {
                apply_rotation(&mut self.inner, 270);
                self.mirror()?;
            }
            8 => apply_rotation(&mut self.inner, 270),
            _ => unreachable!(),
        }

        self.orientation = Some(1);
        Ok(())
    }

    pub fn crop(&mut self, x: i64, y: i64, width: i64, height: i64) -> Result<(), ImageError> {
        if x < 0 || y < 0 {
            return Err(ImageError("crop: x and y must be non-negative".into()));
        }
        if width <= 0 || height <= 0 {
            return Err(ImageError("crop: width and height must be positive".into()));
        }
        if x > u32::MAX as i64 || y > u32::MAX as i64
            || width > u32::MAX as i64 || height > u32::MAX as i64
        {
            return Err(ImageError("crop: dimensions exceed u32::MAX".into()));
        }

        let xu = x as u32;
        let yu = y as u32;
        let wu = width as u32;
        let hu = height as u32;

        match &mut self.inner {
            ImageInner::Static(img) => {
                let (iw, ih) = img.dimensions();
                if x + width > iw as i64 || y + height > ih as i64 {
                    return Err(ImageError("crop: region exceeds image bounds".into()));
                }
                *img = crop_image(img, xu, yu, wu, hu);
            }
            ImageInner::Animated(seq) => {
                // Validate bounds against the first (largest) frame before mutating.
                let first_dims = seq.iter().next().map(|f| f.image().dimensions());
                if let Some((iw, ih)) = first_dims {
                    if x + width > iw as i64 || y + height > ih as i64 {
                        return Err(ImageError("crop: region exceeds image bounds".into()));
                    }
                }
                for frame in seq.iter_mut() {
                    let (iw, ih) = frame.image().dimensions();
                    // Sub-frames (delta frames) may be smaller; clamp crop to their actual size.
                    let fx2 = (xu + wu).min(iw);
                    let fy2 = (yu + hu).min(ih);
                    let fx1 = xu.min(fx2);
                    let fy1 = yu.min(fy2);
                    let fw = fx2.saturating_sub(fx1);
                    let fh = fy2.saturating_sub(fy1);
                    if fw == 0 || fh == 0 {
                        // Ideally this frame should be dropped from the sequence, but
                        // ImageSequence does not expose a removal API; 1×1 transparent is a safe
                        // fallback.
                        *frame.image_mut() = Image::new(1, 1, Rgba { r: 0, g: 0, b: 0, a: 0 });
                    } else {
                        let cropped = crop_image(frame.image(), fx1, fy1, fw, fh);
                        *frame.image_mut() = cropped;
                    }
                }
            }
        }
        Ok(())
    }
}

/// Crop `img` to the rectangle (x, y, w, h) by pixel copy.
/// ril's `crop()` takes corner coordinates `(x1, y1, x2, y2)` while the PHP API uses origin+size
/// `(x, y, w, h)`, and ril's crop mutates in place — so a manual pixel-copy approach is used to
/// return a new image of exactly the right size.
fn crop_image(img: &Image<Rgba>, x: u32, y: u32, w: u32, h: u32) -> Image<Rgba> {
    let mut dst = Image::new(w, h, Rgba { r: 0, g: 0, b: 0, a: 0 });
    for dy in 0..h {
        for dx in 0..w {
            dst.set_pixel(dx, dy, *img.pixel(x + dx, y + dy));
        }
    }
    dst
}

/// Apply an in-place rotation (0/90/180/270 degrees CW) to all frames of an ImageInner.
fn apply_rotation(inner: &mut ImageInner, degrees: i32) {
    match inner {
        ImageInner::Static(img) => { img.rotate(degrees); }
        ImageInner::Animated(seq) => {
            for frame in seq.iter_mut() {
                frame.image_mut().rotate(degrees);
            }
        }
    }
}


impl PhpImage {
    fn encode_to_bytes(&self) -> Result<Vec<u8>, ImageError> {
        let format = self.output_format.unwrap_or(OutputFormat::Png);
        let mut buf = Vec::new();

        match (&self.inner, format) {
            (ImageInner::Static(img), OutputFormat::Jpeg(quality)) => {
                encode_jpeg_static(img, quality, &mut buf)?;
            }
            (ImageInner::Static(img), OutputFormat::Png) => {
                img.encode(ImageFormat::Png, &mut Cursor::new(&mut buf))
                    .map_err(|e| ImageError(format!("PNG encoding failed: {}", e)))?;
            }
            (ImageInner::Static(img), OutputFormat::Webp(quality)) => {
                encode_webp_static(img, quality, &mut buf)?;
            }
            (ImageInner::Static(img), OutputFormat::Gif) => {
                img.encode(ImageFormat::Gif, &mut Cursor::new(&mut buf))
                    .map_err(|e| ImageError(format!("GIF encoding failed: {}", e)))?;
            }
            (ImageInner::Animated(seq), OutputFormat::Gif) => {
                seq.encode(ImageFormat::Gif, &mut Cursor::new(&mut buf))
                    .map_err(|e| ImageError(format!("GIF encoding failed: {}", e)))?;
            }
            (ImageInner::Animated(seq), OutputFormat::Jpeg(quality)) => {
                let frame = seq.iter().next()
                    .ok_or_else(|| ImageError("No frames to encode".into()))?;
                encode_jpeg_static(frame.image(), quality, &mut buf)?;
            }
            (ImageInner::Animated(seq), OutputFormat::Png) => {
                let frame = seq.iter().next()
                    .ok_or_else(|| ImageError("No frames to encode".into()))?;
                frame.image().encode(ImageFormat::Png, &mut Cursor::new(&mut buf))
                    .map_err(|e| ImageError(format!("PNG encoding failed: {}", e)))?;
            }
            (ImageInner::Animated(seq), OutputFormat::Webp(quality)) => {
                encode_webp_animated(seq, quality, &mut buf)?;
            }
        }

        Ok(buf)
    }
}

fn encode_jpeg_static(img: &Image<Rgba>, quality: u8, buf: &mut Vec<u8>) -> Result<(), ImageError> {
    let opts = JpegEncoderOptions::new().with_quality(quality);
    let metadata = EncoderMetadata::from(img).with_config(opts);
    let mut encoder = JpegEncoder::new(Cursor::new(&mut *buf), metadata)
        .map_err(|e| ImageError(format!("JPEG encoder init failed: {}", e)))?;
    encoder.add_frame(img)
        .map_err(|e| ImageError(format!("JPEG frame encoding failed: {}", e)))?;
    encoder.finish()
        .map_err(|e| ImageError(format!("JPEG encoding failed: {}", e)))?;
    Ok(())
}

fn encode_webp_static(img: &Image<Rgba>, quality: u8, buf: &mut Vec<u8>) -> Result<(), ImageError> {
    let opts = WebPEncoderOptions::new().with_quality(quality as f32);
    let metadata = EncoderMetadata::from(img).with_config(opts);
    let mut encoder = WebPStaticEncoder::new(Cursor::new(&mut *buf), metadata)
        .map_err(|e| ImageError(format!("WebP encoder init failed: {}", e)))?;
    encoder.add_frame(img)
        .map_err(|e| ImageError(format!("WebP frame encoding failed: {}", e)))?;
    encoder.finish()
        .map_err(|e| ImageError(format!("WebP encoding failed: {}", e)))?;
    Ok(())
}

fn encode_webp_animated(seq: &ImageSequence<Rgba>, quality: u8, buf: &mut Vec<u8>) -> Result<(), ImageError> {
    let opts = WebPEncoderOptions::new().with_quality(quality as f32);
    let metadata = EncoderMetadata::from(seq).with_config(opts);
    let mut encoder = WebPMuxEncoder::new(Cursor::new(&mut *buf), metadata)
        .map_err(|e| ImageError(format!("WebP animated encoder init failed: {}", e)))?;
    for frame in seq.iter() {
        encoder.add_frame(frame)
            .map_err(|e| ImageError(format!("WebP animated frame failed: {}", e)))?;
    }
    encoder.finish()
        .map_err(|e| ImageError(format!("WebP animated encoding failed: {}", e)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ril::{Image, Rgba};

    // ── compute_fit ────────────────────────────────────────────────────────

    #[test]
    fn compute_fit_contain_landscape_into_square() {
        // 200×100 contained in 100×100 → limited by width: 100×50
        let (w, h) = compute_fit(200, 100, 100, 100, "contain");
        assert_eq!(w, 100);
        assert_eq!(h, 50);
    }

    #[test]
    fn compute_fit_contain_portrait_into_square() {
        // 100×200 contained in 100×100 → limited by height: 50×100
        let (w, h) = compute_fit(100, 200, 100, 100, "contain");
        assert_eq!(w, 50);
        assert_eq!(h, 100);
    }

    #[test]
    fn compute_fit_contain_equal_dimensions() {
        let (w, h) = compute_fit(100, 100, 50, 50, "contain");
        assert_eq!(w, 50);
        assert_eq!(h, 50);
    }

    #[test]
    fn compute_fit_contain_one_by_one_source() {
        let (w, h) = compute_fit(1, 1, 100, 100, "contain");
        assert_eq!(w, 100);
        assert_eq!(h, 100);
    }

    #[test]
    fn compute_fit_cover_landscape() {
        // 200×100 covered by 100×100 → scale = max(0.5, 1.0) = 1.0 → 200×100
        let (w, h) = compute_fit(200, 100, 100, 100, "cover");
        assert_eq!(w, 200);
        assert_eq!(h, 100);
    }

    #[test]
    fn compute_fit_fill_always_exact() {
        let (w, h) = compute_fit(200, 100, 77, 33, "fill");
        assert_eq!(w, 77);
        assert_eq!(h, 33);
    }

    // ── crop_image ─────────────────────────────────────────────────────────

    #[test]
    fn crop_image_output_dimensions_correct() {
        let src = Image::new(100, 80, Rgba { r: 0, g: 0, b: 0, a: 255 });
        let cropped = crop_image(&src, 10, 20, 30, 25);
        assert_eq!(cropped.width(), 30);
        assert_eq!(cropped.height(), 25);
    }

    #[test]
    fn crop_image_origin_pixel_is_correct_source_pixel() {
        let known = Rgba { r: 200, g: 100, b: 50, a: 255 };
        let mut src = Image::new(50, 50, Rgba { r: 0, g: 0, b: 0, a: 255 });
        // Paint a known color at (15, 20)
        src.set_pixel(15, 20, known);
        // Crop starting at (15, 20) — pixel (0,0) in result should be `known`
        let cropped = crop_image(&src, 15, 20, 10, 10);
        assert_eq!(*cropped.pixel(0, 0), known);
    }

    // ── read_exif_orientation_from_bytes ───────────────────────────────────

    #[test]
    fn read_exif_orientation_non_jpeg_returns_none() {
        let (_, orientation) = read_exif_full_from_bytes(b"not a jpeg at all");
        assert!(orientation.is_none());
    }
}
