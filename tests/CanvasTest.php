<?php
declare(strict_types=1);

namespace Tests;

use PHPUnit\Framework\TestCase;
use RustImage\Image;
use RustImage\ImageException;
use RustImage\Rgb;

class CanvasTest extends TestCase
{
    use PixelAssertions;

    private function tmp(string $suffix = ''): string
    {
        return sys_get_temp_dir() . '/rustimage_canvas_' . $suffix . '.png';
    }

    public function testCreateDimensions(): void
    {
        $tmp = $this->tmp('dims');
        $img = Image::create(120, 80, new Rgb(0, 0, 255));
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(120, $info->width);
        $this->assertSame(80, $info->height);
        unlink($tmp);
    }

    public function testCreateFillColorAtMultiplePoints(): void
    {
        $tmp = $this->tmp('fill');
        $img = Image::create(50, 50, new Rgb(255, 128, 0));
        $img->toPng();
        $img->save($tmp);
        $this->assertPixelColor($tmp, 0, 0, 255, 128, 0);
        $this->assertPixelColor($tmp, 25, 25, 255, 128, 0);
        $this->assertPixelColor($tmp, 49, 49, 255, 128, 0);
        unlink($tmp);
    }

    public function testCreateZeroWidthThrows(): void
    {
        $this->expectException(ImageException::class);
        Image::create(0, 100, new Rgb(0, 0, 0));
    }

    public function testCreateZeroHeightThrows(): void
    {
        $this->expectException(ImageException::class);
        Image::create(100, 0, new Rgb(0, 0, 0));
    }

    public function testCreateNegativeWidthThrows(): void
    {
        $this->expectException(ImageException::class);
        Image::create(-1, 100, new Rgb(0, 0, 0));
    }

    public function testCreateNegativeHeightThrows(): void
    {
        $this->expectException(ImageException::class);
        Image::create(100, -1, new Rgb(0, 0, 0));
    }
}
