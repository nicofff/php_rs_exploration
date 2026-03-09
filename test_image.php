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
