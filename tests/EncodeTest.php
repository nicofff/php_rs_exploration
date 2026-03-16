<?php
declare(strict_types=1);

namespace Tests;

use PHPUnit\Framework\TestCase;
use RustImage\Image;

class EncodeTest extends TestCase
{
    private static string $tmpSrc;

    public static function setUpBeforeClass(): void
    {
        self::$tmpSrc = sys_get_temp_dir() . '/rustimage_encode_src.jpg';
        $gd = imagecreatetruecolor(100, 80);
        imagefill($gd, 0, 0, imagecolorallocate($gd, 200, 200, 200));
        imagejpeg($gd, self::$tmpSrc, 95);
        imagedestroy($gd);
    }

    public static function tearDownAfterClass(): void
    {
        @unlink(self::$tmpSrc);
    }

    public function testToPng(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_enc_png.png';
        $img = Image::open(self::$tmpSrc);
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame('png', $info->format);
        $this->assertSame(100, $info->width);
        unlink($tmp);
    }

    public function testToJpegWithQuality(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_enc_jpeg.jpg';
        $img = Image::open(self::$tmpSrc);
        $img->toJpeg(quality: 50);
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame('jpeg', $info->format);
        $this->assertGreaterThan(0, filesize($tmp));
        unlink($tmp);
    }

    public function testToWebp(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_enc_webp.webp';
        $img = Image::open(self::$tmpSrc);
        $img->toWebp(quality: 80);
        $img->save($tmp);
        $this->assertGreaterThan(0, filesize($tmp));
        unlink($tmp);
    }

    public function testToGif(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_enc_gif.gif';
        $img = Image::open(self::$tmpSrc);
        $img->toGif();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame('gif', $info->format);
        unlink($tmp);
    }

    public function testToBufferFromBufferRoundTrip(): void
    {
        $img = Image::open(self::$tmpSrc);
        $img->toPng();
        $bytes = $img->toBuffer();
        $this->assertGreaterThan(0, strlen($bytes));

        $img2 = Image::fromBuffer($bytes);
        $tmp = sys_get_temp_dir() . '/rustimage_enc_roundtrip.png';
        $img2->toPng();
        $img2->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(100, $info->width);
        $this->assertSame(80, $info->height);
        unlink($tmp);
    }

    public function testSaveConfirmsFormatAndDimensions(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_enc_save.png';
        $img = Image::open(self::$tmpSrc);
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame('png', $info->format);
        $this->assertSame(100, $info->width);
        $this->assertSame(80, $info->height);
        unlink($tmp);
    }
}
