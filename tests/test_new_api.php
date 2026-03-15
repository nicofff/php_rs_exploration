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
echo "=== Done ===\n";
