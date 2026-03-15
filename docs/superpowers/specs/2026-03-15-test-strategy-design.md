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
├── composer.json              # require-dev: phpunit/phpunit
├── composer.lock
├── phpunit.xml                # suite config; no hardcoded extension path
├── Makefile                   # build, test, phptest targets
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
│       ├── animated.gif       # existing
│       └── exif_rotated.jpg   # existing
└── src/
    ├── image.rs               # + #[cfg(test)] module for Rust unit tests
    └── ...
```

---

## PHP Tests (PHPUnit)

### Setup

- `composer.json` with `"phpunit/phpunit": "^11"` under `require-dev`
- `phpunit.xml` points bootstrap at `tests/bootstrap.php` and scans `tests/` for `*Test.php` files
- No coverage configuration (no Xdebug requirement)

### Extension Loading

The compiled extension path is **not** hardcoded in `phpunit.xml`. The Makefile resolves the path and passes it at invocation time:

```makefile
EXT_PATH := target/release/libphprs_hello_world.dylib

phptest:
    php -d extension=$(EXT_PATH) vendor/bin/phpunit

test: build phptest
```

### bootstrap.php

Checks that `RustImage\Image` class exists. If not, prints a clear diagnostic and calls `exit(1)` so the suite fails immediately with a useful message rather than a cascade of class-not-found errors.

### Test Class Conventions

- Each class extends `PHPUnit\Framework\TestCase`
- Shared fixtures (temp files) created in `setUpBeforeClass()`, removed in `tearDownAfterClass()`
- Assertions use `assertSame()`, `assertGreaterThan()`, etc. — no bare `assert()`
- Exception tests use `$this->expectException(RustImage\ImageException::class)`
- Pixel color assertions extracted into `PixelAssertions` trait (wraps GD `imagecolorat`)

### Test Classes & Coverage

| Class | Covers |
|---|---|
| `RgbTest` | Constructor, r/g/b getters, boundary values |
| `ImageOpenTest` | `open()` success, missing file error, `info()` dimensions/format, `fromBuffer()`, corrupt data |
| `CanvasTest` | `create()` dimensions, fill color, zero/negative dimension errors |
| `ResizeTest` | contain (aspect ratio), cover (exact), fill (stretch), thumbnail, JPEG output |
| `CropTest` | Basic crop dimensions, offset crop color, out-of-bounds error, negative x/y error, animated GIF crop |
| `TransformTest` | `flip()` pixel position, `mirror()` pixel position, `autoRotate()` no-EXIF no-op, orientation 6 rotation, idempotency |
| `EncodeTest` | `toPng`, `toJpeg` with quality, `toWebp`, `toGif`, `toAvif`, `toBuffer` round-trip, `save` |
| `OverlayTest` | Overlay preserves base dimensions, opacity parameter |

---

## Rust Unit Tests

Added as `#[cfg(test)]` modules inside the relevant source files. These run via `cargo test` with no PHP runtime.

### `src/image.rs` — `#[cfg(test)]`

- `compute_fit`: contain/cover/fill modes, edge cases (1×1, non-square, equal dimensions)
- `crop_image`: output dimensions match requested size
- `read_exif_orientation_from_bytes`: returns `None` for non-JPEG bytes

---

## Makefile Targets

```makefile
build:
    cargo build --release

phptest:
    php -d extension=$(EXT_PATH) vendor/bin/phpunit

rusttest:
    cargo test

test: build rusttest phptest
```

---

## Migration Plan

1. Add `composer.json`, run `composer install`
2. Add `phpunit.xml` and `tests/bootstrap.php`
3. Add `PixelAssertions` trait
4. Add PHPUnit test classes, migrating coverage from old scripts
5. Add Rust `#[cfg(test)]` modules to `src/image.rs`
6. Add `Makefile`
7. Delete `test.php`, `test_image.php`, `test_gif_resize.php`, `test_redis.php`, `tests/test_new_api.php`
8. Verify `make test` passes end-to-end

---

## Out of Scope

- Code coverage reporting (no Xdebug requirement)
- CI/CD pipeline
- `thumbnail()` Rust unit test (requires PHP runtime)
- Animated WebP encode test (deferred — encoding path exists but is not tested by any existing script)
