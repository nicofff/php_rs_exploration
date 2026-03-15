# Test Strategy Design

**Date:** 2026-03-15
**Project:** phprs_hello_world — Rust-backed PHP image extension

---

## Context

The project is a PHP extension written in Rust (`ext-php-rs`), exposing image manipulation under the `RustImage\` namespace. At the time of this spec, tests exist as a collection of ad-hoc PHP scripts (`test.php`, `test_image.php`, `test_gif_resize.php`, `test_redis.php`, `tests/test_new_api.php`) that use bare `assert()`, have no runner, overlap in coverage, and include a script (`test_redis.php`) for a removed feature.

---

## Goals

- Replace scattered scripts with a structured, runnable test suite
- Use PHPUnit as the PHP test framework (via Composer)
- Add Rust unit tests for pure-Rust logic
- Single command (`make test`) runs everything
- Delete old root-level `test*.php` scripts

---

## Directory Structure

```
phprs_hello_world/
├── composer.json              # require-dev: phpunit/phpunit; PSR-4 autoload for tests/
├── composer.lock
├── phpunit.xml                # suite config; no hardcoded extension path
├── Makefile                   # new file; build, rusttest, phptest, test targets
├── tests/
│   ├── bootstrap.php          # fail-fast if extension not loaded
│   ├── PixelAssertions.php    # trait: GD-based pixel color helpers
│   ├── ImageOpenTest.php      # open(), info(), fromBuffer(), error cases
│   ├── ResizeTest.php         # resize() contain/cover/fill, thumbnail()
│   ├── CropTest.php           # crop() static + animated GIF
│   ├── TransformTest.php      # flip(), mirror(), autoRotate()
│   ├── CanvasTest.php         # create()
│   ├── EncodeTest.php         # toPng/Jpeg/Webp/Gif/Avif, toBuffer/save
│   ├── OverlayTest.php        # overlay()
│   ├── RgbTest.php            # RustImage\Rgb
│   └── fixtures/
│       ├── animated.gif       # existing committed binary — multi-frame GIF
│       └── exif_rotated.jpg   # existing committed binary — 100×50 JPEG, EXIF orientation=6
└── src/
    ├── image.rs               # + #[cfg(test)] module for Rust unit tests
    └── ...
```

**Fixtures:** `animated.gif` and `exif_rotated.jpg` are already committed to the repository. No new fixture files need to be created. Tests that require images with predictable pixel colors synthesize them at runtime using GD in `setUpBeforeClass()` and delete them in `tearDownAfterClass()`.

---

## PHP API Reference

| Method | Signature | Notes |
|---|---|---|
| `Image::open` | `static open(string $path): Image` | Throws `ImageException` if file missing or corrupt |
| `Image::fromBuffer` | `static fromBuffer(string $bytes): Image` | Throws on corrupt bytes; does NOT preserve EXIF |
| `Image::info` | `static info(string $path): ImageInfo` | |
| `Image::create` | `static create(int $width, int $height, Rgb $color): Image` | Throws on zero/negative dims |
| `Image::resize` | `resize(int $width, int $height, string $fit = 'contain'): void` | fit: contain/cover/fill |
| `Image::crop` | `crop(int $x, int $y, int $width, int $height): void` | Throws on negative coords or out-of-bounds |
| `Image::flip` | `flip(): void` | Vertical flip (about x-axis) |
| `Image::mirror` | `mirror(): void` | Horizontal flip (about y-axis) |
| `Image::autoRotate` | `autoRotate(): void` | No-op if orientation is absent or already 1 |
| `Image::overlay` | `overlay(Image $other, int $x, int $y, float $opacity = 1.0): void` | `$other` read-only, not mutated |
| `Image::toPng` | `toPng(): void` | Sets output format |
| `Image::toJpeg` | `toJpeg(int $quality = 85): void` | |
| `Image::toWebp` | `toWebp(int $quality = 80): void` | |
| `Image::toGif` | `toGif(): void` | |
| `Image::toBuffer` | `toBuffer(): string` | Returns encoded bytes; does NOT embed EXIF in output |
| `Image::save` | `save(string $path): void` | Writes encoded bytes; does NOT embed EXIF in output |
| `Rgb::__construct` | `__construct(int $r, int $g, int $b)` | |
| `ImageInfo` | `width: int`, `height: int`, `format: string`, `is_animated: bool`, `exif: ?array<string,string>` | `exif` is a flat map of EXIF tag name → display string (e.g. `["Orientation" => "right-top"]`); `null` if no EXIF |

**EXIF encoding note:** The encoder does not write EXIF metadata into output bytes. Images produced by `toBuffer()` or `save()` and then re-opened via `fromBuffer()` or `open()` will have `orientation = null`. This is the property that makes the `autoRotate()` idempotency test work.

---

## PHP Tests (PHPUnit)

### composer.json

```json
{
    "name": "phprs/hello-world",
    "require": {},
    "require-dev": {
        "phpunit/phpunit": "^11"
    },
    "autoload-dev": {
        "psr-4": {
            "Tests\\": "tests/"
        }
    },
    "minimum-stability": "stable"
}
```

No `scripts.test` entry — to avoid developers running `composer test` without the `-d extension=` flag. The canonical test command is `make test`.

### phpunit.xml

```xml
<?xml version="1.0" encoding="UTF-8"?>
<phpunit xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:noNamespaceSchemaLocation="vendor/phpunit/phpunit/phpunit.xsd"
         bootstrap="tests/bootstrap.php"
         colors="true">
    <testsuites>
        <testsuite name="RustImage">
            <directory>tests</directory>
        </testsuite>
    </testsuites>
</phpunit>
```

No coverage configuration — no Xdebug requirement.

### Extension Loading

The Makefile passes `-d extension=$(EXT_PATH)` to PHP. `EXT_PATH` is resolved from the platform:

```makefile
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
    EXT_SUFFIX := dylib
else ifeq ($(UNAME_S),Linux)
    EXT_SUFFIX := so
else
    EXT_SUFFIX := dll
endif
EXT_PATH := target/release/libphprs_hello_world.$(EXT_SUFFIX)
```

Windows listed for completeness; not a current goal.

### bootstrap.php

```php
<?php
if (!class_exists('RustImage\Image')) {
    fwrite(STDERR, "ERROR: RustImage extension not loaded.\n");
    fwrite(STDERR, "Run via: make phptest  (passes -d extension=... automatically)\n");
    exit(1);
}
```

### Test Class Conventions

- All classes in namespace `Tests\`, file `tests/<ClassName>.php`
- Each class extends `PHPUnit\Framework\TestCase`
- Shared fixtures (synthesized temp files) created in `setUpBeforeClass()`, removed in `tearDownAfterClass()`
- Individual tests use `assertSame()`, `assertGreaterThan()`, etc. — no bare `assert()`
- Exception tests use `$this->expectException(\RustImage\ImageException::class)`
- Pixel color assertions use the `PixelAssertions` trait

### PixelAssertions Trait

File: `tests/PixelAssertions.php`, namespace `Tests\`.

```php
<?php
namespace Tests;

trait PixelAssertions
{
    /** Assert that pixel ($x, $y) in the PNG at $path has the given RGB values (±$tolerance each). */
    public function assertPixelColor(string $path, int $x, int $y, int $r, int $g, int $b, int $tolerance = 5): void { ... }
}
```

Used by: `CanvasTest`, `CropTest`, `TransformTest`, `OverlayTest`.

### Test Classes & Coverage

| Class | Covers |
|---|---|
| `RgbTest` | Constructor, r/g/b getters, boundary values (0, 255) |
| `ImageOpenTest` | `open()` success, missing file error, `info()` dimensions/format/is_animated, `fromBuffer()`, corrupt data |
| `CanvasTest` | `create()` dimensions, fill color via `assertPixelColor`, zero/negative dimension errors |
| `ResizeTest` | contain (aspect ratio preserved), cover (exact size), fill (stretch), JPEG output format |
| `CropTest` | Basic crop dimensions, offset crop origin color via `assertPixelColor`, out-of-bounds error, negative x/y error, animated GIF crop dimensions |
| `TransformTest` | `flip()` pixel moves from (0,0)→(0,h-1); `mirror()` pixel moves (0,0)→(w-1,0); `autoRotate()` no-EXIF no-op; orientation-6 rotation (50×100); idempotency (see flow below) |
| `EncodeTest` | `toPng`, `toJpeg` with quality, `toWebp`, `toGif`, `toBuffer`→`fromBuffer` round-trip (re-opened image has correct dimensions), `save`→`info()` confirms format and dimensions |
| `OverlayTest` | Preserves base dimensions; pixel at overlay position changes color via `assertPixelColor`; `$other` unchanged after call |

**TransformTest — autoRotate idempotency flow:**
1. Open `tests/fixtures/exif_rotated.jpg` (100×50, EXIF orientation=6)
2. Call `autoRotate()` → image becomes 50×100; orientation internally reset to `Some(1)`
3. Call `autoRotate()` again on the **same object** — `Some(1)` matches the early-return branch, so this is a no-op
4. Assert dimensions remain 50×100

---

## Rust Unit Tests

Added as `#[cfg(test)]` modules inside the relevant source files. These run via `cargo test` with no PHP runtime.

**Cargo.toml prerequisite:** The current `crate-type = ["cdylib"]` does not produce a test binary. `cargo test` requires the `rlib` crate type. Migration step 6 must also change `Cargo.toml` to `crate-type = ["cdylib", "rlib"]`.

### `src/image.rs` — `#[cfg(test)]`

- **`compute_fit`**: all three modes (contain/cover/fill), edge cases (1×1 source, non-square source, equal dimensions)
- **`crop_image`**: create a test `Image<Rgba>` using `Image::new(w, h, Rgba { r:0, g:0, b:0, a:255 })` then `set_pixel(x, y, known_color)`; crop starting at (x, y); assert output width/height equals crop size and `pixel(0,0)` equals `known_color`
- **`read_exif_orientation_from_bytes`**: pass arbitrary non-JPEG bytes (e.g. `b"not jpeg"`); assert result is `None`

---

## Makefile

This is a **new file** (no existing Makefile in the project).

```makefile
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
    EXT_SUFFIX := dylib
else ifeq ($(UNAME_S),Linux)
    EXT_SUFFIX := so
else
    EXT_SUFFIX := dll
endif
EXT_PATH := target/release/libphprs_hello_world.$(EXT_SUFFIX)

build:
	cargo build --release

rusttest:
	cargo test

phptest:
	php -d extension=$(EXT_PATH) vendor/bin/phpunit

test: build rusttest phptest

.PHONY: build rusttest phptest test
```

`test` runs all three in order: `build` first (compiles the extension), then `rusttest` (pure Rust tests, no PHP needed), then `phptest` (PHPUnit with the freshly built extension). `phptest` depends on `build` having completed because it loads the compiled artifact.

---

## Migration Plan

1. Add `composer.json` as specified above, run `composer install`
2. Add `phpunit.xml` as specified above
3. Add `tests/bootstrap.php`
4. Add `tests/PixelAssertions.php` trait
5. Add PHPUnit test classes, migrating coverage from old scripts
6. Add Rust `#[cfg(test)]` modules to `src/image.rs`
7. Add `Makefile` (new file)
8. Delete `test.php`, `test_image.php`, `test_gif_resize.php`, `test_redis.php`, `tests/test_new_api.php`
9. Verify `make test` passes end-to-end

---

## Out of Scope

- Code coverage reporting (no Xdebug requirement)
- CI/CD pipeline
- `thumbnail()` — not yet implemented in the Rust extension
- `toAvif()` — not yet implemented in the Rust extension
- Animated WebP encode test (encoding path exists but no existing test coverage)
- Windows build support (`.dll` path detected but not validated)
