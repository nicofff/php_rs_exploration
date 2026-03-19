<?php
declare(strict_types=1);

require_once __DIR__ . '/BenchmarkAssets.php';

use PhpBench\Attributes as Bench;
use RustImage\Image;

#[Bench\BeforeMethods(['setUp'])]
#[Bench\AfterMethods(['tearDown'])]
#[Bench\ParamProviders(['provideSizes'])]
class ResizeBench
{
    use BenchmarkAssets;

    public function provideSizes(): \Generator
    {
        yield 'large 800x600' => ['width' => 800, 'height' => 600];
        yield 'small 200x150' => ['width' => 200, 'height' => 150];
    }

    public function benchRustImage(array $params): void
    {
        $img = Image::open($this->sourcePath);
        $img->resize($params['width'], $params['height']);
        $img->toJpeg(quality: 80);
        $img->save($this->outPath . '.jpg');
    }

    public function benchGd(array $params): void
    {
        $src     = imagecreatefromjpeg($this->sourcePath);
        $resized = imagescale($src, $params['width'], $params['height']);
        imagejpeg($resized, $this->outPath . '.jpg', 80);
        imagedestroy($src);
        imagedestroy($resized);
    }

    public function benchImagick(array $params): void
    {
        $im = new Imagick($this->sourcePath);
        $im->resizeImage($params['width'], $params['height'], Imagick::FILTER_CATROM, 1, true);
        $im->setImageCompressionQuality(80);
        $im->writeImage($this->outPath . '.jpg');
        $im->destroy();
    }
}
