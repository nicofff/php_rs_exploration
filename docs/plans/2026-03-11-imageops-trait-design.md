# Design: ImageOps trait with Static/Animated split

## Goal

Simplify `PhpImage` to a thin PHP bridge with no image logic. Move all manipulation into two concrete types that implement a shared trait, dispatched statically via an enum.

## Motivation

The current `Vec<image::Frame>` representation forces every static image operation to wrap/unwrap a `Frame` around a `DynamicImage`. Since the `image` crate's `DynamicImage` already provides `resize`, `crop`, `grayscale` etc., the static path should be a direct call with no intermediate types. Animated images genuinely require `Vec<image::Frame>` and are a special case.

## Core Types

### Trait

```rust
pub(crate) trait ImageOps {
    fn resize(&mut self, width: u32, height: u32, fit: &str) -> Result<(), ImageError>;
    fn thumbnail(&mut self, width: u32, height: u32);
    fn crop(&mut self, x: u32, y: u32, width: u32, height: u32) -> Result<(), ImageError>;
    fn grayscale(&mut self);
    fn overlay(&mut self, overlay: &DynamicImage, x: i32, y: i32, opacity: f32);
    fn dimensions(&self) -> (u32, u32);
    fn encode(&self, format: OutputFormat) -> Result<Vec<u8>, ImageError>;
    fn first_frame(&self) -> DynamicImage;
}
```

### Structs

```rust
pub(crate) struct StaticImage(DynamicImage);
pub(crate) struct AnimatedImage(Vec<image::Frame>);
```

`StaticImage` operations are single `DynamicImage` method calls.
`AnimatedImage` operations iterate frames.

### PhpImage

```rust
pub(crate) enum ImageInner {
    Static(StaticImage),
    Animated(AnimatedImage),
}

pub struct PhpImage {
    inner: ImageInner,
    output_format: Option<OutputFormat>,
}
```

## Data Flow

- `open` (static): `ImageReader::open().decode()` → `StaticImage(img)`
- `open` (gif): `GifDecoder::into_frames().collect_frames()` → `AnimatedImage(frames)`
- `from_buffer`: `image::load_from_memory()` → `StaticImage(img)`
- Operations: `PhpImage` matches on `inner`, delegates to the trait method
- Overlay: `other.inner.first_frame()` extracts a `DynamicImage`, passed into `self.inner.overlay(...)`
- Encode: `self.inner.encode(format)` — static GIF produces a single-frame GIF

## File Structure

- `src/image.rs` — `PhpImage`, `ImageInner`, `OutputFormat`
- `src/image_static.rs` — `StaticImage` + `ImageOps` impl
- `src/image_animated.rs` — `AnimatedImage` + `ImageOps` impl
- `src/image_ops_trait.rs` — `ImageOps` trait definition
- `src/image_ops.rs` — `overlay_frame` helper (shared by both impls)
- `src/image_encode.rs` — WebP/AVIF encode helpers (unchanged)
- `src/image_decode.rs` — `is_gif` (unchanged)
