<?php
declare(strict_types=1);

if (!class_exists('RustImage\Image')) {
    fwrite(STDERR, "ERROR: RustImage extension not loaded.\n");
    fwrite(STDERR, "Run via: make phptest  (passes -d extension=... automatically)\n");
    exit(1);
}
