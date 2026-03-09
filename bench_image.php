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
    }
    $gdTime = (hrtime(true) - $start) / 1e6;
    echo sprintf("  GD:        %7.0f ms\n", $gdTime);
    echo sprintf("  Speedup vs GD: %.1fx\n", $gdTime / $rustTime);
}

// --- Benchmark 3: From buffer (no file I/O in the hot path) ---
echo "\n--- Resize from buffer, no I/O ({$iterations}x) ---\n";

$buffer = file_get_contents($source);

// RustImage
$start = hrtime(true);
for ($i = 0; $i < $iterations; $i++) {
    $image = RustImage\Image::fromBuffer($buffer);
    $image->thumbnail(200, 200);
    $image->toJpeg(quality: 80);
    $image->toBuffer();
}
$rustTime = (hrtime(true) - $start) / 1e6;
echo sprintf("  RustImage: %7.0f ms\n", $rustTime);

// GD
$start = hrtime(true);
for ($i = 0; $i < $iterations; $i++) {
    $src = imagecreatefromstring($buffer);
    $thumb = imagescale($src, 200, 200);
    ob_start();
    imagejpeg($thumb, null, 80);
    ob_end_clean();
}
$gdTime = (hrtime(true) - $start) / 1e6;
echo sprintf("  GD:        %7.0f ms\n", $gdTime);

// Imagick
if (extension_loaded('imagick')) {
    $start = hrtime(true);
    for ($i = 0; $i < $iterations; $i++) {
        $im = new Imagick();
        $im->readImageBlob($buffer);
        $im->thumbnailImage(200, 200, true);
        $im->setImageCompressionQuality(80);
        $im->setImageFormat('jpeg');
        $im->getImageBlob();
        $im->destroy();
    }
    $imagickTime = (hrtime(true) - $start) / 1e6;
    echo sprintf("  Imagick:   %7.0f ms\n", $imagickTime);
}

echo sprintf("  Speedup vs GD: %.1fx\n", $gdTime / $rustTime);
if (isset($imagickTime)) {
    echo sprintf("  Speedup vs Imagick: %.1fx\n", $imagickTime / $rustTime);
}
unset($imagickTime);

// --- Benchmark 4: Multiple thumbnails from same source (read once) ---
echo "\n--- Multiple thumbnails, single read ({$iterations}x) ---\n";
$sizes = [[800, 600], [400, 300], [200, 150], [100, 75], [50, 38]];
echo "  Sizes: " . implode(', ', array_map(fn($s) => "{$s[0]}x{$s[1]}", $sizes)) . "\n";

// RustImage — decode once, copy() for each size
$start = hrtime(true);
for ($i = 0; $i < $iterations; $i++) {
    $original = RustImage\Image::open($source);
    foreach ($sizes as [$w, $h]) {
        $thumb = $original->copy();
        $thumb->thumbnail($w, $h);
        $thumb->toJpeg(quality: 80);
        $thumb->toBuffer();
    }
}
$rustTime = (hrtime(true) - $start) / 1e6;
echo sprintf("  RustImage: %7.0f ms  (%d thumbnails)\n", $rustTime, $iterations * count($sizes));

// GD — read once, generate all sizes
$start = hrtime(true);
for ($i = 0; $i < $iterations; $i++) {
    $src = imagecreatefromstring($buffer);
    foreach ($sizes as [$w, $h]) {
        $thumb = imagescale($src, $w, $h);
        ob_start();
        imagejpeg($thumb, null, 80);
        ob_end_clean();
    }
}
$gdTime = (hrtime(true) - $start) / 1e6;
echo sprintf("  GD:        %7.0f ms  (%d thumbnails)\n", $gdTime, $iterations * count($sizes));

// Imagick — read once, clone for each size
if (extension_loaded('imagick')) {
    $start = hrtime(true);
    for ($i = 0; $i < $iterations; $i++) {
        $im = new Imagick();
        $im->readImageBlob($buffer);
        foreach ($sizes as [$w, $h]) {
            $clone = clone $im;
            $clone->thumbnailImage($w, $h, true);
            $clone->setImageCompressionQuality(80);
            $clone->setImageFormat('jpeg');
            $clone->getImageBlob();
            $clone->destroy();
        }
        $im->destroy();
    }
    $imagickTime = (hrtime(true) - $start) / 1e6;
    echo sprintf("  Imagick:   %7.0f ms  (%d thumbnails)\n", $imagickTime, $iterations * count($sizes));
}

echo sprintf("  Speedup vs GD: %.1fx\n", $gdTime / $rustTime);
if (isset($imagickTime)) {
    echo sprintf("  Speedup vs Imagick: %.1fx\n", $imagickTime / $rustTime);
}
unset($imagickTime);

// --- Benchmark 5: Animated GIF resize ---
$gifSource = __DIR__ . '/tests/fixtures/animated.gif';
if (file_exists($gifSource)) {
    $gifIterations = 20;
    echo "\n--- Animated GIF resize ({$gifIterations}x) ---\n";
    echo "Source: 600x600 animated GIF (" . round(filesize($gifSource) / 1024) . " KB)\n";

    // RustImage
    $start = hrtime(true);
    for ($i = 0; $i < $gifIterations; $i++) {
        $image = RustImage\Image::open($gifSource);
        $image->resize(300, 300);
        $image->toGif();
        $image->save("/tmp/bench_rust_gif_{$i}.gif");
    }
    $rustTime = (hrtime(true) - $start) / 1e6;
    echo sprintf("  RustImage: %7.0f ms\n", $rustTime);

    // GD — can't resize animated GIFs
    echo "  GD:        N/A (no animated GIF support)\n";

    // Imagick
    if (extension_loaded('imagick')) {
        $start = hrtime(true);
        for ($i = 0; $i < $gifIterations; $i++) {
            $im = new Imagick($gifSource);
            $im = $im->coalesceImages();
            foreach ($im as $frame) {
                $frame->thumbnailImage(300, 300, true);
            }
            $im->writeImages("/tmp/bench_imagick_gif_{$i}.gif", true);
            $im->destroy();
        }
        $imagickTime = (hrtime(true) - $start) / 1e6;
        echo sprintf("  Imagick:   %7.0f ms\n", $imagickTime);
        echo sprintf("  Speedup vs Imagick: %.1fx\n", $imagickTime / $rustTime);
    }
} else {
    echo "\n--- Animated GIF resize: SKIPPED (tests/fixtures/animated.gif not found) ---\n";
}

// --- Cleanup ---
echo "\n--- Cleanup ---\n";
$cleaned = 0;
foreach (glob('/tmp/bench_*') as $f) { unlink($f); $cleaned++; }
echo "Cleaned up {$cleaned} benchmark files\n";
