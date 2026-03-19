<?php
declare(strict_types=1);

require_once __DIR__ . '/BenchmarkAssets.php';

use PhpBench\Attributes as Bench;
use RustImage\Image;

#[Bench\BeforeMethods(['setUp'])]
#[Bench\AfterMethods(['tearDown'])]
class OverlayBench
{
    use BenchmarkAssets;

    public function benchRustImage(): void
    {
        $base    = Image::open($this->sourcePath);
        $overlay = Image::open($this->overlayPath);
        $base->overlay($overlay, 100, 100);
        $base->toJpeg(quality: 80);
        $base->toBuffer();
    }

    public function benchGd(): void
    {
        $base    = imagecreatefromjpeg($this->sourcePath);
        $overlay = imagecreatefrompng($this->overlayPath);
        imagecopy($base, $overlay, 100, 100, 0, 0, 200, 200);
        ob_start();
        imagejpeg($base, null, 80);
        ob_get_clean();
        imagedestroy($base);
        imagedestroy($overlay);
    }

    public function benchImagick(): void
    {
        $base    = new Imagick($this->sourcePath);
        $overlay = new Imagick($this->overlayPath);
        $base->compositeImage($overlay, Imagick::COMPOSITE_OVER, 100, 100);
        $base->setImageCompressionQuality(80);
        $base->getImageBlob();
        $base->destroy();
        $overlay->destroy();
    }
}
