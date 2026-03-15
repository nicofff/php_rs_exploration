<?php
declare(strict_types=1);

namespace Tests;

use PHPUnit\Framework\TestCase;
use RustImage\Image;

class ResizeTest extends TestCase
{
    private static string $tmpSrc;

    public static function setUpBeforeClass(): void
    {
        // 200x150 source image
        self::$tmpSrc = sys_get_temp_dir() . '/rustimage_resize_src.jpg';
        $gd = imagecreatetruecolor(200, 150);
        imagefill($gd, 0, 0, imagecolorallocate($gd, 200, 200, 200));
        imagejpeg($gd, self::$tmpSrc, 95);
        imagedestroy($gd);
    }

    public static function tearDownAfterClass(): void
    {
        @unlink(self::$tmpSrc);
    }

    public function testContainPreservesAspectRatio(): void
    {
        // 200x150 contained in 100x100 → 100x75
        $tmp = sys_get_temp_dir() . '/rustimage_resize_contain.png';
        $img = Image::open(self::$tmpSrc);
        $img->resize(100, 100); // 'contain' is the default
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(100, $info->width);
        $this->assertSame(75, $info->height);
        unlink($tmp);
    }

    public function testCoverProducesExactSize(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_resize_cover.png';
        $img = Image::open(self::$tmpSrc);
        $img->resize(100, 100, fit: 'cover');
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(100, $info->width);
        $this->assertSame(100, $info->height);
        unlink($tmp);
    }

    public function testFillProducesExactSize(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_resize_fill.png';
        $img = Image::open(self::$tmpSrc);
        $img->resize(100, 100, fit: 'fill');
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(100, $info->width);
        $this->assertSame(100, $info->height);
        unlink($tmp);
    }

    public function testJpegOutputFormat(): void
    {
        // 200×150 contain into 80×60: scale = min(80/200, 60/150) = 0.4, output = 80×60
        $tmp = sys_get_temp_dir() . '/rustimage_resize_out.jpg';
        $img = Image::open(self::$tmpSrc);
        $img->resize(80, 60);
        $img->toJpeg(quality: 70);
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame('jpeg', $info->format);
        $this->assertSame(80, $info->width);
        $this->assertSame(60, $info->height);
        $this->assertGreaterThan(0, filesize($tmp));
        unlink($tmp);
    }
}
