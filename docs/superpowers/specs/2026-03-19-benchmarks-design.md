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

## phpbench Attribute Imports

All benchmark classes use the phpbench attribute namespace via:

```php
use PhpBench\Attributes as Bench;
```

Attributes are then written as `#[Bench\BeforeMethods(['setUp'])]`, `#[Bench\AfterMethods(['tearDown'])]`, `#[Bench\ParamProviders(['providerMethod'])]`.

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

**Fixed temp paths** (not unique per call — same path reused across setUp/tearDown cycles):
- `$this->sourcePath` = `sys_get_temp_dir() . '/phpbench_source.jpg'`
- `$this->overlayPath` = `sys_get_temp_dir() . '/phpbench_overlay.png'`
- `$this->outPath` = `sys_get_temp_dir() . '/phpbench_out'` (extension appended per bench method as needed, e.g. `.jpg`, `.webp`)

**`setUp()` creates:**
- `$this->sourcePath` — 2000×1500 JPEG via GD with random colored rectangles for realistic entropy
- `$this->overlayPath` — 200×200 solid red PNG via GD
- `$this->sourceBuffer` — `file_get_contents($this->sourcePath)`

**`tearDown()` cleans up:**
```php
@unlink($this->sourcePath);
@unlink($this->overlayPath);
foreach (glob(sys_get_temp_dir() . '/phpbench_out*') as $f) {
    @unlink($f);
}
```

Using `@unlink` (with suppression) and `glob` for outPath handles the case where a benchmark method discarded output to buffer and never wrote to `outPath`.

Used via `#[Bench\BeforeMethods(['setUp'])]` and `#[Bench\AfterMethods(['tearDown'])]` on each bench class.

---

## ResizeBench

File: `benchmarks/ResizeBench.php`, no namespace.

```php
use PhpBench\Attributes as Bench;
```

Params provider returns:
- `['width' => 800, 'height' => 600]` — large thumbnail
- `['width' => 200, 'height' => 150]` — small thumbnail

All three subjects use `fit='contain'`. Output written to `$this->outPath . '.jpg'`.

| Subject | Implementation |
|---|---|
| `benchRustImage` | `Image::open($sourcePath) → resize(w, h) → toJpeg(80) → save(outPath.jpg)` |
| `benchGd` | `imagecreatefromjpeg → imagescale(img, w, h) → imagejpeg(result, outPath.jpg, 80)` |
| `benchImagick` | `new Imagick(sourcePath) → resizeImage(w, h, Imagick::FILTER_CATROM, 1, true) → writeImage(outPath.jpg)` |

---

## OverlayBench

File: `benchmarks/OverlayBench.php`, no namespace.

```php
use PhpBench\Attributes as Bench;
```

Single scenario: composite a 200×200 opaque PNG at position (100, 100) onto the 2000×1500 base, encode to JPEG, return as buffer (no file written — `outPath` never touched).

| Subject | Implementation |
|---|---|
| `benchRustImage` | `Image::open(sourcePath) → overlay(Image::open(overlayPath), 100, 100) → toJpeg(80) → toBuffer()` |
| `benchGd` | `imagecreatefromjpeg(sourcePath) → imagecopy(dst, src, 100, 100, 0, 0, 200, 200) → ob_start(); imagejpeg(dst, null, 80); ob_get_clean()` |
| `benchImagick` | `new Imagick(sourcePath) → compositeImage(new Imagick(overlayPath), Imagick::COMPOSITE_OVER, 100, 100) → getImageBlob()` |

---

## ConvertBench

File: `benchmarks/ConvertBench.php`, no namespace.

```php
use PhpBench\Attributes as Bench;
```

Params provider returns:
- `['format' => 'webp', 'quality' => 80]`
- `['format' => 'png']`
- `['format' => 'gif']`

All output discarded to buffer (no file written). GD buffering uses `ob_start() / ob_get_clean()` for all three formats.

| Subject | Format | GD call | Imagick call |
|---|---|---|---|
| `benchRustImage` | webp | — | — |
| `benchRustImage` | png | — | — |
| `benchRustImage` | gif | — | — |

Implemented as a single `benchRustImage($params)` method switching on `$params['format']`:
```php
match($params['format']) {
    'webp' => $img->toWebp($params['quality'] ?? 80),
    'png'  => $img->toPng(),
    'gif'  => $img->toGif(),
};
$img->toBuffer();
```

Similarly for `benchGd($params)`:
```php
ob_start();
match($params['format']) {
    'webp' => imagewebp($gd, null, $params['quality'] ?? 80),
    'png'  => imagepng($gd, null),    // null = output to buffer; compression omitted (default)
    'gif'  => imagegif($gd, null),
};
ob_get_clean();
```

And `benchImagick($params)`:
```php
$im->setImageFormat($params['format']);
if (isset($params['quality'])) $im->setImageCompressionQuality($params['quality']);
$im->getImageBlob();
```

Note: GIF via GD produces a palette-quantized result (lossy colour reduction) — this is an inherent GD limitation, not a benchmark error.

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
