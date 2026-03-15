# Design: crop, create, auto_rotate, flip, mirror

Date: 2026-03-15
Branch: threads

## Context

This extension expands the functionality of the extension with four operations:
pixel-level crop, blank canvas creation, EXIF-based auto-rotation, and image flipping. This spec
covers the design for all four, plus the supporting `RustImage\Rgb` color type.

## Naming convention

Throughout this spec, `PhpImage` refers to the Rust struct. Its PHP-facing class name is
`RustImage\Image` (via `#[php(name = "RustImage\\Image")]`). PHP examples use `RustImage\Image`.
Similarly, `PhpRgb` is the Rust struct; its PHP name is `RustImage\Rgb`.

## Scope

- New PHP class: `RustImage\Rgb`
- New static constructor on `RustImage\Image`: `create()`
- New instance methods on `RustImage\Image`: `crop()`, `auto_rotate()`, `flip()`, `mirror()`
- EXIF data stored on `PhpImage` at construction time (used by `auto_rotate()`)

Out of scope: text rendering, progressive JPEG, PDF rasterization, strip_metadata,
transparent canvas creation (canvas always has alpha = 255 via `to_rgba()`),
harmonizing `resize()`'s missing `u32::MAX` guard (a pre-existing inconsistency, not addressed
here), optimizing `ImageInfo::info()` to reuse a stored EXIF map (it remains a static method with
no instance to draw from, so it continues to call `read_exif` directly).

**Note on `info()` and in-memory state:** `Image::info($path)` always reads from disk and will
not reflect transformations applied to an in-memory `PhpImage`. Callers should not use `info()`
to check the current orientation after calling `auto_rotate()`.

---

## 1. `RustImage\Rgb`

### File

New file: `src/rgb.rs`. Required imports: `use ril::Rgba;`. Registered in `lib.rs` alongside
existing classes.

### PHP constructor pattern

The existing `PhpImage` has no PHP-level constructor (it uses static factory methods like
`open()`). `PhpRgb` needs a real PHP constructor so callers can write `new RustImage\Rgb(r, g, b)`.
In ext-php-rs this is done with `pub fn __construct(&mut self, ...)` inside `#[php_impl]`, with the
struct deriving `Default` so the runtime can allocate it before calling `__construct`.

### API

```php
$color = new RustImage\Rgb(255, 0, 0);
$color->r; // 255  (PHP property access via #[php(getter)], not a PHP method call)
$color->g; // 0
$color->b; // 0
```

### Implementation

```rust
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

    // #[php(getter)] exposes these as $color->r etc. in PHP — same mechanism
    // as ImageInfo. Struct field names and getter method names are identical
    // (r/g/b), so no aliasing is needed.
    #[php(getter)]
    pub fn r(&self) -> u8 { self.r }
    #[php(getter)]
    pub fn g(&self) -> u8 { self.g }
    #[php(getter)]
    pub fn b(&self) -> u8 { self.b }
}
```

No setters. The type is immutable from the PHP side.

Internal helper on `PhpRgb` (used by `create()` in `image.rs`, which will `use crate::rgb::PhpRgb`).
The `a: 255` ensures all canvases created via `create()` are fully opaque:

```rust
pub(crate) fn to_rgba(&self) -> Rgba {
    Rgba { r: self.r, g: self.g, b: self.b, a: 255 }
}
```

---

## 2. `RustImage\Image::create()`

Static methods in ext-php-rs are functions with no `&self` / `&mut self` receiver — the same
pattern as the existing `open()` and `info()` methods. No additional attribute is required.

The `color: &PhpRgb` parameter follows the same by-reference pattern as `other: &PhpImage` in the
existing `overlay()` method (line 181 of `image.rs`). Since `PhpRgb` is a `#[php_class]`, ext-php-rs
handles the PHP object → `&PhpRgb` conversion identically to the existing `&PhpImage` precedent.

### API

```php
$canvas = RustImage\Image::create(1024, 768, new RustImage\Rgb(0, 0, 0));
// Produces a fully opaque canvas (alpha = 255 via to_rgba()).
// Encoding defaults to PNG when no explicit format is set, same as open().
```

### Behavior

- Static constructor returning a new `PhpImage`
- `ImageInner::Static` filled with the given color via ril's `Image::new(width, height, color.to_rgba())`
- `output_format` defaults to `None` (encoding falls back to PNG via the existing `encode_to_bytes` logic)
- `exif_data` is `None`; `orientation` is `None`
- `auto_rotate()` on an image produced by `create()` is always a no-op
- Validation (throw `ImageException` on failure):
  - `width <= 0` or `height <= 0` → error. The `<= 0` check covers all negative values and zero;
    no separate `< 0` check is needed.
  - `width > u32::MAX as i64` or `height > u32::MAX as i64` → error
  - Values in `1..=(u32::MAX as i64)` are valid and cast directly to `u32`

### Struct construction

```rust
Ok(Self {
    inner: ImageInner::Static(Image::new(width as u32, height as u32, color.to_rgba())),
    output_format: None,
    exif_data: None,
    orientation: None,
})
```

### Signature

```rust
pub fn create(width: i64, height: i64, color: &PhpRgb) -> Result<Self, ImageError>
```

---

## 3. `crop(x, y, width, height)`

### API

```php
$img->crop(0, 0, 460, 280);
```

### Behavior

- Mutates in place
- Parameters: `x`, `y` (top-left origin), `width`, `height` — all `i64`
- Validation order (throw `ImageException` on first failure). All comparisons are in `i64`
  arithmetic to avoid u32 overflow in the bounds sum:
  1. `x < 0 || y < 0` → error
  2. `width <= 0 || height <= 0` → error
  3. `x > u32::MAX as i64 || y > u32::MAX as i64 || width > u32::MAX as i64 || height > u32::MAX as i64` → error
  4. `x + width > img_width as i64 || y + height > img_height as i64` → error
  5. Cast `x`, `y`, `width`, `height` to `u32` for use with ril
- A crop equal to the full image dimensions passes validation; no special-casing required.
- For `ImageInner::Static`: replaces inner image with cropped result using ril's `Image::crop()`
- For `ImageInner::Animated`: crops each frame. Access the frame image via `frame.image_mut()`.
  Replace it with the cropped result using `*frame.image_mut() = cropped_image`, which is valid
  because `image_mut()` returns `&mut Image<Rgba>` and assignment through a mutable reference is
  standard Rust. If ril's `Frame` type does not support this assignment pattern, use
  `Frame::from_image()` or equivalent — verify against ril 0.10's `Frame` API at implementation
  time.
- **Fallback if ril 0.10 lacks `Image::crop()`**: for both static and per-frame animated paths,
  manually copy pixels. Read source pixels with `src.pixel(sx, sy)` and write to a new image with
  `dst.set_pixel(dx, dy, pixel)`, using the same APIs already established in `apply_overlay`
  (lines 249–270 of `image.rs`). Initialize the destination as
  `Image::new(width_u32, height_u32, Rgba { r:0, g:0, b:0, a:0 })` (transparent black; since
  the crop region is fully within bounds, no background pixel will remain visible).

### Signature

```rust
pub fn crop(&mut self, x: i64, y: i64, width: i64, height: i64) -> Result<(), ImageError>
```

---

## 4. EXIF storage on `PhpImage`

### Motivation

`auto_rotate()` needs the EXIF orientation tag. EXIF data is parsed once at construction and stored
on the struct. Two separate fields are used:

- `exif_data: Option<HashMap<String, String>>` — full EXIF map with display-string values, same
  data already exposed by `ImageInfo::exif`. This field is **internal-only**; no PHP getter is
  added to `PhpImage` for it (callers who need EXIF metadata already use `Image::info()`).
- `orientation: Option<u32>` — raw numeric orientation value (1–8) extracted from the EXIF field
  for tag `exif::Tag::Orientation` via `field.value.get_uint(0)` (which returns `Option<u32>`).
  Using `u32` directly avoids any truncating cast. Stored separately because `field.display_value()`
  produces locale-dependent strings like `"Normal"` or `"Rotate 90 CW"` that are fragile to parse.

Both fields are `None` for images that carry no EXIF data. PNG and GIF files typically have no
EXIF container; `read_exif_orientation` will return `None` for them, meaning `auto_rotate()` is
always a no-op on PNG and GIF inputs.

`PhpImage` does not need to derive `Default` — it has no PHP-level constructor and ext-php-rs
does not require `Default` for structs that use static factory methods.

### Updated `PhpImage` struct

```rust
pub struct PhpImage {
    pub(crate) inner: ImageInner,
    pub(crate) output_format: Option<OutputFormat>,
    pub(crate) exif_data: Option<HashMap<String, String>>,
    pub(crate) orientation: Option<u32>,
}
```

### Construction

Both `open()` and `from_buffer()` populate these fields after decoding the image. The two new
fields receive the `Option` return values directly — no `unwrap_or`. Failed EXIF parses silently
produce `None`, never an error.

Updated struct construction in `open()`:

```rust
Ok(Self {
    inner,
    output_format: None,
    exif_data: read_exif(&path),
    orientation: read_exif_orientation(&path),
})
```

Updated struct construction in `from_buffer()`:

```rust
Ok(Self {
    inner,
    output_format: None,
    exif_data: read_exif_from_bytes(data),
    orientation: read_exif_orientation_from_bytes(data),
})
```

### Four internal helpers in `image.rs`

| Function | Source |
|----------|--------|
| `read_exif(path: &str) -> Option<HashMap<String, String>>` | existing, unchanged |
| `read_exif_orientation(path: &str) -> Option<u32>` | new — opens file, wraps in `BufReader`, calls `exif::Reader::new().read_from_container()`, finds `exif::Tag::Orientation`, returns `field.value.get_uint(0)` |
| `read_exif_from_bytes(data: &[u8]) -> Option<HashMap<String, String>>` | new — `let mut reader = std::io::BufReader::new(std::io::Cursor::new(data));` then same logic as `read_exif` |
| `read_exif_orientation_from_bytes(data: &[u8]) -> Option<u32>` | new — same `BufReader<Cursor<&[u8]>>` setup, returns orientation integer |

---

## 5. `auto_rotate()`

### API

```php
$img->auto_rotate();
```

### Behavior

- Reads `self.orientation`
- If `None`, `Some(1)`, or any value outside `2–8`: returns `Ok(())` without modifying the image
- Otherwise applies the transform for the given orientation value. The decompositions below are
  authoritative — do not re-derive from the EXIF standard independently, as equivalent
  decompositions exist that produce the same pixel result with different operation order:

  | Value | Visual meaning | Transform (authoritative order) |
  |-------|----------------|--------------------------------|
  | 2     | Mirror horizontal | `self.mirror()?` |
  | 3     | Rotate 180° | Rotate 180° (or equivalently: `self.flip()?; self.mirror()?`) |
  | 4     | Mirror vertical | `self.flip()?` |
  | 5     | Transpose (swap x↔y) | Rotate 90° CW, then `self.mirror()?` |
  | 6     | Rotate 90° CW | Rotate 90° CW |
  | 7     | Anti-transpose | Rotate 90° CCW, then `self.mirror()?` |
  | 8     | Rotate 90° CCW | Rotate 90° CCW |

- All calls to `self.flip()` and `self.mirror()` use `?` to propagate errors. Since those methods
  wrap infallible ril operations in `Ok(...)`, they will never actually error in practice.
- After applying the transform, set `self.orientation = Some(1)` (identity), making repeated calls
  to `auto_rotate()` safe no-ops.
- `exif_data` is **not** modified — it always reflects the original file metadata.
- For animated images: the transform is applied to every frame, using the same per-frame iteration
  pattern as `resize()` and `crop()` (`for frame in seq.iter_mut() { ... frame.image_mut() ... }`).

### Rotation support — static and animated

ril 0.10's rotation API must be verified at implementation time. The same fallback applies to both
static images and individual animation frames:

- If ril has a rotation method: call it per frame (same as `resize()` pattern).
- If ril lacks rotation: fall back to manual pixel transposition using `img.pixel(x, y)` and
  `new_img.set_pixel(dx, dy, val)` (same APIs as `apply_overlay`). Pixel mappings for a W×H source:
  - 90° CW → new image is H×W; `(x, y)` maps to `(H-1-y, x)`
  - 90° CCW → new image is H×W; `(x, y)` maps to `(y, W-1-x)`
  - 180° → same size; `(x, y)` maps to `(W-1-x, H-1-y)`

### Signature

```rust
pub fn auto_rotate(&mut self) -> Result<(), ImageError>
```

`Result` is used for consistency with all other `PhpImage` mutation methods and to allow rotation
errors to surface as `ImageException` on the PHP side.

---

## 6. `flip()` and `mirror()`

### API

```php
$img->flip();   // vertical flip: top <-> bottom
$img->mirror(); // horizontal flip: left <-> right
```

### Behavior

- Both mutate in place
- For animated images: transform applied to each frame
- No parameters
- Both return `Result<(), ImageError>` for uniformity with all other `PhpImage` mutation methods.
  If ril's methods are infallible the implementation wraps them in `Ok(...)`.

### ril method names

Verify the exact method names on `ril::Image<Rgba>` at implementation time. The expected names
based on ril's API design are `flip` (vertical) and `mirror` (horizontal), matching the PHP method
names in this spec. If the actual ril method names differ, use them internally and keep the PHP
method names as specified here.

### Signatures

```rust
pub fn flip(&mut self) -> Result<(), ImageError>
pub fn mirror(&mut self) -> Result<(), ImageError>
```

---

## File changes summary

| File | Change |
|------|--------|
| `src/rgb.rs` | New — `PhpRgb` class; imports `use ril::Rgba` |
| `src/image.rs` | Add `exif_data` + `orientation` fields to `PhpImage`; add `create`, `crop`, `auto_rotate`, `flip`, `mirror`; add `read_exif_orientation`, `read_exif_from_bytes`, `read_exif_orientation_from_bytes`; update `open` and `from_buffer` struct construction; add `use crate::rgb::PhpRgb` |
| `src/lib.rs` | Register `PhpRgb` class |

---

## Error handling

All methods throw `RustImage\ImageException` (the existing `ImageException` type) on invalid input
or internal failure. No new error types are introduced.
