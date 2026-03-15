<?php

$outputDir = __DIR__ . '/demo_output';
if (!is_dir($outputDir)) mkdir($outputDir, 0755, true);

$FIT_MODES = ['contain', 'cover', 'fill'];
$results   = [];
$error     = null;
$hasImagick = extension_loaded('imagick');

// ── Helpers ────────────────────────────────────────────────────────────────

function fmt_kb(int $bytes): string {
    return number_format($bytes / 1024, 1) . ' KB';
}

function fmt_ms(float $ms): string {
    return number_format($ms, 1) . ' ms';
}

// Compute output dimensions after a contain resize (cover/fill are always $tw×$th).
function contain_dims(int $sw, int $sh, int $tw, int $th): array {
    $scale = min($tw / $sw, $th / $sh);
    return [max(1, (int)round($sw * $scale)), max(1, (int)round($sh * $scale))];
}

// Build a small transparent PNG with white "RustImage" text using GD built-in fonts.
// Returns [pngBytes, width, height].
function make_watermark(): array {
    $text = 'RustImage';
    $font = 10;
    $pad  = 7;
    $w    = imagefontwidth($font) * strlen($text) + $pad * 2;
    $h    = imagefontheight($font) + $pad * 2;

    $img = imagecreatetruecolor($w, $h);
    imagealphablending($img, false);
    imagesavealpha($img, true);
    imagefill($img, 0, 0, imagecolorallocatealpha($img, 0, 0, 0, 127)); // transparent

    imagealphablending($img, true);
    // Drop shadow
    imagestring($img, $font, $pad + 1, $pad + 1, $text, imagecolorallocatealpha($img, 0, 0, 0, 60));
    // White text
    imagestring($img, $font, $pad, $pad, $text, imagecolorallocatealpha($img, 255, 255, 255, 15));

    ob_start();
    imagepng($img);
    $png = ob_get_clean();
    imagedestroy($img);

    return [$png, $w, $h];
}

// Imagick: resize $path to $tw × $th using $fit mode, save to $outPath.
function imagick_resize(string $path, int $tw, int $th, string $fit, string $outPath, string $fmt): void {
    $im = new Imagick($path);
    $im->setImageBackgroundColor('white');

    if ($fit === 'fill') {
        $im->resizeImage($tw, $th, Imagick::FILTER_LANCZOS, 1, false);
    } elseif ($fit === 'cover') {
        $sw    = $im->getImageWidth();
        $sh    = $im->getImageHeight();
        $scale = max($tw / $sw, $th / $sh);
        $nw    = (int)round($sw * $scale);
        $nh    = (int)round($sh * $scale);
        $im->resizeImage($nw, $nh, Imagick::FILTER_LANCZOS, 1, false);
        $cx = (int)(($nw - $tw) / 2);
        $cy = (int)(($nh - $th) / 2);
        $im->cropImage($tw, $th, $cx, $cy);
    } else {
        // contain
        $im->resizeImage($tw, $th, Imagick::FILTER_LANCZOS, 1, true);
    }

    $imFmt = match ($fmt) { 'png' => 'png', 'webp' => 'webp', default => 'jpeg' };
    $im->setImageFormat($imFmt);
    if ($fmt !== 'png') $im->setImageCompressionQuality(85);
    $im->writeImage($outPath);
    $im->destroy();
}

// ── Process upload ──────────────────────────────────────────────────────────

if ($_SERVER['REQUEST_METHOD'] === 'POST' && isset($_FILES['image'])) {

    // Clean last run
    foreach (glob("$outputDir/resize_*") as $f) unlink($f);
    foreach (glob("$outputDir/original_resize.*") as $f) unlink($f);

    $upload = $_FILES['image'];
    if ($upload['error'] !== UPLOAD_ERR_OK) {
        $error = "Upload failed (error code {$upload['error']})";
    } else {
        $tw     = max(1, (int)($_POST['width']  ?? 400));
        $th     = max(1, (int)($_POST['height'] ?? 300));
        $fmt    = in_array($_POST['format'] ?? '', ['jpeg','png','webp']) ? $_POST['format'] : 'jpeg';
        $src    = $upload['tmp_name'];
        $origExt = strtolower(pathinfo($upload['name'], PATHINFO_EXTENSION)) ?: 'jpg';

        // Preserve original for display
        $origDst = "$outputDir/original_resize.$origExt";
        copy($src, $origDst);

        foreach ($FIT_MODES as $fit) {

            // — RustImage —
            try {
                $t0  = hrtime(true);
                $img = RustImage\Image::open($src);
                $img->resize($tw, $th, $fit);

                // Watermark: build a transparent PNG with GD, overlay via RustImage
                [$wmPng, $wmW, $wmH] = make_watermark();
                [$srcW, $srcH] = getimagesize($src);
                [$rw, $rh] = ($fit === 'contain')
                    ? contain_dims($srcW, $srcH, $tw, $th)
                    : [$tw, $th];
                $margin = 8;
                $wm = RustImage\Image::fromBuffer($wmPng);
                $img->overlay($wm, max(0, $rw - $wmW - $margin), max(0, $rh - $wmH - $margin), 0.8);

                match ($fmt) {
                    'png'  => $img->toPng(),
                    'webp' => $img->toWebp(85),
                    default => $img->toJpeg(85),
                };
                $out = "$outputDir/resize_{$fit}_rustimage.$fmt";
                $img->save($out);
                $ms  = (hrtime(true) - $t0) / 1e6;
                $info = RustImage\Image::info($out);
                $results[$fit]['RustImage'] = [
                    'file' => basename($out),
                    'time' => $ms,
                    'size' => filesize($out),
                    'dims' => "{$info->width}×{$info->height}",
                ];
            } catch (Throwable $e) {
                $results[$fit]['RustImage'] = ['error' => $e->getMessage()];
            }

            // — ImageMagick —
            if ($hasImagick) {
                try {
                    $t0  = hrtime(true);
                    $out = "$outputDir/resize_{$fit}_imagick.$fmt";
                    imagick_resize($src, $tw, $th, $fit, $out, $fmt);

                    // Watermark: composite the same GD-generated PNG over the Imagick image
                    [$wmPng, $wmW, $wmH] = make_watermark();
                    $im   = new Imagick($out);
                    $iw   = $im->getImageWidth();
                    $ih   = $im->getImageHeight();
                    $wmIm = new Imagick();
                    $wmIm->readImageBlob($wmPng);
                    $margin = 8;
                    $im->compositeImage(
                        $wmIm,
                        Imagick::COMPOSITE_OVER,
                        max(0, $iw - $wmW - $margin),
                        max(0, $ih - $wmH - $margin)
                    );
                    $wmIm->destroy();
                    $imFmt = match ($fmt) { 'png' => 'png', 'webp' => 'webp', default => 'jpeg' };
                    $im->setImageFormat($imFmt);
                    if ($fmt !== 'png') $im->setImageCompressionQuality(85);
                    $im->writeImage($out);
                    $im->destroy();

                    $ms = (hrtime(true) - $t0) / 1e6;
                    $results[$fit]['ImageMagick'] = [
                        'file' => basename($out),
                        'time' => $ms,
                        'size' => filesize($out),
                        'dims' => "{$iw}×{$ih}",
                    ];
                } catch (Throwable $e) {
                    $results[$fit]['ImageMagick'] = ['error' => $e->getMessage()];
                }
            }
        }
    }
}

// ── HTML ──────────────────────────────────────────────────────────────────

$postWidth  = htmlspecialchars($_POST['width']  ?? '400');
$postHeight = htmlspecialchars($_POST['height'] ?? '300');
$postFmt    = $_POST['format'] ?? 'jpeg';

$ENGINES = ['RustImage', 'ImageMagick'];
$ENGINE_COLORS = [
    'RustImage'  => '#b45309',
    'ImageMagick'=> '#1d4ed8',
];
$MODE_DESC = [
    'contain' => 'Shrinks to fit inside the box, preserving aspect ratio — may have empty space',
    'cover'   => 'Fills the box, cropping edges to maintain aspect ratio',
    'fill'    => 'Stretches to exactly fill the box — may distort',
];

?><!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Resize Fit Modes — RustImage vs ImageMagick</title>
<style>
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
    background: #0d0d0d;
    color: #d1d5db;
    padding: 2rem;
    line-height: 1.5;
}

h1 {
    font-size: 1.4rem;
    font-weight: 700;
    color: #f9fafb;
    margin-bottom: 0.25rem;
}
.subtitle {
    font-size: 0.85rem;
    color: #6b7280;
    margin-bottom: 2rem;
}

/* ── Form ── */
.form-bar {
    background: #161616;
    border: 1px solid #262626;
    border-radius: 10px;
    padding: 1.25rem 1.5rem;
    display: flex;
    gap: 1.25rem;
    align-items: flex-end;
    flex-wrap: wrap;
    margin-bottom: 2.5rem;
}
.field { display: flex; flex-direction: column; gap: 0.3rem; }
.field label {
    font-size: 0.72rem;
    font-weight: 600;
    color: #6b7280;
    text-transform: uppercase;
    letter-spacing: 0.06em;
}
.field input, .field select {
    background: #1f1f1f;
    border: 1px solid #303030;
    color: #e5e7eb;
    padding: 0.45rem 0.7rem;
    border-radius: 6px;
    font-size: 0.9rem;
}
.field input[type=number] { width: 80px; }
.field input[type=file]   { width: 240px; }
.btn {
    background: #dc4f26;
    color: #fff;
    border: none;
    padding: 0.5rem 1.4rem;
    border-radius: 6px;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
}
.btn:hover { background: #f05b30; }

/* ── Error ── */
.error-msg {
    background: #2a1515;
    border: 1px solid #6b2020;
    color: #f87171;
    padding: 0.75rem 1rem;
    border-radius: 6px;
    margin-bottom: 1.5rem;
    font-size: 0.9rem;
}

/* ── Original strip ── */
.original-strip {
    display: flex;
    align-items: center;
    gap: 1.5rem;
    background: #161616;
    border: 1px solid #262626;
    border-radius: 10px;
    padding: 1rem 1.25rem;
    margin-bottom: 2.5rem;
}
.original-strip img {
    max-height: 120px;
    max-width: 240px;
    border-radius: 6px;
    object-fit: contain;
    background: #1a1a1a;
}
.original-meta { font-size: 0.85rem; color: #6b7280; }
.original-meta strong { color: #9ca3af; }

/* ── Target label ── */
.target-label {
    margin-bottom: 1.25rem;
    font-size: 0.85rem;
    color: #6b7280;
}
.target-label strong { color: #d1d5db; }

/* ── Fit section ── */
.fit-section { margin-bottom: 3rem; }
.fit-header {
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
    margin-bottom: 1rem;
    padding-bottom: 0.6rem;
    border-bottom: 1px solid #222;
}
.fit-name {
    font-size: 1.05rem;
    font-weight: 700;
    color: #f3f4f6;
    text-transform: uppercase;
    letter-spacing: 0.05em;
}
.fit-desc {
    font-size: 0.8rem;
    color: #6b7280;
}

/* ── Engine cards row ── */
.engine-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 1rem;
}

.card {
    background: #161616;
    border: 1px solid #262626;
    border-radius: 10px;
    overflow: hidden;
}
.card-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.65rem 1rem;
    border-bottom: 1px solid #1e1e1e;
}
.engine-name {
    font-size: 0.85rem;
    font-weight: 700;
    display: flex;
    align-items: center;
    gap: 0.5rem;
}
.engine-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    display: inline-block;
    flex-shrink: 0;
}
.card-time {
    font-size: 0.75rem;
    color: #9ca3af;
    font-variant-numeric: tabular-nums;
}
.card-img-wrap {
    background: repeating-conic-gradient(#1a1a1a 0% 25%, #141414 0% 50%)
                0 0 / 16px 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 160px;
    padding: 0.5rem;
}
.card-img {
    max-width: 100%;
    max-height: 240px;
    display: block;
    border-radius: 2px;
}
.card-foot {
    padding: 0.55rem 1rem;
    font-size: 0.78rem;
    color: #6b7280;
    display: flex;
    gap: 1.25rem;
    border-top: 1px solid #1e1e1e;
}
.card-foot span { font-variant-numeric: tabular-nums; }
.card-error {
    padding: 0.75rem 1rem;
    color: #f87171;
    font-size: 0.82rem;
}

/* ── Legend ── */
.legend {
    display: flex;
    gap: 1.5rem;
    flex-wrap: wrap;
    margin-bottom: 2rem;
}
.legend-item {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.8rem;
    color: #9ca3af;
}
</style>
</head>
<body>

<h1>Resize Fit Modes</h1>
<p class="subtitle">RustImage&ensp;·&ensp;ImageMagick — side-by-side output comparison</p>

<form class="form-bar" method="post" enctype="multipart/form-data">
  <div class="field">
    <label>Image file</label>
    <input type="file" name="image" accept="image/*" required>
  </div>
  <div class="field">
    <label>Target width</label>
    <input type="number" name="width"  value="<?= $postWidth ?>"  min="1" max="4000">
  </div>
  <div class="field">
    <label>Target height</label>
    <input type="number" name="height" value="<?= $postHeight ?>" min="1" max="4000">
  </div>
  <div class="field">
    <label>Output format</label>
    <select name="format">
      <option value="jpeg" <?= $postFmt === 'jpeg' ? 'selected' : '' ?>>JPEG</option>
      <option value="webp" <?= $postFmt === 'webp'  ? 'selected' : '' ?>>WebP</option>
      <option value="png"  <?= $postFmt === 'png'   ? 'selected' : '' ?>>PNG</option>
    </select>
  </div>
  <button class="btn" type="submit">Generate</button>
</form>

<?php if ($error): ?>
  <div class="error-msg"><?= htmlspecialchars($error) ?></div>
<?php endif; ?>

<?php if ($results): ?>

<?php if (isset($origDst)): ?>
<div class="original-strip">
  <img src="demo_output/<?= basename($origDst) ?>?<?= time() ?>" alt="Original">
  <div class="original-meta">
    <strong>Original</strong> &mdash; <?= htmlspecialchars($upload['name']) ?><br>
    <?= fmt_kb(filesize($origDst)) ?> &mdash; <?= implode('×', array_slice(getimagesize($origDst) ?: [0,0], 0, 2)) ?> px
  </div>
</div>
<?php endif; ?>

<div class="target-label">
  Target: <strong><?= $tw ?> × <?= $th ?> px</strong>
  &ensp;·&ensp; Format: <strong><?= strtoupper($fmt) ?></strong>
</div>

<div class="legend">
<?php foreach ($ENGINE_COLORS as $eng => $col): ?>
  <div class="legend-item">
    <span class="engine-dot" style="background:<?= $col ?>"></span>
    <?= $eng ?>
    <?php if ($eng === 'ImageMagick' && !$hasImagick): ?>
      <em style="color:#6b7280">(not installed)</em>
    <?php endif; ?>
  </div>
<?php endforeach; ?>
</div>

<?php foreach ($FIT_MODES as $fit):
    $row = $results[$fit] ?? [];
?>
<div class="fit-section">
  <div class="fit-header">
    <span class="fit-name"><?= $fit ?></span>
    <span class="fit-desc"><?= $MODE_DESC[$fit] ?></span>
  </div>
  <div class="engine-row">
  <?php foreach ($ENGINES as $eng):
      if ($eng === 'ImageMagick' && !$hasImagick) continue;
      $r = $row[$eng] ?? null;
  ?>
    <div class="card">
      <div class="card-head">
        <span class="engine-name">
          <span class="engine-dot" style="background:<?= $ENGINE_COLORS[$eng] ?>"></span>
          <?= $eng ?>
        </span>
        <?php if (isset($r['time'])): ?>
          <span class="card-time"><?= fmt_ms($r['time']) ?></span>
        <?php endif; ?>
      </div>
      <?php if (!$r): ?>
        <div class="card-error">No result</div>
      <?php elseif (isset($r['error'])): ?>
        <div class="card-error"><?= htmlspecialchars($r['error']) ?></div>
      <?php else: ?>
        <div class="card-img-wrap">
          <img class="card-img"
               src="demo_output/<?= $r['file'] ?>?<?= time() ?>"
               alt="<?= $fit ?> <?= $eng ?>">
        </div>
        <div class="card-foot">
          <span><?= $r['dims'] ?> px</span>
          <span><?= fmt_kb($r['size']) ?></span>
        </div>
      <?php endif; ?>
    </div>
  <?php endforeach; ?>
  </div>
</div>
<?php endforeach; ?>

<?php endif; // $results ?>

</body>
</html>
