<?php

echo "=== RustImage Benchmark ===\n";
echo "Comparing: RustImage vs GD" . (extension_loaded('imagick') ? " vs Imagick" : "") . "\n\n";

// Generate test image
$source = '/tmp/bench_source.jpg';
$img = imagecreatetruecolor(2000, 1500);
for ($i = 0; $i < 100; $i++) {
    $color = imagecolorallocate($img, rand(0, 255), rand(0, 255), rand(0, 255));
    imagefilledrectangle($img, rand(0, 1900), rand(0, 1400), rand(0, 1900), rand(0, 1400), $color);
}
imagejpeg($img, $source, 90);
imagedestroy($img);
echo "Source image: 2000x1500 JPEG (" . round(filesize($source) / 1024) . " KB)\n\n";

$iterations = 100;

// --- Benchmark 1: Resize to thumbnail ---
echo "--- Thumbnail generation ({$iterations}x) ---\n";

// RustImage
$start = hrtime(true);
for ($i = 0; $i < $iterations; $i++) {
    $image = RustImage\Image::open($source);
    $image->thumbnail(200, 200);
    $image->toJpeg(quality: 80);
    $image->save("/tmp/bench_rust_{$i}.jpg");
}
$rustTime = (hrtime(true) - $start) / 1e6;
echo sprintf("  RustImage: %7.0f ms\n", $rustTime);

// GD
$start = hrtime(true);
for ($i = 0; $i < $iterations; $i++) {
    $src = imagecreatefromjpeg($source);
    $thumb = imagescale($src, 200, 200);
    imagejpeg($thumb, "/tmp/bench_gd_{$i}.jpg", 80);
    imagedestroy($src);
    imagedestroy($thumb);
}
$gdTime = (hrtime(true) - $start) / 1e6;
echo sprintf("  GD:        %7.0f ms\n", $gdTime);

// Imagick
if (extension_loaded('imagick')) {
    $start = hrtime(true);
    for ($i = 0; $i < $iterations; $i++) {
        $im = new Imagick($source);
        $im->thumbnailImage(200, 200, true);
        $im->setImageCompressionQuality(80);
        $im->writeImage("/tmp/bench_imagick_{$i}.jpg");
        $im->destroy();
    }
    $imagickTime = (hrtime(true) - $start) / 1e6;
    echo sprintf("  Imagick:   %7.0f ms\n", $imagickTime);
}

echo sprintf("  Speedup vs GD: %.1fx\n", $gdTime / $rustTime);
if (isset($imagickTime)) {
    echo sprintf("  Speedup vs Imagick: %.1fx\n", $imagickTime / $rustTime);
}

// --- Benchmark 2: JPEG to WebP ---
echo "\n--- JPEG to WebP ({$iterations}x) ---\n";

$start = hrtime(true);
for ($i = 0; $i < $iterations; $i++) {
    $image = RustImage\Image::open($source);
    $image->resize(800, 600);
    $image->toWebp(quality: 80);
    $image->save("/tmp/bench_rust_webp_{$i}.webp");
}
$rustTime = (hrtime(true) - $start) / 1e6;
echo sprintf("  RustImage: %7.0f ms\n", $rustTime);

if (function_exists('imagewebp')) {
    $start = hrtime(true);
    for ($i = 0; $i < $iterations; $i++) {
        $src = imagecreatefromjpeg($source);
        $resized = imagescale($src, 800, 600);
        imagewebp($resized, "/tmp/bench_gd_webp_{$i}.webp", 80);
        imagedestroy($src);
        imagedestroy($resized);
    }
    $gdTime = (hrtime(true) - $start) / 1e6;
    echo sprintf("  GD:        %7.0f ms\n", $gdTime);
    echo sprintf("  Speedup vs GD: %.1fx\n", $gdTime / $rustTime);
}

// --- Cleanup ---
echo "\n--- Cleanup ---\n";
$cleaned = 0;
foreach (glob('/tmp/bench_*') as $f) { unlink($f); $cleaned++; }
echo "Cleaned up {$cleaned} benchmark files\n";
