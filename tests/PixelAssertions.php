<?php
declare(strict_types=1);

namespace Tests;

trait PixelAssertions
{
    /**
     * Assert that pixel ($x, $y) in the PNG at $path has the given RGB values (±$tolerance each).
     */
    public function assertPixelColor(
        string $path,
        int $x,
        int $y,
        int $r,
        int $g,
        int $b,
        int $tolerance = 5
    ): void {
        $gd = imagecreatefrompng($path);
        $this->assertNotFalse($gd, "Failed to open PNG: $path");
        $pixel = imagecolorat($gd, $x, $y);
        imagedestroy($gd);

        $pr = ($pixel >> 16) & 0xFF;
        $pg = ($pixel >> 8) & 0xFF;
        $pb = $pixel & 0xFF;

        $this->assertEqualsWithDelta($r, $pr, $tolerance, "Red mismatch at ($x,$y) in $path: expected $r got $pr");
        $this->assertEqualsWithDelta($g, $pg, $tolerance, "Green mismatch at ($x,$y) in $path: expected $g got $pg");
        $this->assertEqualsWithDelta($b, $pb, $tolerance, "Blue mismatch at ($x,$y) in $path: expected $b got $pb");
    }
}
