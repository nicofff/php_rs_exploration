use image::DynamicImage;

pub fn overlay_frame(base: &image::Frame, overlay_img: &DynamicImage, x: i32, y: i32, opacity: f32) -> image::Frame {
    let mut result = base.buffer().clone();
    let overlay_rgba = overlay_img.to_rgba8();
    let ow = overlay_rgba.width() as i32;
    let oh = overlay_rgba.height() as i32;
    let bw = result.width() as i32;
    let bh = result.height() as i32;

    for oy in 0..oh {
        for ox in 0..ow {
            let bx = x + ox;
            let by = y + oy;
            if bx >= 0 && bx < bw && by >= 0 && by < bh {
                let src = overlay_rgba.get_pixel(ox as u32, oy as u32);
                let dst = result.get_pixel(bx as u32, by as u32);

                let src_a = (src.0[3] as f32 / 255.0) * opacity;
                let dst_a = dst.0[3] as f32 / 255.0;
                let out_a = src_a + dst_a * (1.0 - src_a);

                let blend = |s: u8, d: u8| -> u8 {
                    if out_a == 0.0 { return 0; }
                    ((s as f32 * src_a + d as f32 * dst_a * (1.0 - src_a)) / out_a) as u8
                };

                result.put_pixel(bx as u32, by as u32, image::Rgba([
                    blend(src.0[0], dst.0[0]),
                    blend(src.0[1], dst.0[1]),
                    blend(src.0[2], dst.0[2]),
                    (out_a * 255.0) as u8,
                ]));
            }
        }
    }

    image::Frame::from_parts(result, base.left(), base.top(), base.delay())
}
