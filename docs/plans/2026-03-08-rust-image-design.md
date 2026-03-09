# RustImage: Rust-Powered Image Processing Extension for PHP

**Date:** 2026-03-08
**Status:** Design approved
**Approach:** Hybrid — `image` crate + SIMD-accelerated resize + optional MozJPEG

## Motivation

Image processing in PHP has two options, both flawed:

- **GD** — Limited formats, poor quality, no animated GIF support, basic operations only
- **Imagick** — Wraps ImageMagick, which brings dependency hell, security CVEs, resource limit crashes that segfault PHP, and complex animated image handling

Rust solves these problems: memory-safe (no segfaults), zero system dependencies (pure Rust decoders), and `fast_image_resize` delivers SIMD-accelerated performance that benchmarks *faster* than libvips.

## Architecture

### PHP API Surface

Three classes in the `RustImage` namespace:

```
RustImage\Image          — main class, fluent/chainable API
RustImage\ImageInfo      — lightweight metadata (no pixel decode)
RustImage\ImageException — typed exception for all errors
```

### Fluent API

```php
use RustImage\Image;

// Basic resize + convert
Image::open('photo.jpg')
    ->resize(800, 600)
    ->toWebp(quality: 80)
    ->save('photo.webp');

// Thumbnail with aspect ratio preservation
Image::open('photo.jpg')
    ->thumbnail(200, 200)
    ->toJpeg(quality: 85)
    ->save('thumb.jpg');

// Animated GIF to animated WebP
Image::open('animation.gif')
    ->resize(400, 300)
    ->toWebp(quality: 75)
    ->save('animation.webp');

// Chain multiple operations
Image::open('photo.jpg')
    ->crop(100, 100, 500, 500)
    ->resize(800, 600)
    ->grayscale()
    ->toAvif(quality: 60)
    ->save('processed.avif');

// Output to buffer (for HTTP responses)
$bytes = Image::open('photo.jpg')
    ->resize(800, 600)
    ->toWebp(quality: 80)
    ->toBuffer();

// Load from buffer (e.g., from S3/upload)
$image = Image::fromBuffer($uploadedBytes);

// Composition / watermark
Image::open('photo.jpg')
    ->resize(800, 600)
    ->overlay(Image::open('watermark.png'), x: 10, y: 10, opacity: 0.5)
    ->toJpeg(quality: 90)
    ->save('watermarked.jpg');
```

### Metadata (no pixel decoding)

```php
$info = Image::info('photo.jpg');
$info->width;       // int
$info->height;      // int
$info->format;      // string: "jpeg", "png", "gif", "webp", ...
$info->hasAlpha;    // bool
$info->isAnimated;  // bool
$info->exif;        // ?array — EXIF data if present
```

### Resource Limits

The anti-ImageMagick-crash feature. Exceeding limits throws `ImageException` instead of segfaulting:

```php
Image::open('untrusted-upload.jpg', maxWidth: 10000, maxHeight: 10000, maxBytes: 50_000_000)
    ->resize(800, 600)
    ->toWebp()
    ->save('safe.webp');
```

### Resize Modes

```php
->resize(800, 600)                          // default: 'contain'
->resize(800, 600, fit: 'contain')          // fit within, preserve aspect ratio
->resize(800, 600, fit: 'cover')            // cover area, crop excess
->resize(800, 600, fit: 'fill')             // stretch to exact dimensions
->thumbnail(200, 200)                       // alias for contain + fast bilinear
```

### Supported Formats

| Format | Decode | Encode | Animated |
|--------|--------|--------|----------|
| JPEG   | Yes    | Yes (MozJPEG optional) | N/A |
| PNG    | Yes    | Yes    | No  |
| GIF    | Yes    | Yes    | Yes |
| WebP   | Yes    | Yes    | Yes |
| AVIF   | Yes    | Yes    | No  |
| BMP    | Yes    | Yes    | No  |
| TIFF   | Yes    | Yes    | No  |
| ICO    | Yes    | No     | No  |

## Rust Internals

### Struct Design

```rust
#[php_class(name = "RustImage\\Image")]
pub struct Image {
    frames: Vec<Frame>,
    source_format: Option<ImageFormat>,
}

struct Frame {
    buffer: RgbaImage,       // RGBA8 pixel buffer from `image` crate
    delay_ms: u32,           // frame delay (0 for static images)
}

#[php_class(name = "RustImage\\ImageInfo")]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub has_alpha: bool,
    pub is_animated: bool,
    pub exif: Option<HashMap<String, String>>,
}

#[php_class(name = "RustImage\\ImageException")]
pub struct ImageException;
// extends \Exception via ext-php-rs
```

### Method Signatures

```rust
#[php_impl]
impl Image {
    // Constructors
    pub fn open(path: String, max_width: Option<u32>, max_height: Option<u32>, max_bytes: Option<u64>) -> PhpResult<Self>;
    pub fn from_buffer(bytes: Vec<u8>) -> PhpResult<Self>;

    // Operations — mutate in place, return &mut self for chaining
    pub fn resize(&mut self, width: u32, height: u32, fit: Option<String>) -> PhpResult<&mut Self>;
    pub fn crop(&mut self, x: u32, y: u32, width: u32, height: u32) -> PhpResult<&mut Self>;
    pub fn grayscale(&mut self) -> PhpResult<&mut Self>;
    pub fn overlay(&mut self, other: &Image, x: i32, y: i32, opacity: Option<f32>) -> PhpResult<&mut Self>;
    pub fn thumbnail(&mut self, width: u32, height: u32) -> PhpResult<&mut Self>;

    // Format selection
    pub fn to_jpeg(&mut self, quality: Option<u8>) -> PhpResult<&mut Self>;
    pub fn to_webp(&mut self, quality: Option<u8>) -> PhpResult<&mut Self>;
    pub fn to_avif(&mut self, quality: Option<u8>) -> PhpResult<&mut Self>;
    pub fn to_png(&mut self) -> PhpResult<&mut Self>;

    // Output
    pub fn save(&self, path: String) -> PhpResult<()>;
    pub fn to_buffer(&self) -> PhpResult<Vec<u8>>;

    // Static metadata reader
    pub fn info(path: String) -> PhpResult<ImageInfo>;
}
```

### Resize Pipeline

```
Input pixels (RgbaImage)
    |
    v
fast_image_resize::Resizer        <-- SIMD-accelerated (AVX2/SSE4/NEON)
    |  - Lanczos3 filter (high quality, default)
    |  - Bilinear (fast, for thumbnails)
    |  - Nearest (pixel art)
    |
    v
Output pixels (RgbaImage)
```

For animated images, each frame goes through the same pipeline independently.

### Crate Dependencies

```toml
[dependencies]
ext-php-rs = "*"

# Core image handling
image = "0.25"
fast_image_resize = "5"

# Format-specific
gif = "0.13"
webp = "0.3"
ravif = "0.11"
libavif = "0.14"

# Metadata
kamadak-exif = "0.5"

# Optional high-quality JPEG
mozjpeg = { version = "0.10", optional = true }

[features]
default = []
mozjpeg = ["dep:mozjpeg"]
```

### Error Handling

All Rust errors map to `RustImage\ImageException`:

| Rust error | PHP exception message |
|---|---|
| File not found | `"Failed to open image: file not found: {path}"` |
| Unsupported format | `"Unsupported image format: {ext}"` |
| Exceeds limits | `"Image dimensions 5000x3000 exceed limit 4000x4000"` |
| Corrupt image data | `"Failed to decode image: {detail}"` |
| Encode failure | `"Failed to encode as {format}: {detail}"` |
| Write failure | `"Failed to save to {path}: {detail}"` |

### File Structure

```
src/
├── lib.rs              # PHP module registration, Image + ImageInfo + ImageException classes
├── decode.rs           # Format detection, decoding, animated GIF/WebP frame extraction
├── encode.rs           # JPEG/PNG/WebP/AVIF encoding, animated assembly
├── ops.rs              # resize, crop, grayscale, overlay operations
└── info.rs             # Lightweight metadata reading (no full decode)
```

## Testing Strategy

### Test Fixtures (`tests/fixtures/`)

| File | Purpose |
|---|---|
| `photo.jpg` | Standard JPEG, ~2000x1500 |
| `transparent.png` | PNG with alpha channel |
| `animation.gif` | Animated GIF, 10+ frames |
| `tiny.png` | 1x1 pixel (edge case) |
| `large.jpg` | 4000x3000 (resource limit testing) |
| `corrupt.jpg` | Truncated file (error handling) |
| `no-exif.jpg` | JPEG without EXIF |
| `with-exif.jpg` | JPEG with GPS, orientation, camera data |

### Integration Tests (`test_image.php`)

1. Basic open + save roundtrip
2. Resize + format conversion (verify dimensions via `Image::info`)
3. Animated GIF to animated WebP (verify `isAnimated`)
4. Aspect ratio preservation with `contain` mode
5. Crop (verify exact output dimensions)
6. Alpha channel preservation through resize
7. Buffer round-trip (`toBuffer` then `fromBuffer`)
8. EXIF reading
9. Resource limits throw `ImageException`
10. Corrupt file throws `ImageException`
11. Overlay / watermark composition

### Benchmark (`bench_image.php`)

Compare RustImage vs GD vs Imagick on 100 iterations of:

1. **Thumbnail generation** — Resize 2000x1500 to 200x200 JPEG
2. **Format conversion** — JPEG to WebP
3. **Animated resize** — Animated GIF resize (10 frames)

### Target Performance

| Operation (100x) | GD | Imagick | RustImage |
|---|---|---|---|
| Resize 2000x1500 to 200x200 | ~3000ms | ~2500ms | ~800ms |
| JPEG to WebP | ~4000ms | ~3500ms | ~1200ms |
| Animated GIF resize (10 frames) | N/A | ~8000ms | ~2000ms |

## Decisions & Trade-offs

- **Pure Rust decoders** over system libraries — zero dependencies wins over marginal decode speed
- **RGBA8 internal format** — simplifies operations at cost of ~33% more memory vs RGB for opaque images; worth it for uniform pipeline
- **MozJPEG as optional feature** — it's a C dependency, so opt-in only; default JPEG encoder from `image` crate is good enough for most uses
- **Multi-file structure** from the start — unlike the Redis client's single lib.rs, image processing has enough distinct concerns (decode/encode/ops/info) to warrant separation
- **Single-threaded initially** — animated frame processing is naturally parallelizable with `rayon`, but deferred to avoid complexity in v1
