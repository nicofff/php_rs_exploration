# crop, create, auto_rotate, flip, mirror — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `RustImage\Rgb`, `create()`, `crop()`, `flip()`, `mirror()`, and `auto_rotate()` to the PHP extension, backed by ril 0.10 and kamadak-exif.

**Architecture:** One new file `src/rgb.rs` for the `PhpRgb` color type; all image operations added to the existing `PhpImage` impl in `src/image.rs`; `PhpImage` gains two new fields (`exif_data`, `orientation`) populated at construction. Tests are PHP scripts in `tests/` run against the compiled extension.

**Tech Stack:** Rust / ext-php-rs, ril 0.10 (`features = ["all"]`), kamadak-exif 0.5, PHP 8+ CLI with GD for test fixtures.

**Spec:** `docs/superpowers/specs/2026-03-15-crop-create-autorotate-flip-design.md`

---

## How to build and run tests

```bash
# Build the extension (debug)
cargo build

# Run tests (macOS)
php -dextension=./target/debug/libphprs_hello_world.dylib tests/test_new_api.php

# Run tests (Linux)
php -dextension=./target/debug/libphprs_hello_world.so tests/test_new_api.php
```

---

## File map

| File | Action | Responsibility |
|------|--------|----------------|
| `src/rgb.rs` | Create | `PhpRgb` struct, constructor, getters, `to_rgba()` helper |
| `src/image.rs` | Modify | Add `exif_data`/`orientation` fields; add EXIF helpers; add `create`, `crop`, `flip`, `mirror`, `auto_rotate`; update `open`/`from_buffer` |
| `src/lib.rs` | Modify | Register `PhpRgb` class |
| `tests/test_new_api.php` | Create | Integration tests for all new PHP API surface |
| `tests/fixtures/exif_rotated.jpg` | Create | JPEG with EXIF orientation=6 for `auto_rotate()` testing |

---

## Chunk 1: PhpRgb, PhpImage struct expansion, create()

### Task 1: Verify ril 0.10 API surface

Before writing any code, confirm the exact method names available on `ril::Image<Rgba>`. This unblocks all later tasks.

**Files:**
- Read: `Cargo.lock` (to find exact ril version)
- Run: `cargo doc --open` or check [docs.rs/ril/0.10](https://docs.rs/ril/0.10)

- [ ] **Step 1: Check ril docs for flip, mirror, crop, rotation**

```bash
cargo doc --no-deps 2>&1 | head -5
# Then open target/doc/ril/struct.Image.html in browser, or:
grep -r "fn flip\|fn mirror\|fn crop\|fn rotate" ~/.cargo/registry/src/*/ril-0.10*/src/ 2>/dev/null || \
  find ~/.cargo/registry/src -path "*/ril-0.10*" -name "*.rs" | xargs grep "pub fn flip\|pub fn mirror\|pub fn crop\|pub fn rotate" 2>/dev/null
```

- [ ] **Step 2: Record findings as a comment block at the top of `src/image.rs`**

Add this block right after the existing imports (will be removed or replaced during implementation):

```rust
// ── ril 0.10 API verification ────────────────────────────────────────────────
// Confirmed available:  (fill in from docs)
//   Image::crop(x, y, w, h)       → yes / no / fallback needed
//   Image::flip_vertical / flip   → <actual method name>
//   Image::flip_horizontal/mirror → <actual method name>
//   Image::rotate(degrees)        → yes / no / fallback needed
// ────────────────────────────────────────────────────────────────────────────
```

- [ ] **Step 3: Commit the API notes**

```bash
git add src/image.rs
git commit -m "chore: document ril 0.10 API surface for upcoming features"
```

---

### Task 2: PhpRgb class

**Files:**
- Create: `src/rgb.rs`
- Modify: `src/lib.rs`
- Create: `tests/test_new_api.php` (initial version, tests only PhpRgb)

- [ ] **Step 1: Write the failing test**

Create `tests/test_new_api.php`:

```php
<?php
declare(strict_types=1);

echo "=== New API Test Suite ===\n\n";

// ── Task 2: PhpRgb ────────────────────────────────────────────────────────────
echo "--- Task 2: PhpRgb ---\n";

$color = new RustImage\Rgb(255, 128, 0);
assert($color->r === 255, "r should be 255, got {$color->r}");
assert($color->g === 128, "g should be 128, got {$color->g}");
assert($color->b === 0,   "b should be 0, got {$color->b}");
echo "Property access OK: r={$color->r} g={$color->g} b={$color->b}\n";

$black = new RustImage\Rgb(0, 0, 0);
assert($black->r === 0 && $black->g === 0 && $black->b === 0, "Black should be 0,0,0");
echo "Black OK\n";

$white = new RustImage\Rgb(255, 255, 255);
assert($white->r === 255 && $white->g === 255 && $white->b === 255, "White should be 255,255,255");
echo "White OK\n";

echo "Task 2 PASSED\n\n";
echo "=== Done ===\n";
```

- [ ] **Step 2: Run and confirm it fails**

```bash
php -dextension=./target/debug/libphprs_hello_world.dylib tests/test_new_api.php 2>&1 | head -5
```

Expected: `Class "RustImage\Rgb" not found` or similar fatal.

- [ ] **Step 3: Create `src/rgb.rs`**

```rust
use ril::Rgba;
use ext_php_rs::prelude::*;

#[derive(Default)]
#[php_class]
#[php(name = "RustImage\\Rgb")]
pub struct PhpRgb {
    r: u8,
    g: u8,
    b: u8,
}

#[php_impl]
impl PhpRgb {
    pub fn __construct(&mut self, r: u8, g: u8, b: u8) {
        self.r = r;
        self.g = g;
        self.b = b;
    }

    #[php(getter)]
    pub fn r(&self) -> u8 { self.r }
    #[php(getter)]
    pub fn g(&self) -> u8 { self.g }
    #[php(getter)]
    pub fn b(&self) -> u8 { self.b }
}

impl PhpRgb {
    pub(crate) fn to_rgba(&self) -> Rgba {
        Rgba { r: self.r, g: self.g, b: self.b, a: 255 }
    }
}
```

- [ ] **Step 4: Register in `src/lib.rs`**

Add `mod rgb;` and `.class::<rgb::PhpRgb>()` to the module builder:

```rust
mod rgb;
// ... existing mods ...

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .class::<image_error::ImageException>()
        .class::<image_info::ImageInfo>()
        .class::<image::PhpImage>()
        .class::<rgb::PhpRgb>()   // ← add this
}
```

- [ ] **Step 5: Build**

```bash
cargo build 2>&1
```

Expected: compiles cleanly. Fix any compile errors before continuing.

- [ ] **Step 6: Run test**

```bash
php -dextension=./target/debug/libphprs_hello_world.dylib tests/test_new_api.php
```

Expected: `Task 2 PASSED`

- [ ] **Step 7: Commit**

```bash
git add src/rgb.rs src/lib.rs tests/test_new_api.php
git commit -m "feat(rgb): add RustImage\\Rgb PHP class"
```

---

### Task 3: Expand PhpImage struct with EXIF fields

**Files:**
- Modify: `src/image.rs`

This task has no new PHP-visible behavior — it updates the internal struct and construction. The existing test suite (`test_image.php`) is used to confirm nothing regressed.

- [ ] **Step 1: Add `use crate::rgb::PhpRgb;` to `src/image.rs`**

At the top of `src/image.rs`, after existing `use` statements:

```rust
use crate::rgb::PhpRgb;
```

- [ ] **Step 2: Add two new EXIF helper functions after the existing `read_exif` function**

After the existing `read_exif(path)` function (around line 15):

```rust
fn read_exif_orientation(path: &str) -> Option<u32> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
    exif.fields()
        .find(|f| f.tag == exif::Tag::Orientation)
        .and_then(|f| f.value.get_uint(0))
}

fn read_exif_from_bytes(data: &[u8]) -> Option<std::collections::HashMap<String, String>> {
    let mut reader = std::io::BufReader::new(std::io::Cursor::new(data));
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
    let mut map = std::collections::HashMap::new();
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
```

- [ ] **Step 3: Add `exif_data` and `orientation` fields to `PhpImage`**

Update the struct definition (around line 64):

```rust
#[php_class]
#[php(name = "RustImage\\Image")]
pub struct PhpImage {
    pub(crate) inner: ImageInner,
    pub(crate) output_format: Option<OutputFormat>,
    pub(crate) exif_data: Option<HashMap<String, String>>,
    pub(crate) orientation: Option<u32>,
}
```

- [ ] **Step 4: Update `open()` struct construction**

Find the `Ok(Self { inner, output_format: None })` return in `open()` and replace with:

```rust
Ok(Self {
    inner,
    output_format: None,
    exif_data: read_exif(&path),
    orientation: read_exif_orientation(&path),
})
```

- [ ] **Step 5: Update `from_buffer()` struct construction**

Find the `Ok(Self { inner, output_format: None })` return in `from_buffer()` and replace with:

```rust
Ok(Self {
    inner,
    output_format: None,
    exif_data: read_exif_from_bytes(data),
    orientation: read_exif_orientation_from_bytes(data),
})
```

- [ ] **Step 6: Build and verify no regressions**

```bash
cargo build 2>&1
php -dextension=./target/debug/libphprs_hello_world.dylib test_image.php
```

Expected: all existing tests still pass.

- [ ] **Step 7: Commit**

```bash
git add src/image.rs
git commit -m "feat(image): store exif_data and orientation on PhpImage at construction"
```

---

### Task 4: create()

**Files:**
- Modify: `src/image.rs`
- Modify: `tests/test_new_api.php`

- [ ] **Step 1: Add test to `tests/test_new_api.php`**

Append before the final `echo "=== Done ===\n";`:

```php
// ── Task 4: create() ─────────────────────────────────────────────────────────
echo "--- Task 4: create() ---\n";

// Basic creation and dimension check
$canvas = RustImage\Image::create(200, 100, new RustImage\Rgb(255, 0, 0));
$canvas->toPng();
$tmpCanvas = '/tmp/rustimage_test_canvas.png';
$canvas->save($tmpCanvas);
$info = RustImage\Image::info($tmpCanvas);
assert($info->width === 200, "Canvas width should be 200, got {$info->width}");
assert($info->height === 100, "Canvas height should be 100, got {$info->height}");
echo "Dimensions OK: {$info->width}x{$info->height}\n";

// Verify fill color via GD pixel read
$gd = imagecreatefrompng($tmpCanvas);
$pixel = imagecolorat($gd, 0, 0);
$r = ($pixel >> 16) & 0xFF;
$g = ($pixel >> 8) & 0xFF;
$b = $pixel & 0xFF;
assert($r === 255 && $g === 0 && $b === 0, "Fill color should be red (255,0,0), got $r,$g,$b");
echo "Fill color OK: r=$r g=$g b=$b\n";
imagedestroy($gd);
unlink($tmpCanvas);

// Error: zero dimension
try {
    RustImage\Image::create(0, 100, new RustImage\Rgb(0, 0, 0));
    echo "FAIL: zero width should throw\n";
} catch (RustImage\ImageException $e) {
    echo "Zero width error OK: " . $e->getMessage() . "\n";
}

// Error: negative dimension
try {
    RustImage\Image::create(100, -1, new RustImage\Rgb(0, 0, 0));
    echo "FAIL: negative height should throw\n";
} catch (RustImage\ImageException $e) {
    echo "Negative height error OK: " . $e->getMessage() . "\n";
}

echo "Task 4 PASSED\n\n";
```

- [ ] **Step 2: Run and confirm test fails**

```bash
cargo build 2>&1 && php -dextension=./target/debug/libphprs_hello_world.dylib tests/test_new_api.php
```

Expected: fails at `create()` — method not found.

- [ ] **Step 3: Implement `create()` in `src/image.rs`**

Add inside `#[php_impl] impl PhpImage`:

```rust
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
```

- [ ] **Step 4: Build and run test**

```bash
cargo build 2>&1 && php -dextension=./target/debug/libphprs_hello_world.dylib tests/test_new_api.php
```

Expected: `Task 4 PASSED`

- [ ] **Step 5: Commit**

```bash
git add src/image.rs tests/test_new_api.php
git commit -m "feat(image): add Image::create() static constructor"
```

---

## Chunk 2: crop(), flip(), mirror(), auto_rotate()

### Task 5: crop()

**Files:**
- Modify: `src/image.rs`
- Modify: `tests/test_new_api.php`
- Fixture: `tests/fixtures/animated.gif` (already exists)

- [ ] **Step 1: Add crop tests to `tests/test_new_api.php`**

Append before the final `echo "=== Done ===\n";`:

```php
// ── Task 5: crop() ───────────────────────────────────────────────────────────
echo "--- Task 5: crop() ---\n";

// Create a 200x150 test JPEG with a known red square at top-left
$tmpSrc = '/tmp/rustimage_crop_src.jpg';
$gd = imagecreatetruecolor(200, 150);
imagefill($gd, 0, 0, imagecolorallocate($gd, 255, 255, 255));
imagefilledrectangle($gd, 0, 0, 49, 49, imagecolorallocate($gd, 255, 0, 0));
imagejpeg($gd, $tmpSrc, 100);
imagedestroy($gd);

// Basic crop — should produce 100x80
$img = RustImage\Image::open($tmpSrc);
$img->crop(0, 0, 100, 80);
$img->toPng();
$tmpOut = '/tmp/rustimage_crop_out.png';
$img->save($tmpOut);
$info = RustImage\Image::info($tmpOut);
assert($info->width === 100, "Crop width should be 100, got {$info->width}");
assert($info->height === 80,  "Crop height should be 80, got {$info->height}");
echo "Dimensions OK: {$info->width}x{$info->height}\n";
unlink($tmpOut);

// Crop with offset — red square should be gone (cropped from right side)
$img = RustImage\Image::open($tmpSrc);
$img->crop(100, 0, 100, 150);
$img->toPng();
$tmpOut2 = '/tmp/rustimage_crop_offset.png';
$img->save($tmpOut2);
$gd = imagecreatefrompng($tmpOut2);
$pixel = imagecolorat($gd, 0, 0);
$r = ($pixel >> 16) & 0xFF;
assert($r < 200, "Top-left of offset crop should not be red, got r=$r");
echo "Offset crop color OK: r=$r\n";
imagedestroy($gd);
unlink($tmpOut2);
unlink($tmpSrc);

// Error: out of bounds
$img2 = RustImage\Image::create(200, 150, new RustImage\Rgb(255, 255, 255));
try {
    $img2->crop(0, 0, 300, 300);
    echo "FAIL: out-of-bounds crop should throw\n";
} catch (RustImage\ImageException $e) {
    echo "Out-of-bounds error OK: " . $e->getMessage() . "\n";
}

// Error: negative x
try {
    $img2->crop(-1, 0, 100, 100);
    echo "FAIL: negative x should throw\n";
} catch (RustImage\ImageException $e) {
    echo "Negative x error OK: " . $e->getMessage() . "\n";
}

// Animated GIF crop — all frames should be cropped
$gif = RustImage\Image::open(__DIR__ . '/fixtures/animated.gif');
$gifInfo = RustImage\Image::info(__DIR__ . '/fixtures/animated.gif');
$gifW = $gifInfo->width;
$gifH = $gifInfo->height;
$cropW = (int)($gifW / 2);
$cropH = (int)($gifH / 2);
$gif->crop(0, 0, $cropW, $cropH);
$gif->toGif();
$tmpGif = '/tmp/rustimage_crop_anim.gif';
$gif->save($tmpGif);
$gifOutInfo = RustImage\Image::info($tmpGif);
assert($gifOutInfo->width === $cropW, "Animated crop width should be $cropW, got {$gifOutInfo->width}");
assert($gifOutInfo->height === $cropH, "Animated crop height should be $cropH, got {$gifOutInfo->height}");
echo "Animated GIF crop OK: {$gifOutInfo->width}x{$gifOutInfo->height}\n";
unlink($tmpGif);

echo "Task 5 PASSED\n\n";
```

- [ ] **Step 2: Run and confirm test fails**

```bash
cargo build 2>&1 && php -dextension=./target/debug/libphprs_hello_world.dylib tests/test_new_api.php
```

Expected: fails at crop — method not found.

- [ ] **Step 3: Implement `crop()` in `src/image.rs`**

First verify whether `ril::Image` has a `crop` method (from Task 1 notes). If yes, use it; if no, use the manual pixel fallback.

**With ril crop (preferred):**

```rust
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

    match &mut self.inner {
        ImageInner::Static(img) => {
            let (iw, ih) = img.dimensions();
            if x + width > iw as i64 || y + height > ih as i64 {
                return Err(ImageError("crop: region exceeds image bounds".into()));
            }
            *img = img.crop(x as u32, y as u32, width as u32, height as u32);
        }
        ImageInner::Animated(seq) => {
            for frame in seq.iter_mut() {
                let (iw, ih) = frame.image().dimensions();
                if x + width > iw as i64 || y + height > ih as i64 {
                    return Err(ImageError("crop: region exceeds frame bounds".into()));
                }
                let cropped = frame.image().crop(x as u32, y as u32, width as u32, height as u32);
                *frame.image_mut() = cropped;
            }
        }
    }
    Ok(())
}
```

**Manual fallback (if ril lacks `crop`):**

```rust
fn crop_image(img: &Image<Rgba>, x: u32, y: u32, w: u32, h: u32) -> Image<Rgba> {
    let mut dst = Image::new(w, h, Rgba { r: 0, g: 0, b: 0, a: 0 });
    for dy in 0..h {
        for dx in 0..w {
            dst.set_pixel(dx, dy, *img.pixel(x + dx, y + dy));
        }
    }
    dst
}
```

Use `crop_image(img, x as u32, y as u32, width as u32, height as u32)` in place of `img.crop(...)`.

Note: `img.crop(...)` may return a new `Image` (not mutate). Adjust if it returns `Self` vs `&mut Self`.

- [ ] **Step 4: Build and run test**

```bash
cargo build 2>&1 && php -dextension=./target/debug/libphprs_hello_world.dylib tests/test_new_api.php
```

Expected: `Task 5 PASSED`

- [ ] **Step 5: Commit**

```bash
git add src/image.rs tests/test_new_api.php
git commit -m "feat(image): add crop() for static and animated images"
```

---

### Task 6: flip() and mirror()

**Files:**
- Modify: `src/image.rs`
- Modify: `tests/test_new_api.php`

These are also used internally by `auto_rotate()` in the next task, so they must be correct.

- [ ] **Step 1: Add flip/mirror tests to `tests/test_new_api.php`**

Append before the final `echo "=== Done ===\n";`:

```php
// ── Task 6: flip() and mirror() ──────────────────────────────────────────────
echo "--- Task 6: flip() and mirror() ---\n";

// Helper: create a 10x10 image, red pixel at top-left (0,0), rest white
function makeTestImage(): string {
    $path = '/tmp/rustimage_flip_src.png';
    $gd = imagecreatetruecolor(10, 10);
    imagefill($gd, 0, 0, imagecolorallocate($gd, 255, 255, 255));
    imagesetpixel($gd, 0, 0, imagecolorallocate($gd, 255, 0, 0));
    imagepng($gd, $path);
    imagedestroy($gd);
    return $path;
}

function getPixelRed(string $path, int $x, int $y): int {
    $gd = imagecreatefrompng($path);
    $pixel = imagecolorat($gd, $x, $y);
    imagedestroy($gd);
    return ($pixel >> 16) & 0xFF;
}

// flip() — red pixel (0,0) should move to (0,9)
$src = makeTestImage();
$img = RustImage\Image::open($src);
$img->flip();
$img->toPng();
$out = '/tmp/rustimage_flip_out.png';
$img->save($out);
assert(getPixelRed($out, 0, 9) > 200, "After flip(), red should be at (0,9)");
assert(getPixelRed($out, 0, 0) < 50,  "After flip(), (0,0) should not be red");
echo "flip() OK\n";
unlink($out);
unlink($src);

// mirror() — red pixel (0,0) should move to (9,0)
$src = makeTestImage();
$img = RustImage\Image::open($src);
$img->mirror();
$img->toPng();
$out = '/tmp/rustimage_mirror_out.png';
$img->save($out);
assert(getPixelRed($out, 9, 0) > 200, "After mirror(), red should be at (9,0)");
assert(getPixelRed($out, 0, 0) < 50,  "After mirror(), (0,0) should not be red");
echo "mirror() OK\n";
unlink($out);
unlink($src);

echo "Task 6 PASSED\n\n";
```

- [ ] **Step 2: Run and confirm test fails**

```bash
cargo build 2>&1 && php -dextension=./target/debug/libphprs_hello_world.dylib tests/test_new_api.php
```

Expected: fails at flip/mirror — method not found.

- [ ] **Step 3: Implement `flip()` and `mirror()` in `src/image.rs`**

First check Task 1 notes for exact ril method names. The expected names are `flip` and `mirror` on `ril::Image`. If different (e.g., `flip_vertical` / `flip_horizontal`), use those internally.

```rust
pub fn flip(&mut self) -> Result<(), ImageError> {
    match &mut self.inner {
        ImageInner::Static(img) => { img.flip(); }   // verify ril method name
        ImageInner::Animated(seq) => {
            for frame in seq.iter_mut() {
                frame.image_mut().flip();             // same name
            }
        }
    }
    Ok(())
}

pub fn mirror(&mut self) -> Result<(), ImageError> {
    match &mut self.inner {
        ImageInner::Static(img) => { img.mirror(); } // verify ril method name
        ImageInner::Animated(seq) => {
            for frame in seq.iter_mut() {
                frame.image_mut().mirror();           // same name
            }
        }
    }
    Ok(())
}
```

**Manual fallback for flip (vertical) if ril lacks it:**

```rust
fn flip_image(img: &mut Image<Rgba>) {
    let (w, h) = img.dimensions();
    for y in 0..(h / 2) {
        for x in 0..w {
            let top = *img.pixel(x, y);
            let bot = *img.pixel(x, h - 1 - y);
            img.set_pixel(x, y, bot);
            img.set_pixel(x, h - 1 - y, top);
        }
    }
}
```

**Manual fallback for mirror (horizontal):**

```rust
fn mirror_image(img: &mut Image<Rgba>) {
    let (w, h) = img.dimensions();
    for y in 0..h {
        for x in 0..(w / 2) {
            let left  = *img.pixel(x, y);
            let right = *img.pixel(w - 1 - x, y);
            img.set_pixel(x, y, right);
            img.set_pixel(w - 1 - x, y, left);
        }
    }
}
```

- [ ] **Step 4: Build and run test**

```bash
cargo build 2>&1 && php -dextension=./target/debug/libphprs_hello_world.dylib tests/test_new_api.php
```

Expected: `Task 6 PASSED`

- [ ] **Step 5: Commit**

```bash
git add src/image.rs tests/test_new_api.php
git commit -m "feat(image): add flip() and mirror()"
```

---

### Task 7: auto_rotate() — EXIF fixture + implementation

**Files:**
- Create: `tests/fixtures/exif_rotated.jpg` (requires `exiftool`)
- Modify: `src/image.rs`
- Modify: `tests/test_new_api.php`

- [ ] **Step 1: Create EXIF orientation fixture**

```bash
# Create a 100x50 JPEG (wider than tall), then tag it with orientation=6
# Orientation 6 = needs 90° CW rotation, so the correct display is 50x100
cd /Users/nfar/Documents/nicofff/phprs_hello_world

php -r "
\$gd = imagecreatetruecolor(100, 50);
imagefill(\$gd, 0, 0, imagecolorallocate(\$gd, 255, 255, 255));
imagefilledrectangle(\$gd, 0, 0, 20, 20, imagecolorallocate(\$gd, 255, 0, 0));
imagejpeg(\$gd, 'tests/fixtures/exif_rotated.jpg', 95);
imagedestroy(\$gd);
echo 'Created base JPEG\n';
"

# Set EXIF orientation to 6 (Rotate 90 CW) using exiftool
exiftool -Orientation=6 -n -overwrite_original tests/fixtures/exif_rotated.jpg
echo "EXIF orientation set"

# Verify
exiftool -Orientation tests/fixtures/exif_rotated.jpg
```

Expected: `Orientation: 6`

- [ ] **Step 2: Add auto_rotate tests to `tests/test_new_api.php`**

Append before the final `echo "=== Done ===\n";`:

```php
// ── Task 7: auto_rotate() ────────────────────────────────────────────────────
echo "--- Task 7: auto_rotate() ---\n";

// No-op on image without EXIF (should not error or change dimensions)
$img = RustImage\Image::create(100, 50, new RustImage\Rgb(255, 255, 255));
$img->auto_rotate();
$img->toPng();
$tmp = '/tmp/rustimage_autorotate_noop.png';
$img->save($tmp);
$info = RustImage\Image::info($tmp);
assert($info->width === 100 && $info->height === 50, "No EXIF: dimensions should be unchanged");
echo "No-op (no EXIF) OK: {$info->width}x{$info->height}\n";
unlink($tmp);

// Idempotency — calling twice should not double-rotate
$img = RustImage\Image::open(__DIR__ . '/fixtures/exif_rotated.jpg');
$img->auto_rotate();
$img->auto_rotate(); // second call should be a no-op
$img->toPng();
$tmpIdem = '/tmp/rustimage_autorotate_idem.png';
$img->save($tmpIdem);
$infoIdem = RustImage\Image::info($tmpIdem);
// Orientation 6 rotates 90° CW: 100x50 becomes 50x100
assert($infoIdem->width === 50 && $infoIdem->height === 100,
    "After auto_rotate (orientation 6): should be 50x100, got {$infoIdem->width}x{$infoIdem->height}");
echo "Idempotency OK: second call is no-op\n";
unlink($tmpIdem);

// Orientation 6: 100x50 JPEG → after 90° CW rotation → 50x100
$img = RustImage\Image::open(__DIR__ . '/fixtures/exif_rotated.jpg');
$img->auto_rotate();
$img->toPng();
$tmpRot = '/tmp/rustimage_autorotate_rot.png';
$img->save($tmpRot);
$infoRot = RustImage\Image::info($tmpRot);
assert($infoRot->width === 50 && $infoRot->height === 100,
    "Orientation 6 should produce 50x100, got {$infoRot->width}x{$infoRot->height}");
echo "Orientation 6 rotation OK: {$infoRot->width}x{$infoRot->height}\n";
unlink($tmpRot);

echo "Task 7 PASSED\n\n";
```

- [ ] **Step 3: Run and confirm test fails**

```bash
cargo build 2>&1 && php -dextension=./target/debug/libphprs_hello_world.dylib tests/test_new_api.php
```

Expected: fails at auto_rotate — method not found.

- [ ] **Step 4: Add rotation helper to `src/image.rs`**

Add a private Rust function for rotation (below `apply_overlay`). First check Task 1 notes — if ril has rotation, use it instead:

```rust
fn rotate_image_cw(img: &Image<Rgba>) -> Image<Rgba> {
    // 90° CW: W×H source → H×W destination
    // source (x, y) → dest (H-1-y, x)
    let (w, h) = img.dimensions();
    let mut dst = Image::new(h, w, Rgba { r: 0, g: 0, b: 0, a: 0 });
    for y in 0..h {
        for x in 0..w {
            dst.set_pixel(h - 1 - y, x, *img.pixel(x, y));
        }
    }
    dst
}

fn rotate_image_ccw(img: &Image<Rgba>) -> Image<Rgba> {
    // 90° CCW: W×H source → H×W destination
    // source (x, y) → dest (y, W-1-x)
    let (w, h) = img.dimensions();
    let mut dst = Image::new(h, w, Rgba { r: 0, g: 0, b: 0, a: 0 });
    for y in 0..h {
        for x in 0..w {
            dst.set_pixel(y, w - 1 - x, *img.pixel(x, y));
        }
    }
    dst
}

fn rotate_image_180(img: &Image<Rgba>) -> Image<Rgba> {
    // 180°: same size, (x,y) → (W-1-x, H-1-y)
    let (w, h) = img.dimensions();
    let mut dst = Image::new(w, h, Rgba { r: 0, g: 0, b: 0, a: 0 });
    for y in 0..h {
        for x in 0..w {
            dst.set_pixel(w - 1 - x, h - 1 - y, *img.pixel(x, y));
        }
    }
    dst
}
```

- [ ] **Step 5: Implement `auto_rotate()` in `src/image.rs`**

Add `apply_rotation` as a **module-level free function** (outside any `impl` block — if placed inside `#[php_impl]`, ext-php-rs will attempt to expose it as a PHP method and compilation will fail):

```rust
// Module-level free function — NOT inside #[php_impl].
// Avoids borrow conflicts: self.inner is released before self.mirror()/self.flip() are called.
fn apply_rotation(inner: &mut ImageInner, f: fn(&Image<Rgba>) -> Image<Rgba>) {
    match inner {
        ImageInner::Static(img) => { *img = f(img); }
        ImageInner::Animated(seq) => {
            for frame in seq.iter_mut() {
                let rotated = f(frame.image());
                *frame.image_mut() = rotated;
            }
        }
    }
}

// Inside #[php_impl] impl PhpImage:
pub fn auto_rotate(&mut self) -> Result<(), ImageError> {
    let orientation = match self.orientation {
        None | Some(1) => return Ok(()),
        Some(v) if v > 8 => return Ok(()),
        Some(v) => v,
    };

    match orientation {
        2 => self.mirror()?,
        3 => apply_rotation(&mut self.inner, rotate_image_180),
        4 => self.flip()?,
        5 => {
            apply_rotation(&mut self.inner, rotate_image_cw);
            self.mirror()?;
        }
        6 => apply_rotation(&mut self.inner, rotate_image_cw),
        7 => {
            apply_rotation(&mut self.inner, rotate_image_ccw);
            self.mirror()?;
        }
        8 => apply_rotation(&mut self.inner, rotate_image_ccw),
        _ => unreachable!(),
    }

    self.orientation = Some(1);
    Ok(())
}
```

**Note:** If ril provides rotation natively (verified in Task 1), replace the `rotate_image_*` calls with ril's methods. If ril's rotation is a `&mut self` method (mutates in place), update `apply_rotation` to call the ril method directly on `img` instead of replacing it with the return value.

- [ ] **Step 6: Build and run test**

```bash
cargo build 2>&1 && php -dextension=./target/debug/libphprs_hello_world.dylib tests/test_new_api.php
```

Expected: `Task 7 PASSED` and `=== Done ===`

- [ ] **Step 7: Run full existing test suite to confirm no regressions**

```bash
php -dextension=./target/debug/libphprs_hello_world.dylib test_image.php
```

Expected: all existing tasks pass.

- [ ] **Step 8: Commit**

```bash
git add src/image.rs tests/test_new_api.php tests/fixtures/exif_rotated.jpg
git commit -m "feat(image): add auto_rotate() with EXIF orientation support"
```

---

## Final verification

- [ ] **Build release and run full test suite**

```bash
cargo build --release
php -dextension=./target/release/libphprs_hello_world.dylib tests/test_new_api.php
php -dextension=./target/release/libphprs_hello_world.dylib test_image.php
```

Expected: all tests pass with the release build.

- [ ] **Final commit if any cleanup was needed**

```bash
git add -p
git commit -m "chore: cleanup after new API implementation"
```
