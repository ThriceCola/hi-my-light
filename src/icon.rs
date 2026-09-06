use std::io::Cursor;

use image::{ImageFormat, Rgba, RgbaImage};

/// 与托盘 StatusNotifier 同一套圆：实心琥珀核 + 一圈淡边。
/// 32×32 时核半径 11、光晕到 14.5，其它尺寸按此比例放大。
pub fn halo_rgba(size: u32) -> RgbaImage {
    let mut img = RgbaImage::new(size, size);
    let center = (size as f32 - 1.0) * 0.5;
    let scale = center / 15.5;
    let inner = 11.0 * scale;
    let outer = 14.5 * scale;
    let halo = (outer - inner).max(0.001);
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let pixel = if dist < inner {
                Rgba([212, 160, 84, 255])
            } else if dist < outer {
                let a = ((outer - dist) / halo * 180.0) as u8;
                Rgba([232, 194, 122, a])
            } else {
                Rgba([0, 0, 0, 0])
            };
            img.put_pixel(x, y, pixel);
        }
    }
    img
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn halo_png(size: u32) -> Result<Vec<u8>, image::ImageError> {
    let img = halo_rgba(size);
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)?;
    Ok(buf)
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn halo_ico(size: u32) -> Result<Vec<u8>, image::ImageError> {
    let img = halo_rgba(size);
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Ico)?;
    Ok(buf)
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub const HALO_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
  <defs>
    <radialGradient id="halo" cx="50%" cy="50%" r="50%">
      <stop offset="0%" stop-color="#D4A054"/>
      <stop offset="68.75%" stop-color="#D4A054"/>
      <stop offset="68.75%" stop-color="#E8C27A" stop-opacity="0.706"/>
      <stop offset="90.625%" stop-color="#E8C27A" stop-opacity="0"/>
    </radialGradient>
  </defs>
  <circle cx="16" cy="16" r="14.5" fill="url(#halo)"/>
</svg>
"##;
