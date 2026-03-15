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
$g = ($pixel >> 8) & 0xFF;
$b = $pixel & 0xFF;
// Not red means: either low r, or high g and b (white also has r=255 but g=b=255 too)
assert(!($r > 200 && $g < 50 && $b < 50), "Top-left of offset crop should not be red, got r=$r g=$g b=$b");
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
    return ($pixel >> 16) & 0xFF;
}

function getPixelGreen(string $path, int $x, int $y): int {
    $gd = imagecreatefrompng($path);
    $pixel = imagecolorat($gd, $x, $y);
    return ($pixel >> 8) & 0xFF;
}

function getPixelBlue(string $path, int $x, int $y): int {
    $gd = imagecreatefrompng($path);
    $pixel = imagecolorat($gd, $x, $y);
    return $pixel & 0xFF;
}

// flip() — red pixel (0,0) should move to (0,9)
$src = makeTestImage();
$img = RustImage\Image::open($src);
$img->flip();
$img->toPng();
$out = '/tmp/rustimage_flip_out.png';
$img->save($out);
assert(getPixelRed($out, 0, 9) > 200 && getPixelGreen($out, 0, 9) < 50 && getPixelBlue($out, 0, 9) < 50, "After flip(), red should be at (0,9)");
assert(getPixelGreen($out, 0, 0) > 200 && getPixelBlue($out, 0, 0) > 200, "After flip(), (0,0) should be white");
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
assert(getPixelRed($out, 9, 0) > 200 && getPixelGreen($out, 9, 0) < 50 && getPixelBlue($out, 9, 0) < 50, "After mirror(), red should be at (9,0)");
assert(getPixelGreen($out, 0, 0) > 200 && getPixelBlue($out, 0, 0) > 200, "After mirror(), (0,0) should be white");
echo "mirror() OK\n";
unlink($out);
unlink($src);

echo "Task 6 PASSED\n\n";

// ── Task 7: auto_rotate() ────────────────────────────────────────────────────
echo "--- Task 7: auto_rotate() ---\n";

// No-op on image without EXIF (create() produces None orientation)
$img = RustImage\Image::create(100, 50, new RustImage\Rgb(255, 255, 255));
$img->autoRotate();
$img->toPng();
$tmp = '/tmp/rustimage_autorotate_noop.png';
$img->save($tmp);
$info = RustImage\Image::info($tmp);
assert($info->width === 100 && $info->height === 50, "No EXIF: dimensions should be unchanged, got {$info->width}x{$info->height}");
echo "No-op (no EXIF) OK: {$info->width}x{$info->height}\n";
unlink($tmp);

// Orientation 6: 100x50 JPEG → after 90° CW rotation → 50x100
$img = RustImage\Image::open(__DIR__ . '/fixtures/exif_rotated.jpg');
$img->autoRotate();
$img->toPng();
$tmpRot = '/tmp/rustimage_autorotate_rot.png';
$img->save($tmpRot);
$infoRot = RustImage\Image::info($tmpRot);
assert($infoRot->width === 50 && $infoRot->height === 100,
    "Orientation 6 should produce 50x100, got {$infoRot->width}x{$infoRot->height}");
echo "Orientation 6 rotation OK: {$infoRot->width}x{$infoRot->height}\n";
unlink($tmpRot);

// Idempotency — calling twice should not double-rotate (orientation reset to Some(1) after first call)
$img = RustImage\Image::open(__DIR__ . '/fixtures/exif_rotated.jpg');
$img->autoRotate();
$img->autoRotate(); // second call should be a no-op
$img->toPng();
$tmpIdem = '/tmp/rustimage_autorotate_idem.png';
$img->save($tmpIdem);
$infoIdem = RustImage\Image::info($tmpIdem);
assert($infoIdem->width === 50 && $infoIdem->height === 100,
    "Idempotency: second autoRotate() should be no-op, got {$infoIdem->width}x{$infoIdem->height}");
echo "Idempotency OK: second call is no-op\n";
unlink($tmpIdem);

echo "Task 7 PASSED\n\n";

echo "=== Done ===\n";
