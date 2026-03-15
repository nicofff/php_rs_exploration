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
echo "=== Done ===\n";
