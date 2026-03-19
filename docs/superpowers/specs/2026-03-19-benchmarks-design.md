# Benchmarks Design

**Date:** 2026-03-19
**Project:** phprs_hello_world — Rust-backed PHP image extension

---

## Goal

Produce a one-shot, side-by-side performance comparison of RustImage vs GD vs Imagick for the three most important image pipeline operations: resize, overlay compositing, and format conversion.

---

## Framework

**phpbench/phpbench ^1.3** (installed via Composer as a dev dependency).

phpbench provides:
- Warmup runs (discarded)
- Configurable revolutions (within one iteration) and iterations (separate process runs)
- Aggregate reporting (`--report=aggregate`) showing mean, min, max per subject
- Natural grouping by class — each class produces one comparison table

---

## Directory Structure

```
benchmarks/
├── BenchmarkAssets.php   # shared trait: source image synthesis, cleanup
├── ResizeBench.php       # resize: contain at two sizes
├── OverlayBench.php      # overlay: composite a PNG onto base image
└── ConvertBench.php      # convert: JPEG→WebP, JPEG→PNG, JPEG→GIF

phpbench.json             # global config: bootstrap, path, warmup, revolutions, iterations
```

`bench_image.php` (existing root-level manual script) is **deleted** — replaced by this structured harness.

---

## phpbench.json

```json
{
    "runner.bootstrap": "vendor/autoload.php",
    "runner.path": "benchmarks/",
    "runner.warmup": 2,
    "runner.revolutions": 5,
    "runner.iterations": 5
}
```

---

## Makefile

Add to the existing Makefile:

```makefile
bench: build
	php -d extension=$(EXT_PATH) vendor/bin/phpbench run --report=aggregate
```

---

## BenchmarkAssets Trait

File: `benchmarks/BenchmarkAssets.php`, no namespace.

```php
trait BenchmarkAssets
{
    private string $sourcePath;
    private string $overlayPath;
    private string $sourceBuffer;
    private string $outPath;

    public function setUp(): void { ... }
    public function tearDown(): void { ... }
}
```

**`setUp()` synthesizes:**
- `$this->sourcePath` — 2000×1500 JPEG written to `sys_get_temp_dir()`, created via GD with random colored rectangles for realistic entropy
- `$this->overlayPath` — 200×200 solid red PNG written to `sys_get_temp_dir()`
- `$this->sourceBuffer` — `file_get_contents($this->sourcePath)`
- `$this->outPath` — a single reusable temp output path; each bench method writes here and overwrites it each revolution (no file accumulation)

**`tearDown()` unlinks** `$sourcePath`, `$overlayPath`, and `$outPath`.

Used via `#[BeforeMethods(['setUp'])]` and `#[AfterMethods(['tearDown'])]` on each bench class.

---

## ResizeBench

File: `benchmarks/ResizeBench.php`, no namespace.

Params (via `#[ParamProviders]`):
- `['width' => 800, 'height' => 600]` — large thumbnail
- `['width' => 200, 'height' => 150]` — small thumbnail

All three subjects use `fit='contain'`.

| Subject | Implementation |
|---|---|
| `benchRustImage` | `Image::open → resize(w, h) → toJpeg(80) → save` |
| `benchGd` | `imagecreatefromjpeg → imagescale → imagejpeg` |
| `benchImagick` | `new Imagick → resizeImage(w, h, FILTER_CATROM, 1, true) → writeImage` |

---

## OverlayBench

File: `benchmarks/OverlayBench.php`, no namespace.

Single scenario: composite a 200×200 opaque PNG at position (100, 100) onto the 2000×1500 base, encode to JPEG, discard output.

| Subject | Implementation |
|---|---|
| `benchRustImage` | `Image::open(base) → overlay(Image::open(overlay), 100, 100) → toJpeg(80) → toBuffer()` |
| `benchGd` | `imagecreatefromjpeg → imagecopy → imagejpeg to buffer via ob_start` |
| `benchImagick` | `new Imagick(base) → compositeImage(new Imagick(overlay), COMPOSITE_OVER, 100, 100) → getImageBlob` |

---

## ConvertBench

File: `benchmarks/ConvertBench.php`, no namespace.

Params (via `#[ParamProviders]`):
- `['format' => 'webp', 'quality' => 80]`
- `['format' => 'png']`
- `['format' => 'gif']`

Source is always the 2000×1500 JPEG from `$this->sourcePath`.

| Subject | Implementation |
|---|---|
| `benchRustImage` | `Image::open → toWebp/toPng/toGif → toBuffer()` |
| `benchGd` | `imagecreatefromjpeg → imagewebp/imagepng/imagegif to buffer` |
| `benchImagick` | `new Imagick → setImageFormat → getImageBlob` |

Note: GD's WebP support (`imagewebp`) is available in PHP 8.0+. GIF via GD produces a palette-quantized result (lossy colour reduction); this is noted in the benchmark output via a comment in the params.

---

## Running

```bash
make bench
```

Sample output (phpbench aggregate report):

```
ResizeBench
+-----------------+----------+----------+----------+
| subject         | mean     | min      | max      |
+-----------------+----------+----------+----------+
| benchRustImage  | 12.34ms  | 11.90ms  | 13.10ms  |
| benchGd         | 25.67ms  | 24.80ms  | 26.50ms  |
| benchImagick    | 18.45ms  | 17.90ms  | 19.20ms  |
+-----------------+----------+----------+----------+
```

---

## Out of Scope

- Saving benchmark results to disk / historical tracking
- HTML report output
- Animated GIF benchmarks (resize only handles first frame in GD)
- Memory usage measurement
- Concurrency / parallel benchmarks
