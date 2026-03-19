<?php
declare(strict_types=1);

trait BenchmarkAssets
{
    private string $sourcePath;
    private string $overlayPath;
    private string $sourceBuffer;
    private string $outPath;

    public function setUp(): void
    {
        $this->sourcePath  = sys_get_temp_dir() . '/phpbench_source.jpg';
        $this->overlayPath = sys_get_temp_dir() . '/phpbench_overlay.png';
        $this->outPath     = sys_get_temp_dir() . '/phpbench_out';

        // 2000×1500 JPEG with random colored rectangles for realistic entropy
        $gd = imagecreatetruecolor(2000, 1500);
        imagefill($gd, 0, 0, imagecolorallocate($gd, 200, 200, 200));
        for ($i = 0; $i < 50; $i++) {
            $color = imagecolorallocate($gd, random_int(0, 255), random_int(0, 255), random_int(0, 255));
            imagefilledrectangle(
                $gd,
                random_int(0, 1900), random_int(0, 1400),
                random_int(0, 1900), random_int(0, 1400),
                $color
            );
        }
        imagejpeg($gd, $this->sourcePath, 90);
        imagedestroy($gd);

        // 200×200 solid red PNG
        $overlay = imagecreatetruecolor(200, 200);
        imagefill($overlay, 0, 0, imagecolorallocate($overlay, 255, 0, 0));
        imagepng($overlay, $this->overlayPath);
        imagedestroy($overlay);

        $this->sourceBuffer = file_get_contents($this->sourcePath);
    }

    public function tearDown(): void
    {
        @unlink($this->sourcePath);
        @unlink($this->overlayPath);
        foreach (glob(sys_get_temp_dir() . '/phpbench_out*') as $f) {
            @unlink($f);
        }
    }
}
