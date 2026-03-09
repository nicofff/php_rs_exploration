# RustImage Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a PHP image processing extension in Rust wrapping the `image` crate with SIMD-accelerated resize via `fast_image_resize`, exposing a fluent API as `RustImage\Image`.

**Architecture:** Multi-file Rust crate (`lib.rs`, `decode.rs`, `encode.rs`, `ops.rs`, `info.rs`). Three PHP classes: `Image` (fluent API), `ImageInfo` (metadata), `ImageException` (errors). All pixel data stored internally as `Vec<Frame>` where each frame is RGBA8. Format encoding is deferred until `save()` or `to_buffer()`.

**Tech Stack:** Rust (edition 2024), `ext-php-rs` (latest), `image` 0.25, `fast_image_resize` 5, `webp` 0.3, `ravif` 0.11, `kamadak-exif` 0.5, PHP 8.5

**Build/Test commands:**
- Build: `cargo build`
- Run PHP test: `php -d extension=target/debug/libphprs_hello_world.dylib test_image.php`
- No external services required (unlike Redis client)

**Reference:** Design doc at `docs/plans/2026-03-08-rust-image-design.md`

**Important note on this project:** This is a NEW extension being added alongside the existing Redis client extension in the same crate. The existing `src/lib.rs` with the Redis client must be preserved. The new image code goes in separate files (`src/image/` module) and gets registered in `lib.rs` alongside the Redis classes. However, ext-php-rs requires all `#[php_class]` and `#[php_impl]` types to be registered in the `#[php_module]` function in `lib.rs`, so we add our new classes there.

**Important note on ext-php-rs:** The `#[php_impl]` macro requires that ALL methods for a class be in a SINGLE `impl` block. You cannot split methods across multiple files using separate `impl` blocks — the macro generates PHP class registration code and needs to see all methods at once. This means Image methods must all be in one file. We'll use `src/image.rs` as the main file with helper modules for internal logic.

**Test fixtures:** Before starting, you'll need some test images. Create `tests/fixtures/` and generate them programmatically in Task 1 using a small PHP/Rust bootstrap, or download small sample images. The plan uses programmatically-generated test images where possible.

---

### Task 1: Scaffold — Cargo.toml, module structure, ImageException, empty Image class

**Files:**
- Modify: `Cargo.toml` — add image processing dependencies
- Create: `src/image.rs` — Image class with constructor stub
- Create: `src/image_info.rs` — ImageInfo class
- Create: `src/image_error.rs` — ImageException + error wrapper
- Create: `src/image_decode.rs` — decode helper functions (empty initially)
- Create: `src/image_encode.rs` — encode helper functions (empty initially)
- Create: `src/image_ops.rs` — operation helper functions (empty initially)
- Modify: `src/lib.rs` — register new modules and PHP classes
- Create: `test_image.php` — basic smoke test

**Step 1: Update `Cargo.toml`**

Add the image processing dependencies alongside existing ones:

```toml
[package]
name = "phprs_hello_world"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
ext-php-rs = "*"
redis = "1.0.4"

# Image processing
image = "0.25"
fast_image_resize = "5"
webp = "0.3"
ravif = "0.11"
kamadak-exif = "0.5"
gif = "0.13"

[profile.release]
strip = "debuginfo"
```

**Step 2: Create `src/image_error.rs`**

```rust
use ext_php_rs::{exception::PhpException, prelude::*, zend::ce};

#[php_class]
#[php(name = "RustImage\\ImageException")]
#[php(extends(ce = ce::exception, stub = "\\Exception"))]
#[derive(Default)]
pub struct ImageException;

pub struct ImageError(pub String);

impl From<String> for ImageError {
    fn from(msg: String) -> Self {
        Self(msg)
    }
}

impl From<&str> for ImageError {
    fn from(msg: &str) -> Self {
        Self(msg.to_string())
    }
}

impl From<image::ImageError> for ImageError {
    fn from(err: image::ImageError) -> Self {
        Self(err.to_string())
    }
}

impl From<std::io::Error> for ImageError {
    fn from(err: std::io::Error) -> Self {
        Self(err.to_string())
    }
}

impl From<ImageError> for PhpException {
    fn from(err: ImageError) -> Self {
        PhpException::from_class::<ImageException>(err.0)
    }
}
```

**Step 3: Create `src/image_info.rs`**

```rust
use ext_php_rs::prelude::*;
use std::collections::HashMap;

#[php_class]
#[php(name = "RustImage\\ImageInfo")]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub has_alpha: bool,
    pub is_animated: bool,
}

#[php_impl]
impl ImageInfo {
    #[getter]
    pub fn width(&self) -> i64 {
        self.width as i64
    }

    #[getter]
    pub fn height(&self) -> i64 {
        self.height as i64
    }

    #[getter]
    pub fn format(&self) -> String {
        self.format.clone()
    }

    #[getter]
    pub fn has_alpha(&self) -> bool {
        self.has_alpha
    }

    #[getter]
    pub fn is_animated(&self) -> bool {
        self.is_animated
    }
}
```

Note: EXIF support will be added in a later task. Start with the core fields.

**Step 4: Create `src/image_decode.rs`** (empty for now)

```rust
// Decode helper functions — filled in Task 2
```

**Step 5: Create `src/image_encode.rs`** (empty for now)

```rust
// Encode helper functions — filled in Task 4
```

**Step 6: Create `src/image_ops.rs`** (empty for now)

```rust
// Image operation helpers — filled in Task 3
```

**Step 7: Create `src/image.rs`**

```rust
use ext_php_rs::prelude::*;
use image::RgbaImage;

use crate::image_error::ImageError;
use crate::image_info::ImageInfo;

pub struct Frame {
    pub buffer: RgbaImage,
    pub delay_ms: u32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum OutputFormat {
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
            .map_err(|e| ImageError(format!("Failed to open '{}': {}", path, e)))?;
        let format = reader.format()
            .map(|f| format!("{:?}", f).to_lowercase())
            .unwrap_or_else(|| "unknown".to_string());
        let reader = reader.with_guessed_format()
            .map_err(|e| ImageError(format!("Failed to guess format: {}", e)))?;
        let (width, height) = reader.into_dimensions()
            .map_err(|e| ImageError(format!("Failed to read dimensions: {}", e)))?;

        Ok(ImageInfo {
            width,
            height,
            format,
            has_alpha: false, // TODO: detect properly
            is_animated: false, // TODO: detect for GIF/WebP
        })
    }
}
```

**Step 8: Update `src/lib.rs`**

Add the new modules and register the new classes in the module function. Keep all existing Redis code:

```rust
#![cfg_attr(windows, feature(abi_vectorcall))]

use ext_php_rs::{exception::PhpException, prelude::*, zend::ce};
use redis::{Commands, RedisError};
use std::collections::{HashMap, HashSet};

mod image;
mod image_decode;
mod image_encode;
mod image_error;
mod image_info;
mod image_ops;

// ── Error Handling ──────────────────────────────────────────────────
// ... (keep all existing Redis code exactly as-is) ...

// ── Module Registration ─────────────────────────────────────────────

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        // Redis
        .class::<PhpRedisException>()
        .class::<RedisClient>()
        // Image
        .class::<image_error::ImageException>()
        .class::<image_info::ImageInfo>()
        .class::<image::PhpImage>()
}
```

**Step 9: Build**

Run: `cargo build`
Expected: Compiles with no errors. There may be warnings about unused imports in the empty helper modules — that's fine.

**Step 10: Write initial test**

Create `test_image.php`:

```php
<?php

echo "=== RustImage Test Suite ===\n\n";

// Task 1: Scaffold — basic open and info
echo "--- Task 1: Scaffold ---\n";

// We need a test image. Create a simple one using GD (available in PHP).
$tmpJpeg = '/tmp/rustimage_test_photo.jpg';
$img = imagecreatetruecolor(200, 150);
$white = imagecolorallocate($img, 255, 255, 255);
imagefill($img, 0, 0, $white);
$red = imagecolorallocate($img, 255, 0, 0);
imagefilledrectangle($img, 50, 30, 150, 120, $red);
imagejpeg($img, $tmpJpeg, 90);
imagedestroy($img);
echo "Created test JPEG: $tmpJpeg\n";

// Test Image::open
$image = RustImage\Image::open($tmpJpeg);
echo "Image::open OK\n";

// Test Image::info
$info = RustImage\Image::info($tmpJpeg);
echo "Width: {$info->width}\n";   // 200
echo "Height: {$info->height}\n"; // 150
echo "Format: {$info->format}\n"; // jpeg
assert($info->width === 200, "Expected width 200");
assert($info->height === 150, "Expected height 150");

// Test error on missing file
try {
    RustImage\Image::open('/tmp/nonexistent_image.jpg');
    echo "FAIL: should have thrown\n";
} catch (RustImage\ImageException $e) {
    echo "Expected error: " . $e->getMessage() . "\n";
}

echo "\nTask 1 passed!\n";
```

**Step 11: Run test**

Run: `php -d extension=target/debug/libphprs_hello_world.dylib test_image.php`
Expected:
```
=== RustImage Test Suite ===

--- Task 1: Scaffold ---
Created test JPEG: /tmp/rustimage_test_photo.jpg
Image::open OK
Width: 200
Height: 150
Format: jpeg
Expected error: Failed to open image '/tmp/nonexistent_image.jpg': ...

Task 1 passed!
```

**Step 12: Commit**

```bash
git add Cargo.toml src/image.rs src/image_info.rs src/image_error.rs src/image_decode.rs src/image_encode.rs src/image_ops.rs src/lib.rs test_image.php
git commit -m "feat(image): scaffold RustImage extension with open, info, and exception"
```

---

### Task 2: Decode — static images (JPEG, PNG, BMP, TIFF) + fromBuffer

**Files:**
- Modify: `src/image_decode.rs` — add decode functions
- Modify: `src/image.rs` — add `from_buffer` method, use decode helpers
- Modify: `test_image.php` — add decode tests

**Step 1: Implement `src/image_decode.rs`**

```rust
use image::{DynamicImage, ImageReader, RgbaImage};
use std::io::Cursor;

use crate::image_error::ImageError;
use crate::image::Frame;

pub fn decode_static_from_path(path: &str) -> Result<Vec<Frame>, ImageError> {
    let img = image::open(path)
        .map_err(|e| ImageError(format!("Failed to decode image '{}': {}", path, e)))?;
    Ok(vec![Frame {
        buffer: img.to_rgba8(),
        delay_ms: 0,
    }])
}

pub fn decode_static_from_buffer(bytes: &[u8]) -> Result<Vec<Frame>, ImageError> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| ImageError(format!("Failed to decode image from buffer: {}", e)))?;
    Ok(vec![Frame {
        buffer: img.to_rgba8(),
        delay_ms: 0,
    }])
}

pub fn detect_has_alpha(img: &RgbaImage) -> bool {
    img.pixels().any(|p| p.0[3] < 255)
}
```

**Step 2: Update `src/image.rs`**

Add `from_buffer` method and resource limit support to `open`. Update the `open` method to use decode helpers and add `max_width`/`max_height`/`max_bytes` parameters:

```rust
use ext_php_rs::prelude::*;
use image::RgbaImage;

use crate::image_error::ImageError;
use crate::image_info::ImageInfo;
use crate::image_decode;

pub struct Frame {
    pub buffer: RgbaImage,
    pub delay_ms: u32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum OutputFormat {
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
    pub fn open(
        path: String,
        max_width: Option<i64>,
        max_height: Option<i64>,
        max_bytes: Option<i64>,
    ) -> Result<Self, ImageError> {
        // Check file size limit
        if let Some(max) = max_bytes {
            let metadata = std::fs::metadata(&path)
                .map_err(|e| ImageError(format!("Failed to open image '{}': {}", path, e)))?;
            if metadata.len() > max as u64 {
                return Err(ImageError(format!(
                    "File size {} bytes exceeds limit {} bytes",
                    metadata.len(), max
                )));
            }
        }

        let frames = image_decode::decode_static_from_path(&path)?;

        // Check dimension limits
        if let Some(frame) = frames.first() {
            let w = frame.buffer.width();
            let h = frame.buffer.height();
            let mw = max_width.map(|v| v as u32).unwrap_or(u32::MAX);
            let mh = max_height.map(|v| v as u32).unwrap_or(u32::MAX);
            if w > mw || h > mh {
                return Err(ImageError(format!(
                    "Image dimensions {}x{} exceed limit {}x{}",
                    w, h, mw, mh
                )));
            }
        }

        Ok(Self { frames, output_format: None })
    }

    pub fn from_buffer(bytes: Vec<u8>) -> Result<Self, ImageError> {
        let frames = image_decode::decode_static_from_buffer(&bytes)?;
        Ok(Self { frames, output_format: None })
    }

    pub fn info(path: String) -> Result<ImageInfo, ImageError> {
        let reader = image::ImageReader::open(&path)
            .map_err(|e| ImageError(format!("Failed to open '{}': {}", path, e)))?;
        let format = reader.format()
            .map(|f| format!("{:?}", f).to_lowercase())
            .unwrap_or_else(|| "unknown".to_string());
        let reader = reader.with_guessed_format()
            .map_err(|e| ImageError(format!("Failed to guess format: {}", e)))?;
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
```

**Step 3: Build**

Run: `cargo build`

**Step 4: Add tests to `test_image.php`**

Append:

```php
// Task 2: Decode + fromBuffer
echo "\n--- Task 2: Decode + fromBuffer ---\n";

// Create a PNG with transparency for testing
$tmpPng = '/tmp/rustimage_test_alpha.png';
$img = imagecreatetruecolor(100, 80);
imagesavealpha($img, true);
$transparent = imagecolorallocatealpha($img, 0, 0, 0, 127);
imagefill($img, 0, 0, $transparent);
$blue = imagecolorallocate($img, 0, 0, 255);
imagefilledellipse($img, 50, 40, 60, 40, $blue);
imagepng($img, $tmpPng);
imagedestroy($img);
echo "Created test PNG: $tmpPng\n";

// Open PNG
$image = RustImage\Image::open($tmpPng);
echo "PNG open OK\n";
$info = RustImage\Image::info($tmpPng);
assert($info->width === 100, "Expected PNG width 100");
assert($info->height === 80, "Expected PNG height 80");
echo "PNG info: {$info->width}x{$info->height} {$info->format}\n";

// fromBuffer
$bytes = file_get_contents($tmpJpeg);
$image = RustImage\Image::fromBuffer($bytes);
echo "fromBuffer OK\n";

// Resource limits — dimension check
try {
    RustImage\Image::open($tmpJpeg, maxWidth: 50, maxHeight: 50);
    echo "FAIL: should have thrown for dimension limit\n";
} catch (RustImage\ImageException $e) {
    echo "Dimension limit: " . $e->getMessage() . "\n";
}

// Resource limits — file size check
try {
    RustImage\Image::open($tmpJpeg, maxBytes: 10);
    echo "FAIL: should have thrown for size limit\n";
} catch (RustImage\ImageException $e) {
    echo "Size limit: " . $e->getMessage() . "\n";
}

// Corrupt file
$tmpCorrupt = '/tmp/rustimage_test_corrupt.jpg';
file_put_contents($tmpCorrupt, "not a real image");
try {
    RustImage\Image::open($tmpCorrupt);
    echo "FAIL: should have thrown for corrupt file\n";
} catch (RustImage\ImageException $e) {
    echo "Corrupt file: " . $e->getMessage() . "\n";
}

echo "\nTask 2 passed!\n";
```

**Step 5: Run test**

Run: `php -d extension=target/debug/libphprs_hello_world.dylib test_image.php`

**Step 6: Commit**

```bash
git add src/image.rs src/image_decode.rs test_image.php
git commit -m "feat(image): add static image decoding, fromBuffer, and resource limits"
```

---

### Task 3: Resize — SIMD-accelerated via fast_image_resize

This is the core "wow" feature. We use `fast_image_resize` for SIMD-accelerated Lanczos3 resizing.

**Files:**
- Modify: `src/image_ops.rs` — resize logic
- Modify: `src/image.rs` — add `resize` and `thumbnail` methods
- Modify: `test_image.php`

**Step 1: Implement `src/image_ops.rs`**

```rust
use fast_image_resize::{images::Image as FirImage, ResizeAlg, ResizeOptions, Resizer, PixelType};
use image::RgbaImage;

use crate::image_error::ImageError;
use crate::image::Frame;

pub fn resize_frame(frame: &Frame, new_width: u32, new_height: u32, algorithm: ResizeAlg) -> Result<Frame, ImageError> {
    let src_width = frame.buffer.width();
    let src_height = frame.buffer.height();

    if new_width == 0 || new_height == 0 {
        return Err(ImageError("Resize dimensions must be > 0".into()));
    }

    let src_image = FirImage::from_vec_u8(
        src_width,
        src_height,
        frame.buffer.as_raw().clone(),
        PixelType::U8x4,
    ).map_err(|e| ImageError(format!("Failed to create source image: {}", e)))?;

    let mut dst_image = FirImage::new(new_width, new_height, PixelType::U8x4);

    let mut resizer = Resizer::new();
    let options = ResizeOptions::new().resize_alg(algorithm);
    resizer.resize(&src_image, &mut dst_image, &options)
        .map_err(|e| ImageError(format!("Resize failed: {}", e)))?;

    let dst_buf = dst_image.into_vec();
    let rgba = RgbaImage::from_raw(new_width, new_height, dst_buf)
        .ok_or_else(|| ImageError("Failed to construct output image".into()))?;

    Ok(Frame {
        buffer: rgba,
        delay_ms: frame.delay_ms,
    })
}

/// Calculate dimensions that fit within max_w x max_h while preserving aspect ratio
pub fn fit_contain(src_w: u32, src_h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    let ratio_w = max_w as f64 / src_w as f64;
    let ratio_h = max_h as f64 / src_h as f64;
    let ratio = ratio_w.min(ratio_h);
    let new_w = (src_w as f64 * ratio).round() as u32;
    let new_h = (src_h as f64 * ratio).round() as u32;
    (new_w.max(1), new_h.max(1))
}

/// Calculate dimensions and crop to fill max_w x max_h exactly
pub fn fit_cover(src_w: u32, src_h: u32, target_w: u32, target_h: u32) -> (u32, u32, u32, u32) {
    let ratio_w = target_w as f64 / src_w as f64;
    let ratio_h = target_h as f64 / src_h as f64;
    let ratio = ratio_w.max(ratio_h);
    let scaled_w = (src_w as f64 * ratio).round() as u32;
    let scaled_h = (src_h as f64 * ratio).round() as u32;
    // Return: resize_w, resize_h, crop_offset_x, crop_offset_y
    let crop_x = (scaled_w - target_w) / 2;
    let crop_y = (scaled_h - target_h) / 2;
    (scaled_w, scaled_h, crop_x, crop_y)
}

pub fn crop_frame(frame: &Frame, x: u32, y: u32, width: u32, height: u32) -> Result<Frame, ImageError> {
    let src_w = frame.buffer.width();
    let src_h = frame.buffer.height();

    if x + width > src_w || y + height > src_h {
        return Err(ImageError(format!(
            "Crop region {}x{} at ({},{}) exceeds image bounds {}x{}",
            width, height, x, y, src_w, src_h
        )));
    }

    let cropped = image::imageops::crop_imm(&frame.buffer, x, y, width, height).to_image();
    Ok(Frame {
        buffer: cropped,
        delay_ms: frame.delay_ms,
    })
}

pub fn grayscale_frame(frame: &Frame) -> Frame {
    let gray = image::DynamicImage::ImageRgba8(frame.buffer.clone()).grayscale().to_rgba8();
    Frame {
        buffer: gray,
        delay_ms: frame.delay_ms,
    }
}
```

**Step 2: Add methods to `src/image.rs`**

Add these methods inside the `#[php_impl] impl PhpImage` block, after the existing methods:

```rust
    pub fn resize(&mut self, width: i64, height: i64, fit: Option<String>) -> Result<&mut Self, ImageError> {
        use crate::image_ops;
        use fast_image_resize::ResizeAlg;

        let fit_mode = fit.as_deref().unwrap_or("contain");

        self.frames = self.frames.iter().map(|frame| {
            let src_w = frame.buffer.width();
            let src_h = frame.buffer.height();
            let target_w = width as u32;
            let target_h = height as u32;

            match fit_mode {
                "fill" => {
                    image_ops::resize_frame(frame, target_w, target_h, ResizeAlg::Convolution(fast_image_resize::FilterType::Lanczos3))
                }
                "cover" => {
                    let (resize_w, resize_h, crop_x, crop_y) = image_ops::fit_cover(src_w, src_h, target_w, target_h);
                    let resized = image_ops::resize_frame(frame, resize_w, resize_h, ResizeAlg::Convolution(fast_image_resize::FilterType::Lanczos3))?;
                    image_ops::crop_frame(&resized, crop_x, crop_y, target_w, target_h)
                }
                _ => {
                    // "contain" (default)
                    let (new_w, new_h) = image_ops::fit_contain(src_w, src_h, target_w, target_h);
                    image_ops::resize_frame(frame, new_w, new_h, ResizeAlg::Convolution(fast_image_resize::FilterType::Lanczos3))
                }
            }
        }).collect::<Result<Vec<_>, _>>()?;

        Ok(self)
    }

    pub fn thumbnail(&mut self, width: i64, height: i64) -> Result<&mut Self, ImageError> {
        use crate::image_ops;
        use fast_image_resize::ResizeAlg;

        self.frames = self.frames.iter().map(|frame| {
            let (new_w, new_h) = image_ops::fit_contain(
                frame.buffer.width(), frame.buffer.height(),
                width as u32, height as u32,
            );
            image_ops::resize_frame(frame, new_w, new_h, ResizeAlg::Interpolation(fast_image_resize::FilterType::Bilinear))
        }).collect::<Result<Vec<_>, _>>()?;

        Ok(self)
    }
```

Note: `&mut Self` return for method chaining — verify ext-php-rs supports this. If not, return `()` and have PHP chain differently, or clone. The Redis client doesn't chain, so this needs testing. If `&mut Self` doesn't work with ext-php-rs, change all operation methods to return `Result<(), ImageError>` and adjust the PHP API to not chain (call methods sequentially on the same object).

**Step 3: Build**

Run: `cargo build`
If `&mut Self` return doesn't compile with ext-php-rs, change to returning `Result<(), ImageError>` instead.

**Step 4: Add JPEG save for testing**

We need `save` to verify resize output. Add a minimal save to `src/image.rs` (full encoding comes in Task 4):

```rust
    pub fn to_jpeg(&mut self, quality: Option<i64>) -> Result<&mut Self, ImageError> {
        self.output_format = Some(OutputFormat::Jpeg(quality.unwrap_or(85) as u8));
        Ok(self)
    }

    pub fn to_png(&mut self) -> Result<&mut Self, ImageError> {
        self.output_format = Some(OutputFormat::Png);
        Ok(self)
    }

    pub fn save(&self, path: String) -> Result<(), ImageError> {
        let frame = self.frames.first()
            .ok_or_else(|| ImageError("No image data to save".into()))?;

        let format = self.output_format.unwrap_or(OutputFormat::Png);
        match format {
            OutputFormat::Jpeg(quality) => {
                let rgb = image::DynamicImage::ImageRgba8(frame.buffer.clone()).to_rgb8();
                let file = std::fs::File::create(&path)
                    .map_err(|e| ImageError(format!("Failed to save to '{}': {}", path, e)))?;
                let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                    std::io::BufWriter::new(file), quality
                );
                encoder.encode_image(&rgb)
                    .map_err(|e| ImageError(format!("Failed to encode JPEG: {}", e)))?;
            }
            OutputFormat::Png => {
                frame.buffer.save(&path)
                    .map_err(|e| ImageError(format!("Failed to save PNG '{}': {}", path, e)))?;
            }
            _ => {
                return Err(ImageError("Format not yet implemented".into()));
            }
        }
        Ok(())
    }
```

**Step 5: Build**

Run: `cargo build`

**Step 6: Add tests**

```php
// Task 3: Resize
echo "\n--- Task 3: Resize ---\n";

// Test contain (default) — should preserve aspect ratio
$outContain = '/tmp/rustimage_test_contain.png';
$image = RustImage\Image::open($tmpJpeg);  // 200x150
$image->resize(100, 100);
$image->toPng();
$image->save($outContain);
$info = RustImage\Image::info($outContain);
echo "Contain 200x150 into 100x100: {$info->width}x{$info->height}\n";
// Should be 100x75 (aspect preserved, fit within)
assert($info->width === 100, "Contain width should be 100, got {$info->width}");
assert($info->height === 75, "Contain height should be 75, got {$info->height}");

// Test fill — exact dimensions, stretches
$outFill = '/tmp/rustimage_test_fill.png';
$image = RustImage\Image::open($tmpJpeg);
$image->resize(100, 100, fit: 'fill');
$image->toPng();
$image->save($outFill);
$info = RustImage\Image::info($outFill);
echo "Fill 200x150 into 100x100: {$info->width}x{$info->height}\n";
assert($info->width === 100 && $info->height === 100, "Fill should be exactly 100x100");

// Test cover — fill area, crop excess
$outCover = '/tmp/rustimage_test_cover.png';
$image = RustImage\Image::open($tmpJpeg);
$image->resize(100, 100, fit: 'cover');
$image->toPng();
$image->save($outCover);
$info = RustImage\Image::info($outCover);
echo "Cover 200x150 into 100x100: {$info->width}x{$info->height}\n";
assert($info->width === 100 && $info->height === 100, "Cover should be exactly 100x100");

// Test thumbnail (bilinear, fast)
$outThumb = '/tmp/rustimage_test_thumb.png';
$image = RustImage\Image::open($tmpJpeg);
$image->thumbnail(50, 50);
$image->toPng();
$image->save($outThumb);
$info = RustImage\Image::info($outThumb);
echo "Thumbnail 200x150 into 50x50: {$info->width}x{$info->height}\n";
assert($info->width === 50, "Thumb width should be 50");
// Height should be 37 or 38 (aspect preserved)

// Test JPEG output with quality
$outJpeg = '/tmp/rustimage_test_resized.jpg';
$image = RustImage\Image::open($tmpJpeg);
$image->resize(100, 100);
$image->toJpeg(quality: 70);
$image->save($outJpeg);
$info = RustImage\Image::info($outJpeg);
echo "JPEG resize: {$info->width}x{$info->height} {$info->format}\n";
assert($info->format === 'jpeg', "Should be JPEG format");

echo "\nTask 3 passed!\n";
```

**Step 7: Run test**

Run: `php -d extension=target/debug/libphprs_hello_world.dylib test_image.php`

**Step 8: Commit**

```bash
git add src/image.rs src/image_ops.rs test_image.php
git commit -m "feat(image): add SIMD-accelerated resize with contain/cover/fill modes"
```

---

### Task 4: Crop + Grayscale operations

**Files:**
- Modify: `src/image.rs` — add `crop` and `grayscale` methods
- Modify: `test_image.php`

**Step 1: Add methods to `src/image.rs`**

Add inside the `#[php_impl]` block:

```rust
    pub fn crop(&mut self, x: i64, y: i64, width: i64, height: i64) -> Result<&mut Self, ImageError> {
        use crate::image_ops;

        self.frames = self.frames.iter().map(|frame| {
            image_ops::crop_frame(frame, x as u32, y as u32, width as u32, height as u32)
        }).collect::<Result<Vec<_>, _>>()?;

        Ok(self)
    }

    pub fn grayscale(&mut self) -> Result<&mut Self, ImageError> {
        use crate::image_ops;

        self.frames = self.frames.iter().map(|frame| {
            Ok(image_ops::grayscale_frame(frame))
        }).collect::<Result<Vec<_>, _>>()?;

        Ok(self)
    }
```

**Step 2: Build**

Run: `cargo build`

**Step 3: Add tests**

```php
// Task 4: Crop + Grayscale
echo "\n--- Task 4: Crop + Grayscale ---\n";

// Crop
$outCrop = '/tmp/rustimage_test_crop.png';
$image = RustImage\Image::open($tmpJpeg);  // 200x150
$image->crop(10, 10, 100, 80);
$image->toPng();
$image->save($outCrop);
$info = RustImage\Image::info($outCrop);
echo "Crop: {$info->width}x{$info->height}\n";
assert($info->width === 100 && $info->height === 80, "Crop should be 100x80");

// Crop out of bounds
try {
    $image = RustImage\Image::open($tmpJpeg);
    $image->crop(0, 0, 500, 500);
    echo "FAIL: should have thrown for out-of-bounds crop\n";
} catch (RustImage\ImageException $e) {
    echo "Crop bounds error: " . $e->getMessage() . "\n";
}

// Grayscale
$outGray = '/tmp/rustimage_test_gray.png';
$image = RustImage\Image::open($tmpJpeg);
$image->grayscale();
$image->toPng();
$image->save($outGray);
$info = RustImage\Image::info($outGray);
echo "Grayscale: {$info->width}x{$info->height}\n";
assert($info->width === 200 && $info->height === 150, "Grayscale shouldn't change dimensions");

// Chain: crop then resize
$outChain = '/tmp/rustimage_test_chain.png';
$image = RustImage\Image::open($tmpJpeg);
$image->crop(10, 10, 180, 130);
$image->resize(90, 65);
$image->toPng();
$image->save($outChain);
$info = RustImage\Image::info($outChain);
echo "Chain crop+resize: {$info->width}x{$info->height}\n";

echo "\nTask 4 passed!\n";
```

**Step 4: Run test, Step 5: Commit**

```bash
git add src/image.rs test_image.php
git commit -m "feat(image): add crop and grayscale operations"
```

---

### Task 5: WebP encoding/decoding

**Files:**
- Modify: `src/image_decode.rs` — WebP decode
- Modify: `src/image_encode.rs` — WebP encode helper
- Modify: `src/image.rs` — add `to_webp` method, update `save`
- Modify: `test_image.php`

**Step 1: Add WebP encode to `src/image_encode.rs`**

```rust
use image::RgbaImage;
use crate::image_error::ImageError;
use crate::image::Frame;

pub fn encode_webp(frame: &Frame, quality: u8) -> Result<Vec<u8>, ImageError> {
    let encoder = webp::Encoder::from_rgba(
        frame.buffer.as_raw(),
        frame.buffer.width(),
        frame.buffer.height(),
    );
    let memory = encoder.encode(quality as f32);
    Ok(memory.to_vec())
}
```

**Step 2: Update `save` in `src/image.rs`**

Add the WebP case to the `save` method's match:

```rust
            OutputFormat::Webp(quality) => {
                let data = crate::image_encode::encode_webp(frame, quality)?;
                std::fs::write(&path, &data)
                    .map_err(|e| ImageError(format!("Failed to save WebP '{}': {}", path, e)))?;
            }
```

And add the `to_webp` method:

```rust
    pub fn to_webp(&mut self, quality: Option<i64>) -> Result<&mut Self, ImageError> {
        self.output_format = Some(OutputFormat::Webp(quality.unwrap_or(80) as u8));
        Ok(self)
    }
```

**Step 3: Build**

Run: `cargo build`

**Step 4: Add tests**

```php
// Task 5: WebP encode/decode
echo "\n--- Task 5: WebP ---\n";

// JPEG to WebP conversion
$outWebp = '/tmp/rustimage_test.webp';
$image = RustImage\Image::open($tmpJpeg);
$image->resize(100, 100);
$image->toWebp(quality: 80);
$image->save($outWebp);
echo "Saved WebP: " . filesize($outWebp) . " bytes\n";
$info = RustImage\Image::info($outWebp);
echo "WebP info: {$info->width}x{$info->height} {$info->format}\n";

// Re-open WebP and resize again
$outWebp2 = '/tmp/rustimage_test2.webp';
$image = RustImage\Image::open($outWebp);
$image->resize(50, 50);
$image->toWebp(quality: 60);
$image->save($outWebp2);
$info = RustImage\Image::info($outWebp2);
echo "Re-encoded WebP: {$info->width}x{$info->height}\n";

echo "\nTask 5 passed!\n";
```

**Step 5: Run test, Step 6: Commit**

```bash
git add src/image.rs src/image_encode.rs test_image.php
git commit -m "feat(image): add WebP encoding and decoding"
```

---

### Task 6: AVIF encoding

**Files:**
- Modify: `src/image_encode.rs` — AVIF encode helper
- Modify: `src/image.rs` — add `to_avif` method, update `save`
- Modify: `test_image.php`

**Step 1: Add AVIF encode to `src/image_encode.rs`**

```rust
pub fn encode_avif(frame: &Frame, quality: u8) -> Result<Vec<u8>, ImageError> {
    let rgba_data = frame.buffer.as_raw();
    let width = frame.buffer.width() as usize;
    let height = frame.buffer.height() as usize;

    let img = ravif::Img::new(
        ravif::RGBA8::from_slice(rgba_data),
        width,
        height,
    );

    let config = ravif::EncoderConfig::default()
        .with_quality(quality as f32)
        .with_speed(6); // Balance speed/quality

    let result = ravif::encode_rgba(img, &config)
        .map_err(|e| ImageError(format!("AVIF encoding failed: {}", e)))?;

    Ok(result.avif_file)
}
```

Note: The `ravif` API may differ slightly depending on the exact version. Check the docs and adjust. The key is: take RGBA pixels, produce AVIF bytes.

**Step 2: Add `to_avif` method and update `save` in `src/image.rs`**

```rust
    pub fn to_avif(&mut self, quality: Option<i64>) -> Result<&mut Self, ImageError> {
        self.output_format = Some(OutputFormat::Avif(quality.unwrap_or(60) as u8));
        Ok(self)
    }
```

Add to `save` match:

```rust
            OutputFormat::Avif(quality) => {
                let data = crate::image_encode::encode_avif(frame, quality)?;
                std::fs::write(&path, &data)
                    .map_err(|e| ImageError(format!("Failed to save AVIF '{}': {}", path, e)))?;
            }
```

**Step 3: Build**

Run: `cargo build`
Note: AVIF encoding via `ravif` can be slow in debug builds. If testing is painful, use `cargo build --release` for this task.

**Step 4: Add tests**

```php
// Task 6: AVIF
echo "\n--- Task 6: AVIF ---\n";

$outAvif = '/tmp/rustimage_test.avif';
$image = RustImage\Image::open($tmpJpeg);
$image->resize(100, 100);
$image->toAvif(quality: 60);
$image->save($outAvif);
echo "Saved AVIF: " . filesize($outAvif) . " bytes\n";

// Verify we can read it back
$info = RustImage\Image::info($outAvif);
echo "AVIF info: {$info->width}x{$info->height} {$info->format}\n";

echo "\nTask 6 passed!\n";
```

**Step 5: Run test, Step 6: Commit**

```bash
git add src/image.rs src/image_encode.rs test_image.php
git commit -m "feat(image): add AVIF encoding support"
```

---

### Task 7: toBuffer output + animated GIF decode/encode

**Files:**
- Modify: `src/image_decode.rs` — animated GIF frame extraction
- Modify: `src/image_encode.rs` — GIF encode, animated WebP encode
- Modify: `src/image.rs` — add `to_buffer`, `to_gif`, update `save` for GIF, update `info` for animated detection
- Modify: `test_image.php`

**Step 1: Add animated GIF decoding to `src/image_decode.rs`**

```rust
use gif::DecodeOptions;

pub fn decode_gif_frames(path: &str) -> Result<Vec<Frame>, ImageError> {
    let file = std::fs::File::open(path)
        .map_err(|e| ImageError(format!("Failed to open GIF '{}': {}", path, e)))?;
    let mut decoder = DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);
    let mut reader = decoder.read_info(std::io::BufReader::new(file))
        .map_err(|e| ImageError(format!("Failed to decode GIF '{}': {}", path, e)))?;

    let width = reader.width() as u32;
    let height = reader.height() as u32;

    let mut frames = Vec::new();
    while let Some(frame) = reader.read_next_frame()
        .map_err(|e| ImageError(format!("Failed to read GIF frame: {}", e)))? {

        let delay_ms = (frame.delay as u32) * 10; // GIF delay is in centiseconds

        // Frame may be smaller than canvas — composite onto full-size buffer
        let mut canvas = vec![0u8; (width * height * 4) as usize];
        let frame_x = frame.left as u32;
        let frame_y = frame.top as u32;
        let frame_w = frame.width as u32;
        let frame_h = frame.height as u32;

        for y in 0..frame_h {
            for x in 0..frame_w {
                let src_idx = ((y * frame_w + x) * 4) as usize;
                let dst_x = frame_x + x;
                let dst_y = frame_y + y;
                if dst_x < width && dst_y < height {
                    let dst_idx = ((dst_y * width + dst_x) * 4) as usize;
                    canvas[dst_idx..dst_idx + 4].copy_from_slice(&frame.buffer[src_idx..src_idx + 4]);
                }
            }
        }

        let rgba = RgbaImage::from_raw(width, height, canvas)
            .ok_or_else(|| ImageError("Failed to construct GIF frame".into()))?;

        frames.push(Frame {
            buffer: rgba,
            delay_ms,
        });
    }

    if frames.is_empty() {
        return Err(ImageError("GIF contains no frames".into()));
    }

    Ok(frames)
}

pub fn is_gif(path: &str) -> bool {
    path.ends_with(".gif")
}
```

Note: This is a simplified GIF decoder that doesn't handle disposal methods properly. For a v1 this is acceptable — proper disposal can be added later.

**Step 2: Update `open` in `src/image.rs` to detect GIF**

Replace the body of `open` to detect GIF and use animated decoder:

```rust
    pub fn open(
        path: String,
        max_width: Option<i64>,
        max_height: Option<i64>,
        max_bytes: Option<i64>,
    ) -> Result<Self, ImageError> {
        if let Some(max) = max_bytes {
            let metadata = std::fs::metadata(&path)
                .map_err(|e| ImageError(format!("Failed to open image '{}': {}", path, e)))?;
            if metadata.len() > max as u64 {
                return Err(ImageError(format!(
                    "File size {} bytes exceeds limit {} bytes",
                    metadata.len(), max
                )));
            }
        }

        let frames = if image_decode::is_gif(&path) {
            image_decode::decode_gif_frames(&path)?
        } else {
            image_decode::decode_static_from_path(&path)?
        };

        if let Some(frame) = frames.first() {
            let w = frame.buffer.width();
            let h = frame.buffer.height();
            let mw = max_width.map(|v| v as u32).unwrap_or(u32::MAX);
            let mh = max_height.map(|v| v as u32).unwrap_or(u32::MAX);
            if w > mw || h > mh {
                return Err(ImageError(format!(
                    "Image dimensions {}x{} exceed limit {}x{}",
                    w, h, mw, mh
                )));
            }
        }

        Ok(Self { frames, output_format: None })
    }
```

**Step 3: Add `to_buffer` and `to_gif` to `src/image.rs`**

```rust
    pub fn to_buffer(&self) -> Result<Vec<u8>, ImageError> {
        let frame = self.frames.first()
            .ok_or_else(|| ImageError("No image data".into()))?;

        let format = self.output_format.unwrap_or(OutputFormat::Png);
        match format {
            OutputFormat::Jpeg(quality) => {
                let rgb = image::DynamicImage::ImageRgba8(frame.buffer.clone()).to_rgb8();
                let mut buf = Vec::new();
                let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                    std::io::Cursor::new(&mut buf), quality
                );
                encoder.encode_image(&rgb)
                    .map_err(|e| ImageError(format!("Failed to encode JPEG: {}", e)))?;
                Ok(buf)
            }
            OutputFormat::Png => {
                let mut buf = Vec::new();
                let encoder = image::codecs::png::PngEncoder::new(std::io::Cursor::new(&mut buf));
                encoder.write_image(
                    frame.buffer.as_raw(),
                    frame.buffer.width(),
                    frame.buffer.height(),
                    image::ExtendedColorType::Rgba8,
                ).map_err(|e| ImageError(format!("Failed to encode PNG: {}", e)))?;
                Ok(buf)
            }
            OutputFormat::Webp(quality) => {
                crate::image_encode::encode_webp(frame, quality)
            }
            OutputFormat::Avif(quality) => {
                crate::image_encode::encode_avif(frame, quality)
            }
            OutputFormat::Gif => {
                // For single frame, save as static GIF
                let mut buf = Vec::new();
                let encoder = image::codecs::gif::GifEncoder::new(&mut buf);
                let dyn_img = image::DynamicImage::ImageRgba8(frame.buffer.clone());
                encoder.encode(
                    dyn_img.to_rgba8().as_raw(),
                    frame.buffer.width(),
                    frame.buffer.height(),
                    image::ExtendedColorType::Rgba8,
                ).map_err(|e| ImageError(format!("Failed to encode GIF: {}", e)))?;
                Ok(buf)
            }
        }
    }

    pub fn to_gif(&mut self) -> Result<&mut Self, ImageError> {
        self.output_format = Some(OutputFormat::Gif);
        Ok(self)
    }
```

Also update `save` to handle GIF and add a `to_buffer`-based fallback for formats:

Update the `save` match to add:

```rust
            OutputFormat::Gif => {
                // Use the image crate's GIF encoder for static
                let dyn_img = image::DynamicImage::ImageRgba8(frame.buffer.clone());
                dyn_img.save(&path)
                    .map_err(|e| ImageError(format!("Failed to save GIF '{}': {}", path, e)))?;
            }
```

**Step 4: Update `info` to detect animated GIFs**

In the `info` method, add animated detection:

```rust
    pub fn info(path: String) -> Result<ImageInfo, ImageError> {
        let reader = image::ImageReader::open(&path)
            .map_err(|e| ImageError(format!("Failed to open '{}': {}", path, e)))?;
        let format = reader.format()
            .map(|f| format!("{:?}", f).to_lowercase())
            .unwrap_or_else(|| "unknown".to_string());

        let is_animated = if image_decode::is_gif(&path) {
            // Check if GIF has multiple frames
            if let Ok(frames) = image_decode::decode_gif_frames(&path) {
                frames.len() > 1
            } else {
                false
            }
        } else {
            false
        };

        let reader = image::ImageReader::open(&path)
            .map_err(|e| ImageError(format!("Failed to open '{}': {}", path, e)))?
            .with_guessed_format()
            .map_err(|e| ImageError(format!("Failed to guess format: {}", e)))?;
        let (width, height) = reader.into_dimensions()
            .map_err(|e| ImageError(format!("Failed to read dimensions: {}", e)))?;

        Ok(ImageInfo {
            width,
            height,
            format,
            has_alpha: false, // TODO
            is_animated,
        })
    }
```

**Step 5: Build**

Run: `cargo build`

**Step 6: Add tests**

```php
// Task 7: toBuffer + animated GIF
echo "\n--- Task 7: toBuffer + Animated GIF ---\n";

// toBuffer test
$image = RustImage\Image::open($tmpJpeg);
$image->resize(50, 50);
$image->toJpeg(quality: 80);
$buf = $image->toBuffer();
echo "toBuffer: " . strlen($buf) . " bytes\n";
assert(strlen($buf) > 0, "Buffer should not be empty");

// Verify buffer can be re-opened
$image2 = RustImage\Image::fromBuffer($buf);
echo "fromBuffer(toBuffer) OK\n";

// Create animated GIF using GD
$tmpGif = '/tmp/rustimage_test_anim.gif';
// GD can't create animated GIFs natively, so create a simple static GIF
$gifImg = imagecreatetruecolor(80, 60);
$green = imagecolorallocate($gifImg, 0, 255, 0);
imagefill($gifImg, 0, 0, $green);
imagegif($gifImg, $tmpGif);
imagedestroy($gifImg);
echo "Created test GIF: $tmpGif\n";

// Open and resize GIF
$image = RustImage\Image::open($tmpGif);
$image->resize(40, 30);
$outGif = '/tmp/rustimage_test_resized.gif';
$image->toGif();
$image->save($outGif);
$info = RustImage\Image::info($outGif);
echo "Resized GIF: {$info->width}x{$info->height} {$info->format}\n";

echo "\nTask 7 passed!\n";
```

**Step 7: Run test, Step 8: Commit**

```bash
git add src/image.rs src/image_decode.rs src/image_encode.rs test_image.php
git commit -m "feat(image): add toBuffer, animated GIF decode, and GIF output"
```

---

### Task 8: Overlay / watermark composition

**Files:**
- Modify: `src/image_ops.rs` — overlay logic
- Modify: `src/image.rs` — add `overlay` method
- Modify: `test_image.php`

**Step 1: Add overlay to `src/image_ops.rs`**

```rust
pub fn overlay_frame(base: &Frame, overlay: &Frame, x: i32, y: i32, opacity: f32) -> Frame {
    let mut result = base.buffer.clone();
    let ow = overlay.buffer.width() as i32;
    let oh = overlay.buffer.height() as i32;
    let bw = base.buffer.width() as i32;
    let bh = base.buffer.height() as i32;

    for oy in 0..oh {
        for ox in 0..ow {
            let bx = x + ox;
            let by = y + oy;
            if bx >= 0 && bx < bw && by >= 0 && by < bh {
                let src = overlay.buffer.get_pixel(ox as u32, oy as u32);
                let dst = result.get_pixel(bx as u32, by as u32);

                let src_a = (src.0[3] as f32 / 255.0) * opacity;
                let dst_a = dst.0[3] as f32 / 255.0;
                let out_a = src_a + dst_a * (1.0 - src_a);

                let blend = |s: u8, d: u8| -> u8 {
                    if out_a == 0.0 { return 0; }
                    ((s as f32 * src_a + d as f32 * dst_a * (1.0 - src_a)) / out_a) as u8
                };

                result.put_pixel(bx as u32, by as u32, image::Rgba([
                    blend(src.0[0], dst.0[0]),
                    blend(src.0[1], dst.0[1]),
                    blend(src.0[2], dst.0[2]),
                    (out_a * 255.0) as u8,
                ]));
            }
        }
    }

    Frame {
        buffer: result,
        delay_ms: base.delay_ms,
    }
}
```

**Step 2: Add overlay method to `src/image.rs`**

```rust
    pub fn overlay(&mut self, other: &PhpImage, x: i64, y: i64, opacity: Option<f64>) -> Result<&mut Self, ImageError> {
        use crate::image_ops;

        let overlay_frame = other.frames.first()
            .ok_or_else(|| ImageError("Overlay image has no data".into()))?;
        let opacity = opacity.unwrap_or(1.0) as f32;

        self.frames = self.frames.iter().map(|frame| {
            Ok(image_ops::overlay_frame(frame, overlay_frame, x as i32, y as i32, opacity))
        }).collect::<Result<Vec<_>, _>>()?;

        Ok(self)
    }
```

Note: The `other: &PhpImage` parameter — verify ext-php-rs supports passing PHP objects by reference. If not, you may need to pass the overlay as a file path instead and open it internally. Check ext-php-rs docs for accepting `&T` where T is a `#[php_class]`.

**Step 3: Build**

Run: `cargo build`

**Step 4: Add tests**

```php
// Task 8: Overlay
echo "\n--- Task 8: Overlay ---\n";

// Create a small "watermark" PNG
$tmpWatermark = '/tmp/rustimage_test_watermark.png';
$wm = imagecreatetruecolor(40, 20);
imagesavealpha($wm, true);
$wmBg = imagecolorallocatealpha($wm, 0, 0, 0, 64);
imagefill($wm, 0, 0, $wmBg);
$wmText = imagecolorallocate($wm, 255, 255, 255);
imagestring($wm, 3, 2, 2, "TEST", $wmText);
imagepng($wm, $tmpWatermark);
imagedestroy($wm);
echo "Created watermark: $tmpWatermark\n";

// Overlay watermark on photo
$outOverlay = '/tmp/rustimage_test_overlay.png';
$base = RustImage\Image::open($tmpJpeg);
$watermark = RustImage\Image::open($tmpWatermark);
$base->overlay($watermark, x: 10, y: 10, opacity: 0.7);
$base->toPng();
$base->save($outOverlay);
$info = RustImage\Image::info($outOverlay);
echo "Overlay result: {$info->width}x{$info->height}\n";
assert($info->width === 200 && $info->height === 150, "Overlay shouldn't change dimensions");

// Overlay at negative offset (partially off-screen)
$outPartial = '/tmp/rustimage_test_overlay_partial.png';
$base = RustImage\Image::open($tmpJpeg);
$watermark = RustImage\Image::open($tmpWatermark);
$base->overlay($watermark, x: -20, y: -10);
$base->toPng();
$base->save($outPartial);
echo "Partial overlay OK\n";

echo "\nTask 8 passed!\n";
```

**Step 5: Run test, Step 6: Commit**

```bash
git add src/image.rs src/image_ops.rs test_image.php
git commit -m "feat(image): add overlay/watermark composition with alpha blending"
```

---

### Task 9: EXIF metadata reading

**Files:**
- Modify: `src/image_info.rs` — add EXIF field
- Modify: `src/image.rs` — update `info` to read EXIF
- Modify: `test_image.php`

**Step 1: Update `src/image_info.rs`**

Add an `exif` getter. We'll store EXIF as a `HashMap<String, String>` internally:

```rust
use ext_php_rs::prelude::*;
use std::collections::HashMap;

#[php_class]
#[php(name = "RustImage\\ImageInfo")]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub has_alpha: bool,
    pub is_animated: bool,
    pub exif_data: Option<HashMap<String, String>>,
}

#[php_impl]
impl ImageInfo {
    #[getter]
    pub fn width(&self) -> i64 {
        self.width as i64
    }

    #[getter]
    pub fn height(&self) -> i64 {
        self.height as i64
    }

    #[getter]
    pub fn format(&self) -> String {
        self.format.clone()
    }

    #[getter]
    pub fn has_alpha(&self) -> bool {
        self.has_alpha
    }

    #[getter]
    pub fn is_animated(&self) -> bool {
        self.is_animated
    }

    #[getter]
    pub fn exif(&self) -> Option<HashMap<String, String>> {
        self.exif_data.clone()
    }
}
```

**Step 2: Add EXIF reading to `src/image.rs`**

In the `info` method, add EXIF extraction using `kamadak-exif`:

```rust
    // Add this function (outside the impl block, or as a helper in image_decode.rs)
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
```

Add `use std::collections::HashMap;` and `use exif;` (the `kamadak-exif` crate re-exports as `exif`) at the top of `image.rs`.

Update `info` to call this and populate `exif_data`:

```rust
        let exif_data = read_exif(&path);
```

And pass it to the `ImageInfo` constructor.

Also update all previous `ImageInfo` constructions to include `exif_data: None` where EXIF isn't read.

**Step 3: Build**

Run: `cargo build`
Note: The `kamadak-exif` crate imports as `exif` in Rust. Make sure the import is `use exif;` not `use kamadak_exif;`. Check `Cargo.lock` for the actual crate name. You may need to add `exif = { package = "kamadak-exif", version = "0.5" }` to Cargo.toml if it doesn't resolve automatically.

**Step 4: Add tests**

```php
// Task 9: EXIF
echo "\n--- Task 9: EXIF ---\n";

// JPEG files from cameras have EXIF, our generated ones don't
$info = RustImage\Image::info($tmpJpeg);
echo "EXIF on generated JPEG: " . var_export($info->exif, true) . "\n"; // null (no EXIF in generated images)

// Test that exif field is accessible and returns null for images without EXIF
assert($info->exif === null, "Generated JPEG should have no EXIF");
echo "EXIF null check OK\n";

echo "\nTask 9 passed!\n";
```

Note: Testing with real EXIF data would require a camera photo. For now, we verify the field is accessible and returns null for generated images. A more thorough test can be added later with a real photo fixture.

**Step 5: Run test, Step 6: Commit**

```bash
git add src/image.rs src/image_info.rs test_image.php Cargo.toml
git commit -m "feat(image): add EXIF metadata reading via kamadak-exif"
```

---

### Task 10: Benchmark + cleanup

**Files:**
- Create: `bench_image.php` — performance benchmark
- Modify: `test_image.php` — add cleanup and summary
- No Rust changes

**Step 1: Add cleanup to `test_image.php`**

Append:

```php
// --- Cleanup ---
echo "\n=== Cleanup ===\n";
$tmpFiles = glob('/tmp/rustimage_test_*');
foreach ($tmpFiles as $f) {
    unlink($f);
}
echo "Cleaned up " . count($tmpFiles) . " temp files\n";
echo "\n=== All tests passed! ===\n";
```

**Step 2: Create `bench_image.php`**

```php
<?php

echo "=== RustImage Benchmark ===\n";
echo "Comparing: RustImage vs GD" . (extension_loaded('imagick') ? " vs Imagick" : "") . "\n\n";

// Generate test image
$source = '/tmp/bench_source.jpg';
$img = imagecreatetruecolor(2000, 1500);
for ($i = 0; $i < 100; $i++) {
    $color = imagecolorallocate($img, rand(0, 255), rand(0, 255), rand(0, 255));
    imagefilledrectangle($img, rand(0, 1900), rand(0, 1400), rand(0, 1900), rand(0, 1400), $color);
}
imagejpeg($img, $source, 90);
imagedestroy($img);
echo "Source image: 2000x1500 JPEG (" . round(filesize($source) / 1024) . " KB)\n\n";

$iterations = 100;

// --- Benchmark 1: Resize to thumbnail ---
echo "--- Thumbnail generation ({$iterations}x) ---\n";

// RustImage
$start = hrtime(true);
for ($i = 0; $i < $iterations; $i++) {
    $image = RustImage\Image::open($source);
    $image->thumbnail(200, 200);
    $image->toJpeg(quality: 80);
    $image->save("/tmp/bench_rust_{$i}.jpg");
}
$rustTime = (hrtime(true) - $start) / 1e6;
echo sprintf("  RustImage: %7.0f ms\n", $rustTime);

// GD
$start = hrtime(true);
for ($i = 0; $i < $iterations; $i++) {
    $src = imagecreatefromjpeg($source);
    $thumb = imagescale($src, 200, 200);
    imagejpeg($thumb, "/tmp/bench_gd_{$i}.jpg", 80);
    imagedestroy($src);
    imagedestroy($thumb);
}
$gdTime = (hrtime(true) - $start) / 1e6;
echo sprintf("  GD:        %7.0f ms\n", $gdTime);

// Imagick
if (extension_loaded('imagick')) {
    $start = hrtime(true);
    for ($i = 0; $i < $iterations; $i++) {
        $im = new Imagick($source);
        $im->thumbnailImage(200, 200, true);
        $im->setImageCompressionQuality(80);
        $im->writeImage("/tmp/bench_imagick_{$i}.jpg");
        $im->destroy();
    }
    $imagickTime = (hrtime(true) - $start) / 1e6;
    echo sprintf("  Imagick:   %7.0f ms\n", $imagickTime);
}

echo sprintf("  Speedup vs GD: %.1fx\n", $gdTime / $rustTime);
if (isset($imagickTime)) {
    echo sprintf("  Speedup vs Imagick: %.1fx\n", $imagickTime / $rustTime);
}

// --- Benchmark 2: JPEG → WebP ---
echo "\n--- JPEG to WebP ({$iterations}x) ---\n";

$start = hrtime(true);
for ($i = 0; $i < $iterations; $i++) {
    $image = RustImage\Image::open($source);
    $image->resize(800, 600);
    $image->toWebp(quality: 80);
    $image->save("/tmp/bench_rust_webp_{$i}.webp");
}
$rustTime = (hrtime(true) - $start) / 1e6;
echo sprintf("  RustImage: %7.0f ms\n", $rustTime);

if (function_exists('imagewebp')) {
    $start = hrtime(true);
    for ($i = 0; $i < $iterations; $i++) {
        $src = imagecreatefromjpeg($source);
        $resized = imagescale($src, 800, 600);
        imagewebp($resized, "/tmp/bench_gd_webp_{$i}.webp", 80);
        imagedestroy($src);
        imagedestroy($resized);
    }
    $gdTime = (hrtime(true) - $start) / 1e6;
    echo sprintf("  GD:        %7.0f ms\n", $gdTime);
    echo sprintf("  Speedup vs GD: %.1fx\n", $gdTime / $rustTime);
}

// --- Cleanup ---
echo "\n--- Cleanup ---\n";
$cleaned = 0;
foreach (glob('/tmp/bench_*') as $f) { unlink($f); $cleaned++; }
echo "Cleaned up {$cleaned} benchmark files\n";
```

**Step 3: Run benchmark**

Run: `php -d extension=target/release/libphprs_hello_world.dylib bench_image.php`

Note: Use **release** build for benchmarks! `cargo build --release` first.

**Step 4: Run final test suite**

Run: `php -d extension=target/debug/libphprs_hello_world.dylib test_image.php`
Expected: All tasks pass, cleanup runs.

**Step 5: Commit**

```bash
git add bench_image.php test_image.php
git commit -m "feat(image): add benchmark and test cleanup"
```

---

## Implementation Notes

### ext-php-rs Gotchas to Watch For

1. **Method chaining (`&mut self` return):** ext-php-rs may not support returning `&mut Self` from methods. If compilation fails, change all operation methods to return `Result<(), ImageError>` and adjust PHP tests to call methods sequentially (not chained). The PHP code already uses sequential calls in the test plan.

2. **Passing PHP objects as parameters (`&PhpImage` in overlay):** ext-php-rs may require `&mut PhpImage` or a different signature. If the `overlay` method fails to compile, try accepting a path string instead and opening the overlay internally.

3. **Static methods:** `Image::open()` and `Image::info()` are static constructors. ext-php-rs supports these — the Redis client uses `__construct` but static methods also work. If issues arise, switch to `__construct` for `open`.

4. **`Vec<u8>` return for `to_buffer`:** ext-php-rs should map this to a PHP string. Verify this works correctly.

5. **Named/optional parameters:** `quality: Option<i64>` should work as optional params. ext-php-rs maps Rust `Option<T>` to PHP optional parameters with `null` default.

### Build Sequence

For each task:
1. `cargo build` (debug, for testing)
2. `php -d extension=target/debug/libphprs_hello_world.dylib test_image.php`
3. For benchmarks: `cargo build --release` then use `target/release/libphprs_hello_world.dylib`

### Crate Version Compatibility

The exact crate versions in the plan may need adjustment. If a dependency doesn't compile:
- Check crates.io for the latest compatible version
- Look at the crate's changelog for API changes
- The `ravif` API in particular changes frequently between versions

### What's Deferred to Future Work

- Animated WebP encoding (complex, needs frame assembly)
- Proper GIF disposal method handling
- SVG rasterization (needs `resvg` crate)
- Text rendering (needs `ab_glyph` crate)
- MozJPEG optional feature flag
- `has_alpha` detection in `ImageInfo` (currently always false)
