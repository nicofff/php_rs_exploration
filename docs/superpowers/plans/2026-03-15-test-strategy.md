# Test Strategy Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace ad-hoc PHP test scripts with a PHPUnit suite plus Rust unit tests, runnable via `make test`.

**Architecture:** Composer manages PHPUnit as a dev dependency; tests live under `tests/` as PSR-4 classes in the `Tests\` namespace; a platform-detecting Makefile wires `cargo build`, `cargo test`, and `phpunit` together. Rust pure-logic functions get `#[cfg(test)]` units in `src/image.rs`.

**Tech Stack:** PHP 8.2+, PHPUnit 11, Composer, Rust/Cargo, ext-php-rs, GD (for fixture synthesis in tests), GNU Make.

**Spec:** `docs/superpowers/specs/2026-03-15-test-strategy-design.md`

---

## Chunk 1: Scaffold

### Task 1: Add composer.json

**Files:**
- Create: `composer.json`

- [ ] **Step 1: Write composer.json**

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

- [ ] **Step 2: Install dependencies**

```bash
composer install
```

Expected: `vendor/` directory created, `composer.lock` written, `vendor/bin/phpunit` available.

- [ ] **Step 3: Commit**

```bash
git add composer.json composer.lock
git commit -m "chore: add composer.json with phpunit dev dependency"
```

---

### Task 2: Add phpunit.xml

**Files:**
- Create: `phpunit.xml`

- [ ] **Step 1: Write phpunit.xml**

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

- [ ] **Step 2: Commit**

```bash
git add phpunit.xml
git commit -m "chore: add phpunit.xml"
```

---

### Task 3: Add Makefile

**Files:**
- Create: `Makefile`

- [ ] **Step 1: Write Makefile**

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

Note: the indented lines use a **tab character**, not spaces. Make requires tabs.

- [ ] **Step 2: Verify build target works**

```bash
make build
```

Expected: `Compiling phprs_hello_world ...` → `Finished release`

- [ ] **Step 3: Commit**

```bash
git add Makefile
git commit -m "chore: add Makefile with build/rusttest/phptest/test targets"
```

---

### Task 4: Enable Rust unit tests (Cargo.toml)

**Files:**
- Modify: `Cargo.toml`

A `cdylib`-only crate does not produce a test binary. Adding `rlib` enables `cargo test`.

- [ ] **Step 1: Edit Cargo.toml**

Change:
```toml
[lib]
crate-type = ["cdylib"]
```

To:
```toml
[lib]
crate-type = ["cdylib", "rlib"]
```

- [ ] **Step 2: Verify cargo test compiles (no tests yet)**

```bash
cargo test
```

Expected: `running 0 tests` — no failures.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: add rlib crate-type to enable cargo test"
```

---

### Task 5: Add bootstrap.php

**Files:**
- Create: `tests/bootstrap.php`

- [ ] **Step 1: Write bootstrap.php**

```php
<?php
declare(strict_types=1);

if (!class_exists('RustImage\Image')) {
    fwrite(STDERR, "ERROR: RustImage extension not loaded.\n");
    fwrite(STDERR, "Run via: make phptest  (passes -d extension=... automatically)\n");
    exit(1);
}
```

- [ ] **Step 2: Commit**

```bash
git add tests/bootstrap.php
git commit -m "test: add phpunit bootstrap with extension load check"
```

---

## Chunk 2: Foundation

### Task 6: Add PixelAssertions trait

**Files:**
- Create: `tests/PixelAssertions.php`

Used by `CanvasTest`, `CropTest`, `TransformTest`, `OverlayTest` to read pixel colors from a saved PNG via GD.

- [ ] **Step 1: Write PixelAssertions.php**

```php
<?php
declare(strict_types=1);

namespace Tests;

trait PixelAssertions
{
    /**
     * Assert that pixel ($x, $y) in the PNG at $path has the given RGB values (±$tolerance each).
     */
    public function assertPixelColor(
        string $path,
        int $x,
        int $y,
        int $r,
        int $g,
        int $b,
        int $tolerance = 5
    ): void {
        $gd = imagecreatefrompng($path);
        $this->assertNotFalse($gd, "Failed to open PNG: $path");
        $pixel = imagecolorat($gd, $x, $y);
        imagedestroy($gd);

        $pr = ($pixel >> 16) & 0xFF;
        $pg = ($pixel >> 8) & 0xFF;
        $pb = $pixel & 0xFF;

        $this->assertEqualsWithDelta($r, $pr, $tolerance, "Red mismatch at ($x,$y) in $path: expected $r got $pr");
        $this->assertEqualsWithDelta($g, $pg, $tolerance, "Green mismatch at ($x,$y) in $path: expected $g got $pg");
        $this->assertEqualsWithDelta($b, $pb, $tolerance, "Blue mismatch at ($x,$y) in $path: expected $b got $pb");
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add tests/PixelAssertions.php
git commit -m "test: add PixelAssertions trait for GD-based pixel color assertions"
```

---

### Task 7: RgbTest

**Files:**
- Create: `tests/RgbTest.php`

- [ ] **Step 1: Write RgbTest.php**

```php
<?php
declare(strict_types=1);

namespace Tests;

use PHPUnit\Framework\TestCase;
use RustImage\Rgb;

class RgbTest extends TestCase
{
    public function testConstructorAndGetters(): void
    {
        $rgb = new Rgb(100, 150, 200);
        $this->assertSame(100, $rgb->r);
        $this->assertSame(150, $rgb->g);
        $this->assertSame(200, $rgb->b);
    }

    public function testBlackBoundary(): void
    {
        $rgb = new Rgb(0, 0, 0);
        $this->assertSame(0, $rgb->r);
        $this->assertSame(0, $rgb->g);
        $this->assertSame(0, $rgb->b);
    }

    public function testWhiteBoundary(): void
    {
        $rgb = new Rgb(255, 255, 255);
        $this->assertSame(255, $rgb->r);
        $this->assertSame(255, $rgb->g);
        $this->assertSame(255, $rgb->b);
    }
}
```

- [ ] **Step 2: Run the test**

```bash
make phptest
```

Expected: `3 / 3 (100%)` — all pass. (The Rgb implementation is already present.)

- [ ] **Step 3: Commit**

```bash
git add tests/RgbTest.php
git commit -m "test: add RgbTest"
```

---

### Task 8: ImageOpenTest

**Files:**
- Create: `tests/ImageOpenTest.php`

- [ ] **Step 1: Write ImageOpenTest.php**

```php
<?php
declare(strict_types=1);

namespace Tests;

use PHPUnit\Framework\TestCase;
use RustImage\Image;
use RustImage\ImageException;

class ImageOpenTest extends TestCase
{
    private static string $tmpJpeg;

    public static function setUpBeforeClass(): void
    {
        self::$tmpJpeg = sys_get_temp_dir() . '/rustimage_open_test.jpg';
        $gd = imagecreatetruecolor(200, 150);
        imagefill($gd, 0, 0, imagecolorallocate($gd, 255, 255, 255));
        imagejpeg($gd, self::$tmpJpeg, 90);
        imagedestroy($gd);
    }

    public static function tearDownAfterClass(): void
    {
        @unlink(self::$tmpJpeg);
    }

    public function testOpenSuccess(): void
    {
        $img = Image::open(self::$tmpJpeg);
        $this->assertInstanceOf(Image::class, $img);
    }

    public function testOpenMissingFileThrows(): void
    {
        $this->expectException(ImageException::class);
        Image::open('/tmp/this_file_does_not_exist_rustimage_test.jpg');
    }

    public function testOpenCorruptDataThrows(): void
    {
        $corrupt = sys_get_temp_dir() . '/rustimage_corrupt_test.jpg';
        file_put_contents($corrupt, 'not a real image');
        try {
            $this->expectException(ImageException::class);
            Image::open($corrupt);
        } finally {
            @unlink($corrupt);
        }
    }

    public function testInfoDimensions(): void
    {
        $info = Image::info(self::$tmpJpeg);
        $this->assertSame(200, $info->width);
        $this->assertSame(150, $info->height);
    }

    public function testInfoFormat(): void
    {
        $info = Image::info(self::$tmpJpeg);
        $this->assertSame('jpeg', $info->format);
    }

    public function testInfoIsAnimatedFalseForJpeg(): void
    {
        $info = Image::info(self::$tmpJpeg);
        $this->assertFalse($info->is_animated);
    }

    public function testFromBuffer(): void
    {
        $bytes = file_get_contents(self::$tmpJpeg);
        $img = Image::fromBuffer($bytes);
        $this->assertInstanceOf(Image::class, $img);
    }

    public function testFromBufferCorruptThrows(): void
    {
        $this->expectException(ImageException::class);
        Image::fromBuffer('not an image');
    }
}
```

- [ ] **Step 2: Run the tests**

```bash
make phptest
```

Expected: all pass. (8 tests in this class + 3 from RgbTest = 11 total)

- [ ] **Step 3: Commit**

```bash
git add tests/ImageOpenTest.php
git commit -m "test: add ImageOpenTest"
```

---

## Chunk 3: Canvas, Resize, Crop

### Task 9: CanvasTest

**Files:**
- Create: `tests/CanvasTest.php`

- [ ] **Step 1: Write CanvasTest.php**

```php
<?php
declare(strict_types=1);

namespace Tests;

use PHPUnit\Framework\TestCase;
use RustImage\Image;
use RustImage\ImageException;
use RustImage\Rgb;

class CanvasTest extends TestCase
{
    use PixelAssertions;

    private function tmp(string $suffix = ''): string
    {
        return sys_get_temp_dir() . '/rustimage_canvas_' . $suffix . '.png';
    }

    public function testCreateDimensions(): void
    {
        $tmp = $this->tmp('dims');
        $img = Image::create(120, 80, new Rgb(0, 0, 255));
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(120, $info->width);
        $this->assertSame(80, $info->height);
        unlink($tmp);
    }

    public function testCreateFillColorAtMultiplePoints(): void
    {
        $tmp = $this->tmp('fill');
        $img = Image::create(50, 50, new Rgb(255, 128, 0));
        $img->toPng();
        $img->save($tmp);
        $this->assertPixelColor($tmp, 0, 0, 255, 128, 0);
        $this->assertPixelColor($tmp, 25, 25, 255, 128, 0);
        $this->assertPixelColor($tmp, 49, 49, 255, 128, 0);
        unlink($tmp);
    }

    public function testCreateZeroWidthThrows(): void
    {
        $this->expectException(ImageException::class);
        Image::create(0, 100, new Rgb(0, 0, 0));
    }

    public function testCreateZeroHeightThrows(): void
    {
        $this->expectException(ImageException::class);
        Image::create(100, 0, new Rgb(0, 0, 0));
    }

    public function testCreateNegativeWidthThrows(): void
    {
        $this->expectException(ImageException::class);
        Image::create(-1, 100, new Rgb(0, 0, 0));
    }

    public function testCreateNegativeHeightThrows(): void
    {
        $this->expectException(ImageException::class);
        Image::create(100, -1, new Rgb(0, 0, 0));
    }
}
```

- [ ] **Step 2: Run the tests**

```bash
make phptest
```

Expected: all pass.

- [ ] **Step 3: Commit**

```bash
git add tests/CanvasTest.php
git commit -m "test: add CanvasTest"
```

---

### Task 10: ResizeTest

**Files:**
- Create: `tests/ResizeTest.php`

- [ ] **Step 1: Write ResizeTest.php**

```php
<?php
declare(strict_types=1);

namespace Tests;

use PHPUnit\Framework\TestCase;
use RustImage\Image;

class ResizeTest extends TestCase
{
    private static string $tmpSrc;

    public static function setUpBeforeClass(): void
    {
        // 200x150 source image
        self::$tmpSrc = sys_get_temp_dir() . '/rustimage_resize_src.jpg';
        $gd = imagecreatetruecolor(200, 150);
        imagefill($gd, 0, 0, imagecolorallocate($gd, 200, 200, 200));
        imagejpeg($gd, self::$tmpSrc, 95);
        imagedestroy($gd);
    }

    public static function tearDownAfterClass(): void
    {
        @unlink(self::$tmpSrc);
    }

    public function testContainPreservesAspectRatio(): void
    {
        // 200x150 contained in 100x100 → 100x75
        $tmp = sys_get_temp_dir() . '/rustimage_resize_contain.png';
        $img = Image::open(self::$tmpSrc);
        $img->resize(100, 100); // 'contain' is the default
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(100, $info->width);
        $this->assertSame(75, $info->height);
        unlink($tmp);
    }

    public function testCoverProducesExactSize(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_resize_cover.png';
        $img = Image::open(self::$tmpSrc);
        $img->resize(100, 100, fit: 'cover');
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(100, $info->width);
        $this->assertSame(100, $info->height);
        unlink($tmp);
    }

    public function testFillProducesExactSize(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_resize_fill.png';
        $img = Image::open(self::$tmpSrc);
        $img->resize(100, 100, fit: 'fill');
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(100, $info->width);
        $this->assertSame(100, $info->height);
        unlink($tmp);
    }

    public function testJpegOutputFormat(): void
    {
        // 200×150 contain into 80×60: scale = min(80/200, 60/150) = 0.4, output = 80×60
        $tmp = sys_get_temp_dir() . '/rustimage_resize_out.jpg';
        $img = Image::open(self::$tmpSrc);
        $img->resize(80, 60);
        $img->toJpeg(quality: 70);
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame('jpeg', $info->format);
        $this->assertSame(80, $info->width);
        $this->assertSame(60, $info->height);
        $this->assertGreaterThan(0, filesize($tmp));
        unlink($tmp);
    }
}
```

- [ ] **Step 2: Run the tests**

```bash
make phptest
```

Expected: all pass.

- [ ] **Step 3: Commit**

```bash
git add tests/ResizeTest.php
git commit -m "test: add ResizeTest"
```

---

### Task 11: CropTest

**Files:**
- Create: `tests/CropTest.php`

- [ ] **Step 1: Write CropTest.php**

```php
<?php
declare(strict_types=1);

namespace Tests;

use PHPUnit\Framework\TestCase;
use RustImage\Image;
use RustImage\ImageException;
use RustImage\Rgb;

class CropTest extends TestCase
{
    use PixelAssertions;

    private static string $tmpSrc;

    public static function setUpBeforeClass(): void
    {
        // 200x150 white image with a 50x50 red square at top-left (0,0)–(49,49)
        self::$tmpSrc = sys_get_temp_dir() . '/rustimage_crop_src.jpg';
        $gd = imagecreatetruecolor(200, 150);
        imagefill($gd, 0, 0, imagecolorallocate($gd, 255, 255, 255));
        imagefilledrectangle($gd, 0, 0, 49, 49, imagecolorallocate($gd, 255, 0, 0));
        imagejpeg($gd, self::$tmpSrc, 100);
        imagedestroy($gd);
    }

    public static function tearDownAfterClass(): void
    {
        @unlink(self::$tmpSrc);
    }

    public function testCropDimensions(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_crop_dims.png';
        $img = Image::open(self::$tmpSrc);
        $img->crop(0, 0, 100, 80);
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(100, $info->width);
        $this->assertSame(80, $info->height);
        unlink($tmp);
    }

    public function testCropOffsetExcludesRedSquare(): void
    {
        // Crop starting at x=100 — the red square only occupies x=0..49
        $tmp = sys_get_temp_dir() . '/rustimage_crop_offset.png';
        $img = Image::open(self::$tmpSrc);
        $img->crop(100, 0, 100, 150);
        $img->toPng();
        $img->save($tmp);
        // pixel at (0,0) in the cropped image was at (100,0) in the source — should be white
        $this->assertPixelColor($tmp, 0, 0, 255, 255, 255);
        unlink($tmp);
    }

    public function testCropOutOfBoundsThrows(): void
    {
        $img = Image::create(200, 150, new Rgb(255, 255, 255));
        $this->expectException(ImageException::class);
        $img->crop(0, 0, 300, 300);
    }

    public function testCropNegativeXThrows(): void
    {
        $img = Image::create(200, 150, new Rgb(255, 255, 255));
        $this->expectException(ImageException::class);
        $img->crop(-1, 0, 100, 100);
    }

    public function testCropNegativeYThrows(): void
    {
        $img = Image::create(200, 150, new Rgb(255, 255, 255));
        $this->expectException(ImageException::class);
        $img->crop(0, -1, 100, 100);
    }

    public function testAnimatedGifCropDimensions(): void
    {
        $fixture = __DIR__ . '/fixtures/animated.gif';
        $tmp = sys_get_temp_dir() . '/rustimage_crop_anim.gif';
        $gifInfo = Image::info($fixture);
        $cropW = (int)($gifInfo->width / 2);
        $cropH = (int)($gifInfo->height / 2);

        $gif = Image::open($fixture);
        $gif->crop(0, 0, $cropW, $cropH);
        $gif->toGif();
        $gif->save($tmp);

        $outInfo = Image::info($tmp);
        $this->assertSame($cropW, $outInfo->width);
        $this->assertSame($cropH, $outInfo->height);
        unlink($tmp);
    }
}
```

- [ ] **Step 2: Run the tests**

```bash
make phptest
```

Expected: all pass.

- [ ] **Step 3: Commit**

```bash
git add tests/CropTest.php
git commit -m "test: add CropTest"
```

---

## Chunk 4: Transform, Encode, Overlay

### Task 12: TransformTest

**Files:**
- Create: `tests/TransformTest.php`

- [ ] **Step 1: Write TransformTest.php**

```php
<?php
declare(strict_types=1);

namespace Tests;

use PHPUnit\Framework\TestCase;
use RustImage\Image;
use RustImage\Rgb;

class TransformTest extends TestCase
{
    use PixelAssertions;

    /**
     * Create a 10×10 white PNG with a single red pixel at (0, 0).
     * Returns the path to the saved file.
     */
    private static function makeRedDotImage(): string
    {
        $path = sys_get_temp_dir() . '/rustimage_transform_src_' . uniqid() . '.png';
        $gd = imagecreatetruecolor(10, 10);
        imagefill($gd, 0, 0, imagecolorallocate($gd, 255, 255, 255));
        imagesetpixel($gd, 0, 0, imagecolorallocate($gd, 255, 0, 0));
        imagepng($gd, $path);
        imagedestroy($gd);
        return $path;
    }

    public function testFlipMovesRedPixelToBottomLeft(): void
    {
        $src = self::makeRedDotImage(); // red at (0,0), white elsewhere
        $out = sys_get_temp_dir() . '/rustimage_flip_out.png';

        $img = Image::open($src);
        $img->flip(); // vertical flip: row 0 ↔ row 9
        $img->toPng();
        $img->save($out);

        $this->assertPixelColor($out, 0, 9, 255, 0, 0);   // red moved to (0,9)
        $this->assertPixelColor($out, 0, 0, 255, 255, 255); // (0,0) is now white

        unlink($src);
        unlink($out);
    }

    public function testMirrorMovesRedPixelToTopRight(): void
    {
        $src = self::makeRedDotImage(); // red at (0,0)
        $out = sys_get_temp_dir() . '/rustimage_mirror_out.png';

        $img = Image::open($src);
        $img->mirror(); // horizontal flip: col 0 ↔ col 9
        $img->toPng();
        $img->save($out);

        $this->assertPixelColor($out, 9, 0, 255, 0, 0);    // red moved to (9,0)
        $this->assertPixelColor($out, 0, 0, 255, 255, 255); // (0,0) is now white

        unlink($src);
        unlink($out);
    }

    public function testAutoRotateNoExifIsNoOp(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_autorotate_noop.png';
        // Image::create produces no EXIF; autoRotate should be a no-op
        $img = Image::create(100, 50, new Rgb(255, 255, 255));
        $img->autoRotate();
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(100, $info->width);
        $this->assertSame(50, $info->height);
        unlink($tmp);
    }

    public function testAutoRotateOrientation6Rotates90CW(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_autorotate_rot.png';
        // exif_rotated.jpg: stored as 100×50 with EXIF orientation=6 (needs 90° CW rotation)
        // After autoRotate(), the visual size becomes 50×100.
        $img = Image::open(__DIR__ . '/fixtures/exif_rotated.jpg');
        $img->autoRotate();
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(50, $info->width);
        $this->assertSame(100, $info->height);
        unlink($tmp);
    }

    public function testAutoRotateIsIdempotentOnSameObject(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_autorotate_idem.png';
        $img = Image::open(__DIR__ . '/fixtures/exif_rotated.jpg');
        $img->autoRotate(); // applies rotation; internally sets orientation = Some(1)
        $img->autoRotate(); // Some(1) triggers early return — no second rotation
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(50, $info->width);
        $this->assertSame(100, $info->height);
        unlink($tmp);
    }
}
```

- [ ] **Step 2: Run the tests**

```bash
make phptest
```

Expected: all pass.

- [ ] **Step 3: Commit**

```bash
git add tests/TransformTest.php
git commit -m "test: add TransformTest"
```

---

### Task 13: EncodeTest

**Files:**
- Create: `tests/EncodeTest.php`

- [ ] **Step 1: Write EncodeTest.php**

```php
<?php
declare(strict_types=1);

namespace Tests;

use PHPUnit\Framework\TestCase;
use RustImage\Image;

class EncodeTest extends TestCase
{
    private static string $tmpSrc;

    public static function setUpBeforeClass(): void
    {
        self::$tmpSrc = sys_get_temp_dir() . '/rustimage_encode_src.jpg';
        $gd = imagecreatetruecolor(100, 80);
        imagefill($gd, 0, 0, imagecolorallocate($gd, 200, 200, 200));
        imagejpeg($gd, self::$tmpSrc, 95);
        imagedestroy($gd);
    }

    public static function tearDownAfterClass(): void
    {
        @unlink(self::$tmpSrc);
    }

    public function testToPng(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_enc_png.png';
        $img = Image::open(self::$tmpSrc);
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame('png', $info->format);
        $this->assertSame(100, $info->width);
        unlink($tmp);
    }

    public function testToJpegWithQuality(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_enc_jpeg.jpg';
        $img = Image::open(self::$tmpSrc);
        $img->toJpeg(quality: 50);
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame('jpeg', $info->format);
        $this->assertGreaterThan(0, filesize($tmp));
        unlink($tmp);
    }

    public function testToWebp(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_enc_webp.webp';
        $img = Image::open(self::$tmpSrc);
        $img->toWebp(quality: 80);
        $img->save($tmp);
        $this->assertGreaterThan(0, filesize($tmp));
        unlink($tmp);
    }

    public function testToGif(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_enc_gif.gif';
        $img = Image::open(self::$tmpSrc);
        $img->toGif();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame('gif', $info->format);
        unlink($tmp);
    }

    public function testToBufferFromBufferRoundTrip(): void
    {
        $img = Image::open(self::$tmpSrc);
        $img->toPng();
        $bytes = $img->toBuffer();
        $this->assertGreaterThan(0, strlen($bytes));

        $img2 = Image::fromBuffer($bytes);
        $tmp = sys_get_temp_dir() . '/rustimage_enc_roundtrip.png';
        $img2->toPng();
        $img2->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(100, $info->width);
        $this->assertSame(80, $info->height);
        unlink($tmp);
    }

    public function testSaveConfirmsFormatAndDimensions(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_enc_save.png';
        $img = Image::open(self::$tmpSrc);
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame('png', $info->format);
        $this->assertSame(100, $info->width);
        $this->assertSame(80, $info->height);
        unlink($tmp);
    }
}
```

- [ ] **Step 2: Run the tests**

```bash
make phptest
```

Expected: all pass.

- [ ] **Step 3: Commit**

```bash
git add tests/EncodeTest.php
git commit -m "test: add EncodeTest"
```

---

### Task 14: OverlayTest

**Files:**
- Create: `tests/OverlayTest.php`

- [ ] **Step 1: Write OverlayTest.php**

```php
<?php
declare(strict_types=1);

namespace Tests;

use PHPUnit\Framework\TestCase;
use RustImage\Image;
use RustImage\Rgb;

class OverlayTest extends TestCase
{
    use PixelAssertions;

    public function testOverlayPreservesBaseDimensions(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_overlay_dims.png';
        $base = Image::create(200, 150, new Rgb(255, 255, 255));
        $overlay = Image::create(40, 30, new Rgb(255, 0, 0));
        $base->overlay($overlay, 10, 10);
        $base->toPng();
        $base->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(200, $info->width);
        $this->assertSame(150, $info->height);
        unlink($tmp);
    }

    public function testOverlayChangesPixelInsideOverlayArea(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_overlay_pixel.png';
        // White 100×100 base; fully-opaque red 20×20 overlay placed at (10,10)
        $base = Image::create(100, 100, new Rgb(255, 255, 255));
        $overlay = Image::create(20, 20, new Rgb(255, 0, 0));
        $base->overlay($overlay, 10, 10, opacity: 1.0);
        $base->toPng();
        $base->save($tmp);

        // (15,15) is inside the overlay area → should be red
        $this->assertPixelColor($tmp, 15, 15, 255, 0, 0);
        // (5,5) is outside the overlay area → should remain white
        $this->assertPixelColor($tmp, 5, 5, 255, 255, 255);
        unlink($tmp);
    }

    public function testOverlayDoesNotMutateSource(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_overlay_src.png';
        $base = Image::create(100, 100, new Rgb(255, 255, 255));
        $overlay = Image::create(20, 20, new Rgb(0, 0, 255));
        $base->overlay($overlay, 0, 0);

        // Save the overlay object itself and verify it is unchanged
        $overlay->toPng();
        $overlay->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(20, $info->width);
        $this->assertSame(20, $info->height);
        $this->assertPixelColor($tmp, 0, 0, 0, 0, 255);
        unlink($tmp);
    }
}
```

- [ ] **Step 2: Run the tests**

```bash
make phptest
```

Expected: all pass.

- [ ] **Step 3: Commit**

```bash
git add tests/OverlayTest.php
git commit -m "test: add OverlayTest"
```

---

## Chunk 5: Rust Unit Tests and Cleanup

### Task 15: Rust unit tests in src/image.rs

**Files:**
- Modify: `src/image.rs` (append `#[cfg(test)]` module at end of file)

- [ ] **Step 1: Append the test module to src/image.rs**

Add the following block at the very end of `src/image.rs`, after all existing code:

```rust
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
        let result = read_exif_orientation_from_bytes(b"not a jpeg at all");
        assert!(result.is_none());
    }
}
```

- [ ] **Step 2: Run Rust tests**

```bash
cargo test
```

Expected output includes:
```
test tests::compute_fit_contain_landscape_into_square ... ok
test tests::compute_fit_contain_portrait_into_square ... ok
test tests::compute_fit_contain_equal_dimensions ... ok
test tests::compute_fit_contain_one_by_one_source ... ok
test tests::compute_fit_cover_landscape ... ok
test tests::compute_fit_fill_always_exact ... ok
test tests::crop_image_output_dimensions_correct ... ok
test tests::crop_image_origin_pixel_is_correct_source_pixel ... ok
test tests::read_exif_orientation_non_jpeg_returns_none ... ok

test result: ok. 9 passed
```

- [ ] **Step 3: Commit**

```bash
git add src/image.rs
git commit -m "test: add Rust unit tests for compute_fit, crop_image, EXIF parsing"
```

---

### Task 16: Delete old test scripts

**Files:**
- Delete: `test.php`
- Delete: `test_image.php`
- Delete: `test_gif_resize.php`
- Delete: `test_redis.php`
- Delete: `tests/test_new_api.php`

- [ ] **Step 1: Delete the files**

```bash
rm test.php test_image.php test_gif_resize.php test_redis.php tests/test_new_api.php
```

- [ ] **Step 2: Commit**

```bash
git add -u
git commit -m "chore: delete legacy ad-hoc test scripts"
```

---

### Task 17: Final verification

- [ ] **Step 1: Run the full test suite**

```bash
make test
```

Expected:
- `cargo build --release` completes successfully
- `cargo test`: 9 Rust tests pass
- `vendor/bin/phpunit`: all PHP tests pass (green bar, no failures)

- [ ] **Step 2: Confirm test count**

PHPUnit should report approximately 35–40 tests across the 8 test classes. If any tests fail, investigate and fix before proceeding.

- [ ] **Step 3: Final commit (if any fixups were needed)**

```bash
git add -A
git commit -m "test: verify full suite passes under make test"
```
