use std::collections::HashMap;
use std::io::Cursor;
use ext_php_rs::prelude::*;

use ril::{
    encode::{Encoder, EncoderMetadata},
    encodings::jpeg::{JpegEncoder, JpegEncoderOptions},
    encodings::webp::{WebPEncoderOptions, WebPMuxEncoder, WebPStaticEncoder},
    Image, ImageFormat, ImageSequence, ResizeAlgorithm, Rgba,
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

fn read_exif_orientation(path: &str) -> Option<u32> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
    exif.fields()
        .find(|f| f.tag == exif::Tag::Orientation)
        .and_then(|f| f.value.get_uint(0))
}

fn read_exif_from_bytes(data: &[u8]) -> Option<HashMap<String, String>> {
    let mut reader = std::io::BufReader::new(Cursor::new(data));
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
    let mut map = HashMap::new();
    for field in exif.fields() {
        map.insert(format!("{}", field.tag), field.display_value().to_string());
    }
    Some(map)
}

fn read_exif_orientation_from_bytes(data: &[u8]) -> Option<u32> {
    let mut reader = std::io::BufReader::new(std::io::Cursor::new(data));
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
    exif.fields()
        .find(|f| f.tag == exif::Tag::Orientation)
        .and_then(|f| f.value.get_uint(0))
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
        Ok(Self {
            inner,
            output_format: None,
            exif_data: read_exif(&path),
            orientation: read_exif_orientation(&path),
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
        Ok(Self {
            inner,
            output_format: None,
            exif_data: read_exif_from_bytes(data),
            orientation: read_exif_orientation_from_bytes(data),
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
            exif_data: read_exif(&path),
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
        let opacity = opacity.unwrap_or(1.0) as f32;
        let overlay_img: Image<Rgba> = match &other.inner {
            ImageInner::Static(img) => img.clone(),
            ImageInner::Animated(seq) => seq.iter().next()
                .map(|f| f.image().clone())
                .ok_or_else(|| ImageError("Overlay source has no frames".into()))?,
        };

        match &mut self.inner {
            ImageInner::Static(base) => {
                apply_overlay(base, &overlay_img, x as i32, y as i32, opacity);
            }
            ImageInner::Animated(seq) => {
                for frame in seq.iter_mut() {
                    apply_overlay(frame.image_mut(), &overlay_img, x as i32, y as i32, opacity);
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
                        // Frame is entirely outside crop region; replace with 1x1 transparent.
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
/// ril 0.10's Image::crop() mutates in-place and returns (), so we use this manual fallback.
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

/// Alpha-composite `overlay` onto `base` at position (x, y) with opacity multiplier.
fn apply_overlay(base: &mut Image<Rgba>, overlay: &Image<Rgba>, x: i32, y: i32, opacity: f32) {
    let bw = base.width() as i32;
    let bh = base.height() as i32;
    let ow = overlay.width() as i32;
    let oh = overlay.height() as i32;

    for oy in 0..oh {
        for ox in 0..ow {
            let bx = x + ox;
            let by = y + oy;
            if bx >= 0 && bx < bw && by >= 0 && by < bh {
                let src = *overlay.pixel(ox as u32, oy as u32);
                let dst = *base.pixel(bx as u32, by as u32);

                let src_a = (src.a as f32 / 255.0) * opacity;
                let dst_a = dst.a as f32 / 255.0;
                let out_a = src_a + dst_a * (1.0 - src_a);

                let blended = if out_a == 0.0 {
                    Rgba { r: 0, g: 0, b: 0, a: 0 }
                } else {
                    let blend = |s: u8, d: u8| -> u8 {
                        ((s as f32 * src_a + d as f32 * dst_a * (1.0 - src_a)) / out_a) as u8
                    };
                    Rgba {
                        r: blend(src.r, dst.r),
                        g: blend(src.g, dst.g),
                        b: blend(src.b, dst.b),
                        a: (out_a * 255.0) as u8,
                    }
                };
                base.set_pixel(bx as u32, by as u32, blended);
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
