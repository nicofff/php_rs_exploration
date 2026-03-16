<?php
declare(strict_types=1);

namespace Tests;

use PHPUnit\Framework\TestCase;
use RustImage\Image;
use RustImage\Rgb;

class OverlayTest extends TestCase
{
    use PixelAssertions;

    public function testOverlayPreservesBaseDimensions(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_overlay_dims.png';
        $base = Image::create(200, 150, new Rgb(255, 255, 255));
        $overlay = Image::create(40, 30, new Rgb(255, 0, 0));
        $base->overlay($overlay, 10, 10);
        $base->toPng();
        $base->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(200, $info->width);
        $this->assertSame(150, $info->height);
        unlink($tmp);
    }

    public function testOverlayChangesPixelInsideOverlayArea(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_overlay_pixel.png';
        // White 100×100 base; fully-opaque red 20×20 overlay placed at (10,10)
        $base = Image::create(100, 100, new Rgb(255, 255, 255));
        $overlay = Image::create(20, 20, new Rgb(255, 0, 0));
        $base->overlay($overlay, 10, 10, opacity: 1.0);
        $base->toPng();
        $base->save($tmp);

        // (15,15) is inside the overlay area → should be red
        $this->assertPixelColor($tmp, 15, 15, 255, 0, 0);
        // (5,5) is outside the overlay area → should remain white
        $this->assertPixelColor($tmp, 5, 5, 255, 255, 255);
        unlink($tmp);
    }

    public function testOverlayDoesNotMutateSource(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_overlay_src.png';
        $base = Image::create(100, 100, new Rgb(255, 255, 255));
        $overlay = Image::create(20, 20, new Rgb(0, 0, 255));
        $base->overlay($overlay, 0, 0);

        // Save the overlay object itself and verify it is unchanged
        $overlay->toPng();
        $overlay->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(20, $info->width);
        $this->assertSame(20, $info->height);
        $this->assertPixelColor($tmp, 0, 0, 0, 0, 255);
        unlink($tmp);
    }
}
