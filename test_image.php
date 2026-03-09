<?php

echo "=== RustImage Test Suite ===\n\n";

// Task 1: Scaffold — basic open and info
echo "--- Task 1: Scaffold ---\n";

// We need a test image. Create a simple one using GD.
$tmpJpeg = '/tmp/rustimage_test_photo.jpg';
$img = imagecreatetruecolor(200, 150);
$white = imagecolorallocate($img, 255, 255, 255);
imagefill($img, 0, 0, $white);
$red = imagecolorallocate($img, 255, 0, 0);
imagefilledrectangle($img, 50, 30, 150, 120, $red);
imagejpeg($img, $tmpJpeg, 90);
imagedestroy($img);
echo "Created test JPEG: $tmpJpeg\n";

// Test Image::open
$image = RustImage\Image::open($tmpJpeg);
echo "Image::open OK\n";

// Test Image::info
$info = RustImage\Image::info($tmpJpeg);
echo "Width: {$info->width}\n";
echo "Height: {$info->height}\n";
echo "Format: {$info->format}\n";
assert($info->width === 200, "Expected width 200");
assert($info->height === 150, "Expected height 150");

// Test error on missing file
try {
    RustImage\Image::open('/tmp/nonexistent_image.jpg');
    echo "FAIL: should have thrown\n";
} catch (RustImage\ImageException $e) {
    echo "Expected error: " . $e->getMessage() . "\n";
}

echo "\nTask 1 passed!\n";

// Task 2: Decode + fromBuffer
echo "\n--- Task 2: Decode + fromBuffer ---\n";

// Create a PNG with transparency for testing
$tmpPng = '/tmp/rustimage_test_alpha.png';
$img = imagecreatetruecolor(100, 80);
imagesavealpha($img, true);
$transparent = imagecolorallocatealpha($img, 0, 0, 0, 127);
imagefill($img, 0, 0, $transparent);
$blue = imagecolorallocate($img, 0, 0, 255);
imagefilledellipse($img, 50, 40, 60, 40, $blue);
imagepng($img, $tmpPng);
imagedestroy($img);
echo "Created test PNG: $tmpPng\n";

// Open PNG
$image = RustImage\Image::open($tmpPng);
echo "PNG open OK\n";
$info = RustImage\Image::info($tmpPng);
assert($info->width === 100, "Expected PNG width 100");
assert($info->height === 80, "Expected PNG height 80");
echo "PNG info: {$info->width}x{$info->height} {$info->format}\n";

// fromBuffer
$bytes = file_get_contents($tmpJpeg);
$image = RustImage\Image::fromBuffer($bytes);
echo "fromBuffer OK\n";

// Resource limits — dimension check
try {
    RustImage\Image::open($tmpJpeg, max_width: 50, max_height: 50);
    echo "FAIL: should have thrown for dimension limit\n";
} catch (RustImage\ImageException $e) {
    echo "Dimension limit: " . $e->getMessage() . "\n";
}

// Resource limits — file size check
try {
    RustImage\Image::open($tmpJpeg, max_bytes: 10);
    echo "FAIL: should have thrown for size limit\n";
} catch (RustImage\ImageException $e) {
    echo "Size limit: " . $e->getMessage() . "\n";
}

// Corrupt file
$tmpCorrupt = '/tmp/rustimage_test_corrupt.jpg';
file_put_contents($tmpCorrupt, "not a real image");
try {
    RustImage\Image::open($tmpCorrupt);
    echo "FAIL: should have thrown for corrupt file\n";
} catch (RustImage\ImageException $e) {
    echo "Corrupt file: " . $e->getMessage() . "\n";
}

echo "\nTask 2 passed!\n";

// Task 3: Resize
echo "\n--- Task 3: Resize ---\n";

// Test contain (default) — should preserve aspect ratio
$outContain = '/tmp/rustimage_test_contain.png';
$image = RustImage\Image::open($tmpJpeg);  // 200x150
$image->resize(100, 100);
$image->toPng();
$image->save($outContain);
$info = RustImage\Image::info($outContain);
echo "Contain 200x150 into 100x100: {$info->width}x{$info->height}\n";
assert($info->width === 100, "Contain width should be 100, got {$info->width}");
assert($info->height === 75, "Contain height should be 75, got {$info->height}");

// Test fill — exact dimensions, stretches
$outFill = '/tmp/rustimage_test_fill.png';
$image = RustImage\Image::open($tmpJpeg);
$image->resize(100, 100, fit: 'fill');
$image->toPng();
$image->save($outFill);
$info = RustImage\Image::info($outFill);
echo "Fill 200x150 into 100x100: {$info->width}x{$info->height}\n";
assert($info->width === 100 && $info->height === 100, "Fill should be exactly 100x100");

// Test cover — fill area, crop excess
$outCover = '/tmp/rustimage_test_cover.png';
$image = RustImage\Image::open($tmpJpeg);
$image->resize(100, 100, fit: 'cover');
$image->toPng();
$image->save($outCover);
$info = RustImage\Image::info($outCover);
echo "Cover 200x150 into 100x100: {$info->width}x{$info->height}\n";
assert($info->width === 100 && $info->height === 100, "Cover should be exactly 100x100");

// Test thumbnail (bilinear, fast)
$outThumb = '/tmp/rustimage_test_thumb.png';
$image = RustImage\Image::open($tmpJpeg);
$image->thumbnail(50, 50);
$image->toPng();
$image->save($outThumb);
$info = RustImage\Image::info($outThumb);
echo "Thumbnail 200x150 into 50x50: {$info->width}x{$info->height}\n";
assert($info->width === 50, "Thumb width should be 50");

// Test JPEG output with quality
$outJpeg = '/tmp/rustimage_test_resized.jpg';
$image = RustImage\Image::open($tmpJpeg);
$image->resize(100, 100);
$image->toJpeg(quality: 70);
$image->save($outJpeg);
$info = RustImage\Image::info($outJpeg);
echo "JPEG resize: {$info->width}x{$info->height} {$info->format}\n";
assert($info->format === 'jpeg', "Should be JPEG format");

echo "\nTask 3 passed!\n";
