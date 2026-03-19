# Benchmarks Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a phpbench suite that compares RustImage vs GD vs Imagick for resize, overlay, and format conversion.

**Architecture:** Four files in `benchmarks/` — a shared trait (`BenchmarkAssets`) for image synthesis/cleanup, and one class per operation (`ResizeBench`, `OverlayBench`, `ConvertBench`). phpbench groups results by class, producing a clean side-by-side table per operation. Wired into the Makefile as `make bench`.

**Tech Stack:** PHP 8.2+, phpbench ^1.3 (Composer), GD, Imagick, RustImage extension (loaded via `-d extension=`).

**Spec:** `docs/superpowers/specs/2026-03-19-benchmarks-design.md`

---

## Chunk 1: Scaffold

### Task 1: Add phpbench to Composer and install

**Files:**
- Modify: `composer.json`

- [ ] **Step 1: Add phpbench to composer.json require-dev**

Edit `composer.json` to add `"phpbench/phpbench": "^1.3"` under `require-dev`:

```json
{
    "name": "phprs/hello-world",
    "require": {},
    "require-dev": {
        "phpunit/phpunit": "^11",
        "phpbench/phpbench": "^1.3"
    },
    "autoload-dev": {
        "psr-4": {
            "Tests\\": "tests/"
        }
    },
    "minimum-stability": "stable"
}
```

- [ ] **Step 2: Install**

```bash
composer update
```

Expected: phpbench installed, `vendor/bin/phpbench` available.

- [ ] **Step 3: Commit**

```bash
git add composer.json composer.lock
git commit -m "chore: add phpbench dev dependency"
```

---

### Task 2: Add phpbench.json

**Files:**
- Create: `phpbench.json`

- [ ] **Step 1: Write phpbench.json**

```json
{
    "runner.bootstrap": "vendor/autoload.php",
    "runner.path": "benchmarks/",
    "runner.warmup": 2,
    "runner.revolutions": 5,
    "runner.iterations": 5
}
```

- [ ] **Step 2: Commit**

```bash
git add phpbench.json
git commit -m "chore: add phpbench.json config"
```

---

### Task 3: Add bench target to Makefile and delete bench_image.php

**Files:**
- Modify: `Makefile`
- Delete: `bench_image.php`

- [ ] **Step 1: Add bench target to Makefile**

Add the following to `Makefile` before the `.PHONY` line:

```makefile
bench: build
	php -d extension=$(EXT_PATH) vendor/bin/phpbench run --report=aggregate
```

Also add `bench` to the `.PHONY` declaration:

```makefile
.PHONY: build rusttest phptest test bench
```

The full Makefile should now look like:

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

bench: build
	php -d extension=$(EXT_PATH) vendor/bin/phpbench run --report=aggregate

.PHONY: build rusttest phptest test bench
```

Note: recipe lines use **tab characters**, not spaces.

- [ ] **Step 2: Delete bench_image.php**

```bash
rm bench_image.php
```

- [ ] **Step 3: Commit**

```bash
git add Makefile
git add -u bench_image.php
git commit -m "chore: add make bench target; remove bench_image.php"
```

---

## Chunk 2: BenchmarkAssets and ResizeBench

### Task 4: BenchmarkAssets trait

**Files:**
- Create: `benchmarks/BenchmarkAssets.php`

- [ ] **Step 1: Create benchmarks/ directory and write BenchmarkAssets.php**

```php
<?php
declare(strict_types=1);

trait BenchmarkAssets
{
    private string $sourcePath;
    private string $overlayPath;
    private string $sourceBuffer;
    private string $outPath;

    public function setUp(): void
    {
        $this->sourcePath  = sys_get_temp_dir() . '/phpbench_source.jpg';
        $this->overlayPath = sys_get_temp_dir() . '/phpbench_overlay.png';
        $this->outPath     = sys_get_temp_dir() . '/phpbench_out';

        // 2000×1500 JPEG with random colored rectangles for realistic entropy
        $gd = imagecreatetruecolor(2000, 1500);
        imagefill($gd, 0, 0, imagecolorallocate($gd, 200, 200, 200));
        for ($i = 0; $i < 50; $i++) {
            $color = imagecolorallocate($gd, random_int(0, 255), random_int(0, 255), random_int(0, 255));
            imagefilledrectangle(
                $gd,
                random_int(0, 1900), random_int(0, 1400),
                random_int(0, 1900), random_int(0, 1400),
                $color
            );
        }
        imagejpeg($gd, $this->sourcePath, 90);
        imagedestroy($gd);

        // 200×200 solid red PNG
        $overlay = imagecreatetruecolor(200, 200);
        imagefill($overlay, 0, 0, imagecolorallocate($overlay, 255, 0, 0));
        imagepng($overlay, $this->overlayPath);
        imagedestroy($overlay);

        $this->sourceBuffer = file_get_contents($this->sourcePath);
    }

    public function tearDown(): void
    {
        @unlink($this->sourcePath);
        @unlink($this->overlayPath);
        foreach (glob(sys_get_temp_dir() . '/phpbench_out*') as $f) {
            @unlink($f);
        }
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add benchmarks/BenchmarkAssets.php
git commit -m "bench: add BenchmarkAssets shared trait"
```

---

### Task 5: ResizeBench

**Files:**
- Create: `benchmarks/ResizeBench.php`

- [ ] **Step 1: Write ResizeBench.php**

```php
<?php
declare(strict_types=1);

use PhpBench\Attributes as Bench;
use RustImage\Image;

#[Bench\BeforeMethods(['setUp'])]
#[Bench\AfterMethods(['tearDown'])]
#[Bench\ParamProviders(['provideSizes'])]
class ResizeBench
{
    use BenchmarkAssets;

    public function provideSizes(): \Generator
    {
        yield 'large 800x600' => ['width' => 800, 'height' => 600];
        yield 'small 200x150' => ['width' => 200, 'height' => 150];
    }

    public function benchRustImage(array $params): void
    {
        $img = Image::open($this->sourcePath);
        $img->resize($params['width'], $params['height']);
        $img->toJpeg(quality: 80);
        $img->save($this->outPath . '.jpg');
    }

    public function benchGd(array $params): void
    {
        $src     = imagecreatefromjpeg($this->sourcePath);
        $resized = imagescale($src, $params['width'], $params['height']);
        imagejpeg($resized, $this->outPath . '.jpg', 80);
        imagedestroy($src);
        imagedestroy($resized);
    }

    public function benchImagick(array $params): void
    {
        $im = new Imagick($this->sourcePath);
        $im->resizeImage($params['width'], $params['height'], Imagick::FILTER_CATROM, 1, true);
        $im->setImageCompressionQuality(80);
        $im->writeImage($this->outPath . '.jpg');
        $im->destroy();
    }
}
```

- [ ] **Step 2: Verify ResizeBench runs without errors**

```bash
php -d extension=target/release/libphprs_hello_world.dylib vendor/bin/phpbench run benchmarks/ResizeBench.php --report=aggregate
```

Expected: benchmark table printed, no PHP errors. Timing numbers will vary.

- [ ] **Step 3: Commit**

```bash
git add benchmarks/ResizeBench.php
git commit -m "bench: add ResizeBench (RustImage vs GD vs Imagick)"
```

---

## Chunk 3: OverlayBench, ConvertBench, and final verification

### Task 6: OverlayBench

**Files:**
- Create: `benchmarks/OverlayBench.php`

- [ ] **Step 1: Write OverlayBench.php**

```php
<?php
declare(strict_types=1);

use PhpBench\Attributes as Bench;
use RustImage\Image;

#[Bench\BeforeMethods(['setUp'])]
#[Bench\AfterMethods(['tearDown'])]
class OverlayBench
{
    use BenchmarkAssets;

    public function benchRustImage(): void
    {
        $base    = Image::open($this->sourcePath);
        $overlay = Image::open($this->overlayPath);
        $base->overlay($overlay, 100, 100);
        $base->toJpeg(quality: 80);
        $base->toBuffer();
    }

    public function benchGd(): void
    {
        $base    = imagecreatefromjpeg($this->sourcePath);
        $overlay = imagecreatefrompng($this->overlayPath);
        imagecopy($base, $overlay, 100, 100, 0, 0, 200, 200);
        ob_start();
        imagejpeg($base, null, 80);
        ob_get_clean();
        imagedestroy($base);
        imagedestroy($overlay);
    }

    public function benchImagick(): void
    {
        $base    = new Imagick($this->sourcePath);
        $overlay = new Imagick($this->overlayPath);
        $base->compositeImage($overlay, Imagick::COMPOSITE_OVER, 100, 100);
        $base->setImageCompressionQuality(80);
        $base->getImageBlob();
        $base->destroy();
        $overlay->destroy();
    }
}
```

- [ ] **Step 2: Verify OverlayBench runs without errors**

```bash
php -d extension=target/release/libphprs_hello_world.dylib vendor/bin/phpbench run benchmarks/OverlayBench.php --report=aggregate
```

Expected: benchmark table printed, no PHP errors.

- [ ] **Step 3: Commit**

```bash
git add benchmarks/OverlayBench.php
git commit -m "bench: add OverlayBench (RustImage vs GD vs Imagick)"
```

---

### Task 7: ConvertBench

**Files:**
- Create: `benchmarks/ConvertBench.php`

- [ ] **Step 1: Write ConvertBench.php**

```php
<?php
declare(strict_types=1);

use PhpBench\Attributes as Bench;
use RustImage\Image;

#[Bench\BeforeMethods(['setUp'])]
#[Bench\AfterMethods(['tearDown'])]
#[Bench\ParamProviders(['provideFormats'])]
class ConvertBench
{
    use BenchmarkAssets;

    public function provideFormats(): \Generator
    {
        yield 'webp' => ['format' => 'webp', 'quality' => 80];
        yield 'png'  => ['format' => 'png'];
        yield 'gif'  => ['format' => 'gif'];
    }

    public function benchRustImage(array $params): void
    {
        $img = Image::open($this->sourcePath);
        match ($params['format']) {
            'webp' => $img->toWebp($params['quality'] ?? 80),
            'png'  => $img->toPng(),
            'gif'  => $img->toGif(),
        };
        $img->toBuffer();
    }

    public function benchGd(array $params): void
    {
        $gd = imagecreatefromjpeg($this->sourcePath);
        ob_start();
        match ($params['format']) {
            'webp' => imagewebp($gd, null, $params['quality'] ?? 80),
            'png'  => imagepng($gd, null),
            'gif'  => imagegif($gd, null),
        };
        ob_get_clean();
        imagedestroy($gd);
    }

    public function benchImagick(array $params): void
    {
        $im = new Imagick($this->sourcePath);
        $im->setImageFormat($params['format']);
        if (isset($params['quality'])) {
            $im->setImageCompressionQuality($params['quality']);
        }
        $im->getImageBlob();
        $im->destroy();
    }
}
```

- [ ] **Step 2: Verify ConvertBench runs without errors**

```bash
php -d extension=target/release/libphprs_hello_world.dylib vendor/bin/phpbench run benchmarks/ConvertBench.php --report=aggregate
```

Expected: benchmark table with webp/png/gif rows for each subject, no PHP errors.

- [ ] **Step 3: Commit**

```bash
git add benchmarks/ConvertBench.php
git commit -m "bench: add ConvertBench (RustImage vs GD vs Imagick)"
```

---

### Task 8: Final verification

- [ ] **Step 1: Run the full benchmark suite**

```bash
make bench
```

Expected: three benchmark tables printed (ResizeBench, OverlayBench, ConvertBench), all three libraries measured, no PHP errors or exceptions.

If running on macOS and the extension path differs from the default `.dylib`, the Makefile's `EXT_PATH` detection handles it automatically.

- [ ] **Step 2: Confirm output shape**

The output should look roughly like:

```
ResizeBench
+----------------+-----+------+------+------+------+------+
| benchmark      | set | revs | iter | mean | min  | max  |
+----------------+-----+------+------+------+------+------+
| benchRustImage | ... | 5    | 5    | ...  | ...  | ...  |
| benchGd        | ... | 5    | 5    | ...  | ...  | ...  |
| benchImagick   | ... | 5    | 5    | ...  | ...  | ...  |
+----------------+-----+------+------+------+------+------+

OverlayBench
...

ConvertBench
...
```

If any benchmark class fails to load (e.g., class not found), phpbench will report an error. Check that the `benchmarks/` directory is on the include path — `vendor/autoload.php` bootstraps Composer's autoloader, but the benchmark files themselves are in `benchmarks/` without a namespace. phpbench loads them by scanning the configured `runner.path` directly, so no additional autoload registration is needed.
