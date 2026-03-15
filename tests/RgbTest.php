<?php
declare(strict_types=1);

namespace Tests;

use PHPUnit\Framework\TestCase;
use RustImage\Rgb;

class RgbTest extends TestCase
{
    public function testConstructorAndGetters(): void
    {
        $rgb = new Rgb(100, 150, 200);
        $this->assertSame(100, $rgb->r);
        $this->assertSame(150, $rgb->g);
        $this->assertSame(200, $rgb->b);
    }

    public function testBlackBoundary(): void
    {
        $rgb = new Rgb(0, 0, 0);
        $this->assertSame(0, $rgb->r);
        $this->assertSame(0, $rgb->g);
        $this->assertSame(0, $rgb->b);
    }

    public function testWhiteBoundary(): void
    {
        $rgb = new Rgb(255, 255, 255);
        $this->assertSame(255, $rgb->r);
        $this->assertSame(255, $rgb->g);
        $this->assertSame(255, $rgb->b);
    }
}
