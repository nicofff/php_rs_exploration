<?php
declare(strict_types=1);

namespace Tests;

use PHPUnit\Framework\TestCase;
use RustImage\Image;
use RustImage\Rgb;

class TransformTest extends TestCase
{
    use PixelAssertions;

    /**
     * Create a 10×10 white PNG with a single red pixel at (0, 0).
     * Returns the path to the saved file.
     */
    private static function makeRedDotImage(): string
    {
        $path = sys_get_temp_dir() . '/rustimage_transform_src_' . uniqid() . '.png';
        $gd = imagecreatetruecolor(10, 10);
        imagefill($gd, 0, 0, imagecolorallocate($gd, 255, 255, 255));
        imagesetpixel($gd, 0, 0, imagecolorallocate($gd, 255, 0, 0));
        imagepng($gd, $path);
        imagedestroy($gd);
        return $path;
    }

    public function testFlipMovesRedPixelToBottomLeft(): void
    {
        $src = self::makeRedDotImage(); // red at (0,0), white elsewhere
        $out = sys_get_temp_dir() . '/rustimage_flip_out.png';

        $img = Image::open($src);
        $img->flip(); // vertical flip: row 0 ↔ row 9
        $img->toPng();
        $img->save($out);

        $this->assertPixelColor($out, 0, 9, 255, 0, 0);   // red moved to (0,9)
        $this->assertPixelColor($out, 0, 0, 255, 255, 255); // (0,0) is now white

        unlink($src);
        unlink($out);
    }

    public function testMirrorMovesRedPixelToTopRight(): void
    {
        $src = self::makeRedDotImage(); // red at (0,0)
        $out = sys_get_temp_dir() . '/rustimage_mirror_out.png';

        $img = Image::open($src);
        $img->mirror(); // horizontal flip: col 0 ↔ col 9
        $img->toPng();
        $img->save($out);

        $this->assertPixelColor($out, 9, 0, 255, 0, 0);    // red moved to (9,0)
        $this->assertPixelColor($out, 0, 0, 255, 255, 255); // (0,0) is now white

        unlink($src);
        unlink($out);
    }

    public function testAutoRotateNoExifIsNoOp(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_autorotate_noop.png';
        // Image::create produces no EXIF; autoRotate should be a no-op
        $img = Image::create(100, 50, new Rgb(255, 255, 255));
        $img->autoRotate();
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(100, $info->width);
        $this->assertSame(50, $info->height);
        unlink($tmp);
    }

    public function testAutoRotateOrientation6Rotates90CW(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_autorotate_rot.png';
        // exif_rotated.jpg: stored as 100×50 with EXIF orientation=6 (needs 90° CW rotation)
        // After autoRotate(), the visual size becomes 50×100.
        $img = Image::open(__DIR__ . '/fixtures/exif_rotated.jpg');
        $img->autoRotate();
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(50, $info->width);
        $this->assertSame(100, $info->height);
        unlink($tmp);
    }

    public function testAutoRotateIsIdempotentOnSameObject(): void
    {
        $tmp = sys_get_temp_dir() . '/rustimage_autorotate_idem.png';
        $img = Image::open(__DIR__ . '/fixtures/exif_rotated.jpg');
        $img->autoRotate(); // applies rotation; internally sets orientation = Some(1)
        $img->autoRotate(); // Some(1) triggers early return — no second rotation
        $img->toPng();
        $img->save($tmp);
        $info = Image::info($tmp);
        $this->assertSame(50, $info->width);
        $this->assertSame(100, $info->height);
        unlink($tmp);
    }
}
