use crate::image_error::ImageError;
use crate::image::Frame;

pub fn is_gif(path: &str) -> bool {
    // Check extension first
    if path.to_lowercase().ends_with(".gif") {
        return true;
    }
    // Fall back to magic bytes
    if let Ok(mut f) = std::fs::File::open(path) {
        let mut header = [0u8; 6];
        if std::io::Read::read_exact(&mut f, &mut header).is_ok() {
            return &header[..3] == b"GIF";
        }
    }
    false
}

pub fn decode_gif_frames(path: &str) -> Result<Vec<Frame>, ImageError> {
    use std::fs::File;
    use std::io::BufReader;
    use image::RgbaImage;

    let file = File::open(path)
        .map_err(|e| ImageError(format!("Failed to open GIF '{}': {}", path, e)))?;
    let reader = BufReader::new(file);

    let mut decode_opts = gif::DecodeOptions::new();
    decode_opts.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = decode_opts.read_info(reader)
        .map_err(|e| ImageError(format!("Failed to decode GIF '{}': {}", path, e)))?;

    let canvas_w = decoder.width() as u32;
    let canvas_h = decoder.height() as u32;

    let mut frames = Vec::new();
    // Keep a persistent canvas for frame compositing
    let mut canvas = vec![0u8; (canvas_w * canvas_h * 4) as usize];

    while let Some(gif_frame) = decoder.read_next_frame()
        .map_err(|e| ImageError(format!("Failed to read GIF frame: {}", e)))? {

        let fw = gif_frame.width as u32;
        let fh = gif_frame.height as u32;
        let fl = gif_frame.left as u32;
        let ft = gif_frame.top as u32;

        // Composite frame onto canvas
        for y in 0..fh {
            for x in 0..fw {
                let src_idx = ((y * fw + x) * 4) as usize;
                let dst_x = fl + x;
                let dst_y = ft + y;
                if dst_x < canvas_w && dst_y < canvas_h {
                    let dst_idx = ((dst_y * canvas_w + dst_x) * 4) as usize;
                    let alpha = gif_frame.buffer[src_idx + 3];
                    if alpha > 0 {
                        canvas[dst_idx]     = gif_frame.buffer[src_idx];
                        canvas[dst_idx + 1] = gif_frame.buffer[src_idx + 1];
                        canvas[dst_idx + 2] = gif_frame.buffer[src_idx + 2];
                        canvas[dst_idx + 3] = gif_frame.buffer[src_idx + 3];
                    }
                }
            }
        }

        let delay_ms = gif_frame.delay as u32 * 10;
        let buffer = RgbaImage::from_raw(canvas_w, canvas_h, canvas.clone())
            .ok_or_else(|| ImageError("Failed to create RGBA image from GIF frame".into()))?;

        frames.push(Frame { buffer, delay_ms });
    }

    if frames.is_empty() {
        return Err(ImageError("GIF contains no frames".into()));
    }

    Ok(frames)
}

pub fn decode_static_from_path(path: &str) -> Result<Vec<Frame>, ImageError> {
    let reader = image::ImageReader::open(path)
        .map_err(|e| ImageError(format!("Failed to open image '{}': {}", path, e)))?
        .with_guessed_format()
        .map_err(|e| ImageError(format!("Failed to guess format for '{}': {}", path, e)))?;
    let img = reader.decode()
        .map_err(|e| ImageError(format!("Failed to decode image '{}': {}", path, e)))?;
    Ok(vec![Frame {
        buffer: img.to_rgba8(),
        delay_ms: 0,
    }])
}

pub fn decode_static_from_buffer(bytes: &[u8]) -> Result<Vec<Frame>, ImageError> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| ImageError(format!("Failed to decode image from buffer: {}", e)))?;
    Ok(vec![Frame {
        buffer: img.to_rgba8(),
        delay_ms: 0,
    }])
}
