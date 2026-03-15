<?php

$gifSource = __DIR__ . '/tests/fixtures/animated.gif';
$outDir = '/tmp/rustimage_gif_test';
@mkdir($outDir);

echo "Source: $gifSource\n";
$info = RustImage\Image::info($gifSource);
echo "Info: {$info->width}x{$info->height} {$info->format} animated=" . var_export($info->is_animated, true) . "\n\n";

$sizes = [
    [300, 300],
    [150, 150],
    [50, 50],
];

$original = RustImage\Image::open($gifSource);

foreach ($sizes as [$w, $h]) {
    $out = "$outDir/resized_{$w}x{$h}.gif";
    $copy = $original->copy();
    $copy->resize($w, $h);
    $copy->toGif();
    $copy->save($out);
    echo "Saved {$w}x{$h}: $out (" . round(filesize($out) / 1024) . " KB)\n";
}

// Also test WebP output
$outWebp = "$outDir/resized_300x300.webp";
$copy = $original->copy();
$copy->resize(300, 300);
$copy->toWebp(quality: 80);
$copy->save($outWebp);
echo "Saved WebP: $outWebp (" . round(filesize($outWebp) / 1024) . " KB)\n";

echo "\nOpen $outDir to inspect the results.\n";
