# ImageOps Trait Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor `PhpImage` into a thin PHP bridge by splitting image storage into `StaticImage(DynamicImage)` and `AnimatedImage(Vec<image::Frame>)`, both implementing an `ImageOps` trait.

**Architecture:** A new `ImageOps` trait defines the manipulation contract. `StaticImage` implements it with direct `DynamicImage` calls (single line per op). `AnimatedImage` implements it by iterating frames. `PhpImage` holds an `ImageInner` enum and delegates all logic via `match`.

**Tech Stack:** Rust, `image = "0.25"` crate (`DynamicImage`, `image::Frame`, `GifDecoder`, `GifEncoder`), `webp`, `ext-php-rs`

---

### Task 1: Refactor `image_ops.rs` — change overlay to operate on `RgbaImage` directly

Both `StaticImage` and `AnimatedImage` need to share the overlay pixel-blending logic. The current `overlay_frame` signature takes `&image::Frame` as the base, which doesn't work for `StaticImage`. Change it to take `&mut RgbaImage` (mutates in place) so both impls can use it.

**Files:**
- Modify: `src/image_ops.rs`

**Step 1: Rewrite `image_ops.rs`**

Replace the entire file:

```rust
use image::{DynamicImage, RgbaImage};

pub fn overlay_rgba(base: &mut RgbaImage, overlay_img: &DynamicImage, x: i32, y: i32, opacity: f32) {
    let overlay_rgba = overlay_img.to_rgba8();
    let ow = overlay_rgba.width() as i32;
    let oh = overlay_rgba.height() as i32;
    let bw = base.width() as i32;
    let bh = base.height() as i32;

    for oy in 0..oh {
        for ox in 0..ow {
            let bx = x + ox;
            let by = y + oy;
            if bx >= 0 && bx < bw && by >= 0 && by < bh {
                let src = overlay_rgba.get_pixel(ox as u32, oy as u32);
                let dst = base.get_pixel(bx as u32, by as u32);

                let src_a = (src.0[3] as f32 / 255.0) * opacity;
                let dst_a = dst.0[3] as f32 / 255.0;
                let out_a = src_a + dst_a * (1.0 - src_a);

                let blend = |s: u8, d: u8| -> u8 {
                    if out_a == 0.0 { return 0; }
                    ((s as f32 * src_a + d as f32 * dst_a * (1.0 - src_a)) / out_a) as u8
                };

                base.put_pixel(bx as u32, by as u32, image::Rgba([
                    blend(src.0[0], dst.0[0]),
                    blend(src.0[1], dst.0[1]),
                    blend(src.0[2], dst.0[2]),
                    (out_a * 255.0) as u8,
                ]));
            }
        }
    }
}
```

**Step 2: Verify it compiles**

```bash
cargo build 2>&1
```

Expected: compile error pointing at callers of the old `overlay_frame` in `src/image.rs`. That's expected — we'll fix `image.rs` in Task 6.

Acceptable at this stage: errors in `image.rs`. Not acceptable: errors inside `image_ops.rs` itself.

**Step 3: Commit**

```bash
git add src/image_ops.rs
git commit -m "refactor(image_ops): replace overlay_frame with overlay_rgba operating on &mut RgbaImage"
```

---

### Task 2: Refactor `image_encode.rs` — decouple WebP/AVIF encode from `image::Frame`

Currently `encode_webp` and `encode_avif` take `&image::Frame`. `StaticImage` doesn't have frames, so change both to accept raw RGBA bytes + dimensions. `encode_webp_animated` keeps taking `&[image::Frame]` since that's only called from `AnimatedImage`.

**Files:**
- Modify: `src/image_encode.rs`

**Step 1: Rewrite `image_encode.rs`**

Replace the entire file:

```rust
use crate::image_error::ImageError;

pub fn encode_webp(rgba: &[u8], width: u32, height: u32, quality: u8) -> Result<Vec<u8>, ImageError> {
    let encoder = webp::Encoder::from_rgba(rgba, width, height);
    let memory = encoder.encode(quality as f32);
    Ok(memory.to_vec())
}

pub fn encode_webp_animated(frames: &[image::Frame], quality: u8) -> Result<Vec<u8>, ImageError> {
    if frames.is_empty() {
        return Err(ImageError("No frames to encode".into()));
    }

    let width = frames[0].buffer().width();
    let height = frames[0].buffer().height();

    let mut config = webp::WebPConfig::new()
        .map_err(|_| ImageError("Failed to create WebP config".into()))?;
    config.quality = quality as f32;

    let mut encoder = webp::AnimEncoder::new(width, height, &config);
    encoder.set_loop_count(0);

    let mut timestamp_ms: i32 = 0;
    for frame in frames {
        let ts = timestamp_ms;
        let (numer, denom) = frame.delay().numer_denom_ms();
        let delay_ms = if denom == 0 { 0 } else { numer / denom };
        timestamp_ms += delay_ms as i32;
        let anim_frame = webp::AnimFrame::from_rgba(frame.buffer().as_raw(), width, height, ts);
        encoder.add_frame(anim_frame);
    }

    let data = encoder.encode();
    Ok(data.to_vec())
}

pub fn encode_avif(rgba: &[u8], width: u32, height: u32, quality: u8) -> Result<Vec<u8>, ImageError> {
    use image::codecs::avif::AvifEncoder;
    use image::ImageEncoder;

    let mut buf = Vec::new();
    let encoder = AvifEncoder::new_with_speed_quality(&mut buf, 6, quality);
    encoder
        .write_image(rgba, width, height, image::ExtendedColorType::Rgba8)
        .map_err(|e| ImageError(format!("AVIF encoding failed: {}", e)))?;
    Ok(buf)
}
```

**Step 2: Verify compile**

```bash
cargo build 2>&1
```

Acceptable: errors in `image.rs` (still references old encode signatures). Not acceptable: errors inside `image_encode.rs`.

**Step 3: Commit**

```bash
git add src/image_encode.rs
git commit -m "refactor(image_encode): encode_webp/encode_avif take raw RGBA bytes instead of &image::Frame"
```

---

### Task 3: Create `src/image_ops_trait.rs` — the `ImageOps` trait

**Files:**
- Create: `src/image_ops_trait.rs`

**Step 1: Write the trait**

```rust
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
```

**Step 2: Declare module in `lib.rs`**

Add to `src/lib.rs` after the existing `mod image_ops;` line:

```rust
mod image_ops_trait;
```

**Step 3: Verify compile**

```bash
cargo build 2>&1
```

Expected: clean (trait has no impl yet, just a definition).

**Step 4: Commit**

```bash
git add src/image_ops_trait.rs src/lib.rs
git commit -m "feat: add ImageOps trait defining the image manipulation contract"
```

---

### Task 4: Create `src/image_static.rs` — `StaticImage` + `ImageOps` impl

`StaticImage` wraps a single `DynamicImage`. Every operation is a direct method call — no frame wrapping.

**Files:**
- Create: `src/image_static.rs`

**Step 1: Write `image_static.rs`**

```rust
use image::{DynamicImage, imageops::FilterType};

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
                let mut encoder = GifEncoder::new(&mut buf);
                encoder.set_repeat(Repeat::Infinite)
                    .map_err(|e| ImageError(format!("Failed to set GIF repeat: {}", e)))?;
                encoder.encode_frames(std::iter::once(image::Frame::new(self.0.to_rgba8())))
                    .map_err(|e| ImageError(format!("GIF encoding failed: {}", e)))?;
                Ok(buf)
            }
        }
    }

    fn first_frame(&self) -> DynamicImage {
        self.0.clone()
    }
}
```

**Step 2: Declare module in `lib.rs`**

Add after `mod image_ops_trait;`:

```rust
mod image_static;
```

**Step 3: Verify compile**

```bash
cargo build 2>&1
```

Expected: clean. Acceptable: warnings about unused items. Not acceptable: errors inside `image_static.rs`.

**Step 4: Commit**

```bash
git add src/image_static.rs src/lib.rs
git commit -m "feat: add StaticImage wrapping DynamicImage with ImageOps impl"
```

---

### Task 5: Create `src/image_animated.rs` — `AnimatedImage` + `ImageOps` impl

`AnimatedImage` wraps `Vec<image::Frame>`. Every operation maps over frames.

**Files:**
- Create: `src/image_animated.rs`

**Step 1: Write `image_animated.rs`**

```rust
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
                let mut encoder = GifEncoder::new(&mut buf);
                encoder.set_repeat(Repeat::Infinite)
                    .map_err(|e| ImageError(format!("Failed to set GIF repeat: {}", e)))?;
                encoder.encode_frames(self.0.iter().cloned())
                    .map_err(|e| ImageError(format!("GIF encoding failed: {}", e)))?;
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
```

**Step 2: Declare module in `lib.rs`**

Add after `mod image_static;`:

```rust
mod image_animated;
```

**Step 3: Verify compile**

```bash
cargo build 2>&1
```

Expected: clean. Acceptable: errors still in `image.rs` (not yet refactored).

**Step 4: Commit**

```bash
git add src/image_animated.rs src/lib.rs
git commit -m "feat: add AnimatedImage wrapping Vec<Frame> with ImageOps impl"
```

---

### Task 6: Refactor `src/image.rs` — thin PHP bridge delegating to `ImageInner`

Replace the entire file. `PhpImage` no longer holds `frames: Vec<image::Frame>` — it holds `inner: ImageInner`. All manipulation methods validate inputs then delegate. No image logic lives here.

`open` uses `ImageReader::with_guessed_format()` to detect the file format, then dispatches any format whose decoder implements `AnimationDecoder` (currently `Gif` and `WebP`) to `AnimatedImage`. All other formats go to `StaticImage`. This removes the `is_gif` file-extension hack and generalises to any animated format the `image` crate gains in future.

`image_decode.rs` is no longer needed and is deleted (see Step 2).

**Files:**
- Modify: `src/image.rs`
- Delete: `src/image_decode.rs`
- Modify: `src/lib.rs` (remove `mod image_decode;`)

**Step 1: Rewrite `image.rs`**

```rust
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
        let format = reader.format()
            .map(|f| format!("{:?}", f).to_lowercase())
            .unwrap_or_else(|| "unknown".to_string());
        let (width, height) = reader.into_dimensions()
            .map_err(|e| ImageError(format!("Failed to read dimensions: {}", e)))?;

        // Check for animation by attempting AnimationDecoder on formats that support it.
        let is_animated = match reader.format() {
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

        Ok(ImageInfo { width, height, format, has_alpha: false, is_animated, exif_data: read_exif(&path) })
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
```

**Step 2: Delete `image_decode.rs` and remove its `mod` from `lib.rs`**

```bash
rm src/image_decode.rs
```

In `src/lib.rs`, remove the line:
```rust
mod image_decode;
```

**Step 3: Build**

```bash
cargo build 2>&1
```

Expected: clean compile.

**Step 4: Run tests**

```bash
php -d extension=target/debug/libphprs_hello_world.dylib test_image.php 2>&1
php -d extension=target/debug/libphprs_hello_world.dylib test_gif_resize.php 2>&1
```

Expected: `=== All tests passed! ===` for both scripts.

**Step 5: Commit**

```bash
git add src/image.rs src/lib.rs && git rm src/image_decode.rs
git commit -m "refactor(image): PhpImage delegates to ImageInner enum via ImageOps trait; dispatch AnimationDecoder by format"
```

---

### Task 7: Release build + benchmark

**Step 1: Release build**

```bash
cargo build --release 2>&1
```

Expected: clean.

**Step 2: Run full test suite and benchmark**

```bash
php -d extension=target/release/libphprs_hello_world.dylib test_image.php 2>&1
php -d extension=target/release/libphprs_hello_world.dylib test_gif_resize.php 2>&1
php -d extension=target/release/libphprs_hello_world.dylib bench_image.php 2>&1
```

Expected: all tests pass, benchmark runs to completion.

**Step 3: Commit**

No new code — if all green, move to finishing the branch.
