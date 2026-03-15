<?php
declare(strict_types=1);

namespace Tests;

use PHPUnit\Framework\TestCase;
use RustImage\Image;
use RustImage\ImageException;
use RustImage\Rgb;

class CropTest extends TestCase
{
    use PixelAssertions;

    private static string $tmpSrc;

    public static function setUpBeforeClass(): void
    {
        // 200x150 white image with a 50x50 red square at top-left (0,0)–(49,49)
        self::$tmpSrc = sys_get_temp_dir() . '/rustimage_crop_src.jpg';
        $gd = imagecreatetruecolor(200, 150);
        imagefill($gd, 0, 0, imagecolorallocate($gd, 255, 255, 255));
        imagefilledrectangle($gd, 0, 0, 49, 49, imagecolorallocate($gd, 255, 0, 0));
        imagejpeg($gd, self::$tmpSrc, 100);
        imagedestroy($gd);
    }

    public static function tearDownAfterClass(): void
    {
        @unlink(self::$tmpSrc);
    }

    public function testCropDimensions(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_crop_dims.png';
        $img = Image::open(self::$tmpSrc);
        $img->crop(0, 0, 100, 80);
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(100, $info->width);
        $this->assertSame(80, $info->height);
        unlink($tmp);
    }

    public function testCropOffsetExcludesRedSquare(): void
    {
        // Crop starting at x=100 — the red square only occupies x=0..49
        $tmp = sys_get_temp_dir() . '/rustimage_crop_offset.png';
        $img = Image::open(self::$tmpSrc);
        $img->crop(100, 0, 100, 150);
        $img->toPng();
        $img->save($tmp);
        // pixel at (0,0) in the cropped image was at (100,0) in the source — should be white
        $this->assertPixelColor($tmp, 0, 0, 255, 255, 255);
        unlink($tmp);
    }

    public function testCropOutOfBoundsThrows(): void
    {
        $img = Image::create(200, 150, new Rgb(255, 255, 255));
        $this->expectException(ImageException::class);
        $img->crop(0, 0, 300, 300);
    }

    public function testCropNegativeXThrows(): void
    {
        $img = Image::create(200, 150, new Rgb(255, 255, 255));
        $this->expectException(ImageException::class);
        $img->crop(-1, 0, 100, 100);
    }

    public function testCropNegativeYThrows(): void
    {
        $img = Image::create(200, 150, new Rgb(255, 255, 255));
        $this->expectException(ImageException::class);
        $img->crop(0, -1, 100, 100);
    }

    public function testAnimatedGifCropDimensions(): void
    {
        $fixture = __DIR__ . '/fixtures/animated.gif';
        $tmp = sys_get_temp_dir() . '/rustimage_crop_anim.gif';
        $gifInfo = Image::info($fixture);
        $cropW = (int)($gifInfo->width / 2);
        $cropH = (int)($gifInfo->height / 2);

        $gif = Image::open($fixture);
        $gif->crop(0, 0, $cropW, $cropH);
        $gif->toGif();
        $gif->save($tmp);

        $outInfo = Image::info($tmp);
        $this->assertSame($cropW, $outInfo->width);
        $this->assertSame($cropH, $outInfo->height);
        unlink($tmp);
    }
}
