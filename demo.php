<?php

$outputDir = __DIR__ . '/demo_output';
if (!is_dir($outputDir)) mkdir($outputDir);

// Clean old outputs on fresh load
if ($_SERVER['REQUEST_METHOD'] === 'GET') {
    foreach (glob("$outputDir/*") as $f) unlink($f);
}

$results = [];

if ($_SERVER['REQUEST_METHOD'] === 'POST' && isset($_FILES['image'])) {
    $upload = $_FILES['image'];
    if ($upload['error'] !== UPLOAD_ERR_OK) {
        $error = "Upload failed (error code {$upload['error']})";
    } else {
        $width = max(1, (int)($_POST['width'] ?? 200));
        $height = max(1, (int)($_POST['height'] ?? 200));
        $format = $_POST['format'] ?? 'jpeg';
        $sourcePath = $upload['tmp_name'];
        $originalName = $upload['name'];
        $originalSize = filesize($sourcePath);

        // Save original for display
        $origExt = pathinfo($originalName, PATHINFO_EXTENSION) ?: 'jpg';
        $origCopy = "$outputDir/original.$origExt";
        copy($sourcePath, $origCopy);

        // --- RustImage ---
        try {
            $start = hrtime(true);
            $image = RustImage\Image::open($sourcePath);
            $image->resize($width, $height);
            $outFile = "$outputDir/rustimage.$format";
            match ($format) {
                'webp' => $image->toWebp(quality: 80),
                'png' => $image->toPng(),
                default => $image->toJpeg(quality: 85),
            };
            $image->save($outFile);
            $ms = (hrtime(true) - $start) / 1e6;
            $info = RustImage\Image::info($outFile);
            $results['RustImage'] = [
                'file' => basename($outFile),
                'time' => $ms,
                'size' => filesize($outFile),
                'dims' => "{$info->width}x{$info->height}",
            ];
        } catch (Throwable $e) {
            $results['RustImage'] = ['error' => $e->getMessage()];
        }

        // --- GD ---
        try {
            $start = hrtime(true);
            $src = match (strtolower($origExt)) {
                'png' => imagecreatefrompng($sourcePath),
                'gif' => imagecreatefromgif($sourcePath),
                'webp' => imagecreatefromwebp($sourcePath),
                default => imagecreatefromjpeg($sourcePath),
            };
            if (!$src) throw new Exception("GD failed to open image");

            $srcW = imagesx($src);
            $srcH = imagesy($src);
            $ratio = min($width / $srcW, $height / $srcH);
            $newW = (int)round($srcW * $ratio);
            $newH = (int)round($srcH * $ratio);
            $resized = imagescale($src, $newW, $newH);

            $outFile = "$outputDir/gd.$format";
            match ($format) {
                'webp' => imagewebp($resized, $outFile, 80),
                'png' => imagepng($resized, $outFile),
                default => imagejpeg($resized, $outFile, 85),
            };
            $ms = (hrtime(true) - $start) / 1e6;
            $results['GD'] = [
                'file' => basename($outFile),
                'time' => $ms,
                'size' => filesize($outFile),
                'dims' => imagesx($resized) . "x" . imagesy($resized),
            ];
        } catch (Throwable $e) {
            $results['GD'] = ['error' => $e->getMessage()];
        }

        // --- Imagick ---
        if (extension_loaded('imagick')) {
            try {
                $start = hrtime(true);
                $im = new Imagick($sourcePath);
                $im->resizeImage($width, $height, Imagick::FILTER_LANCZOS, 1, true);
                $outFile = "$outputDir/imagick.$format";
                $im->setImageFormat($format === 'png' ? 'png' : ($format === 'webp' ? 'webp' : 'jpeg'));
                if ($format === 'webp') {
                    $im->setImageCompressionQuality(80);
                } elseif ($format !== 'png') {
                    $im->setImageCompressionQuality(85);
                }
                $im->writeImage($outFile);
                $ms = (hrtime(true) - $start) / 1e6;
                $iw = $im->getImageWidth();
                $ih = $im->getImageHeight();
                $im->destroy();
                $results['Imagick'] = [
                    'file' => basename($outFile),
                    'time' => $ms,
                    'size' => filesize($outFile),
                    'dims' => "{$iw}x{$ih}",
                ];
            } catch (Throwable $e) {
                $results['Imagick'] = ['error' => $e->getMessage()];
            }
        }
    }
}

$fastest = null;
if ($results) {
    $times = array_filter(array_map(fn($r) => $r['time'] ?? null, $results));
    if ($times) $fastest = array_search(min($times), $times);
}

?><!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>RustImage Demo</title>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body { font-family: -apple-system, system-ui, sans-serif; background: #0f0f0f; color: #e0e0e0; padding: 2rem; }
  h1 { margin-bottom: 1.5rem; font-size: 1.5rem; }
  .upload-form { background: #1a1a1a; padding: 1.5rem; border-radius: 8px; margin-bottom: 2rem; display: flex; gap: 1rem; align-items: end; flex-wrap: wrap; }
  .field { display: flex; flex-direction: column; gap: 0.3rem; }
  .field label { font-size: 0.8rem; color: #888; text-transform: uppercase; letter-spacing: 0.05em; }
  .field input, .field select { background: #2a2a2a; border: 1px solid #333; color: #e0e0e0; padding: 0.5rem 0.75rem; border-radius: 4px; font-size: 0.9rem; }
  .field input[type=number] { width: 80px; }
  button { background: #e44d26; color: white; border: none; padding: 0.6rem 1.5rem; border-radius: 4px; cursor: pointer; font-size: 0.9rem; font-weight: 600; }
  button:hover { background: #ff6040; }
  .results { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 1.5rem; }
  .card { background: #1a1a1a; border-radius: 8px; overflow: hidden; }
  .card-header { padding: 1rem; display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid #2a2a2a; }
  .card-header h3 { font-size: 1rem; }
  .badge { font-size: 0.75rem; padding: 0.2rem 0.5rem; border-radius: 3px; background: #2a2a2a; }
  .badge.fastest { background: #2d5a27; color: #7ddf64; }
  .card-img { width: 100%; display: block; background: #222 url('data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><rect width="10" height="10" fill="%23282828"/><rect x="10" y="10" width="10" height="10" fill="%23282828"/></svg>') repeat; min-height: 150px; object-fit: contain; }
  .card-meta { padding: 1rem; font-size: 0.85rem; color: #888; display: flex; gap: 1rem; }
  .card-meta span { display: flex; align-items: center; gap: 0.3rem; }
  .error { color: #ff6b6b; padding: 1rem; }
  .original { margin-bottom: 1.5rem; }
  .original img { max-width: 300px; max-height: 200px; border-radius: 4px; }
  .original-info { font-size: 0.85rem; color: #888; margin-top: 0.5rem; }
</style>
</head>
<body>

<h1>RustImage vs GD vs Imagick</h1>

<form class="upload-form" method="post" enctype="multipart/form-data">
  <div class="field">
    <label>Image</label>
    <input type="file" name="image" accept="image/*" required>
  </div>
  <div class="field">
    <label>Width</label>
    <input type="number" name="width" value="<?= htmlspecialchars($_POST['width'] ?? '400') ?>" min="1" max="4000">
  </div>
  <div class="field">
    <label>Height</label>
    <input type="number" name="height" value="<?= htmlspecialchars($_POST['height'] ?? '400') ?>" min="1" max="4000">
  </div>
  <div class="field">
    <label>Format</label>
    <select name="format">
      <option value="jpeg" <?= ($_POST['format'] ?? '') === 'jpeg' ? 'selected' : '' ?>>JPEG</option>
      <option value="webp" <?= ($_POST['format'] ?? '') === 'webp' ? 'selected' : '' ?>>WebP</option>
      <option value="png" <?= ($_POST['format'] ?? '') === 'png' ? 'selected' : '' ?>>PNG</option>
    </select>
  </div>
  <button type="submit">Resize</button>
</form>

<?php if (isset($error)): ?>
  <div class="error"><?= htmlspecialchars($error) ?></div>
<?php endif; ?>

<?php if ($results): ?>

<?php if (isset($origCopy)): ?>
<div class="original">
  <h3 style="margin-bottom: 0.5rem;">Original</h3>
  <img src="demo_output/<?= basename($origCopy) ?>">
  <div class="original-info"><?= htmlspecialchars($originalName) ?> &mdash; <?= round($originalSize / 1024) ?> KB</div>
</div>
<?php endif; ?>

<div class="results">
<?php foreach ($results as $engine => $r): ?>
  <div class="card">
    <div class="card-header">
      <h3><?= $engine ?></h3>
      <?php if (isset($r['time'])): ?>
        <span class="badge <?= $engine === $fastest ? 'fastest' : '' ?>">
          <?= number_format($r['time'], 1) ?> ms
          <?= $engine === $fastest ? ' ★' : '' ?>
        </span>
      <?php endif; ?>
    </div>
    <?php if (isset($r['error'])): ?>
      <div class="error"><?= htmlspecialchars($r['error']) ?></div>
    <?php else: ?>
      <img class="card-img" src="demo_output/<?= $r['file'] ?>">
      <div class="card-meta">
        <span><?= $r['dims'] ?></span>
        <span><?= round($r['size'] / 1024, 1) ?> KB</span>
      </div>
    <?php endif; ?>
  </div>
<?php endforeach; ?>
</div>

<?php endif; ?>

</body>
</html>
