<?php
declare(strict_types=1);

require_once __DIR__ . '/BenchmarkAssets.php';

use PhpBench\Attributes as Bench;
use RustImage\Image;

#[Bench\BeforeMethods(['setUp'])]
#[Bench\AfterMethods(['tearDown'])]
#[Bench\ParamProviders(['provideFormats'])]
class ConvertBench
{
    use BenchmarkAssets;

    public function provideFormats(): \Generator
    {
        yield 'webp' => ['format' => 'webp', 'quality' => 80];
        yield 'png'  => ['format' => 'png'];
        yield 'gif'  => ['format' => 'gif'];
    }

    public function benchRustImage(array $params): void
    {
        $img = Image::open($this->sourcePath);
        match ($params['format']) {
            'webp' => $img->toWebp($params['quality'] ?? 80),
            'png'  => $img->toPng(),
            'gif'  => $img->toGif(),
        };
        $img->toBuffer();
    }

    public function benchGd(array $params): void
    {
        $gd = imagecreatefromjpeg($this->sourcePath);
        ob_start();
        match ($params['format']) {
            'webp' => imagewebp($gd, null, $params['quality'] ?? 80),
            'png'  => imagepng($gd, null),
            'gif'  => imagegif($gd, null),
        };
        ob_get_clean();
        imagedestroy($gd);
    }

    public function benchImagick(array $params): void
    {
        $im = new Imagick($this->sourcePath);
        $im->setImageFormat($params['format']);
        if (isset($params['quality'])) {
            $im->setImageCompressionQuality($params['quality']);
        }
        $im->getImageBlob();
        $im->destroy();
    }
}
