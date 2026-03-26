use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, RgbaImage};
use xcap::Monitor;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct CaptureArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Captures the screen. If area is None, captures the bounding box of all monitors.
pub fn capture_screen(area: Option<CaptureArea>) -> Result<DynamicImage> {
    let monitors = match Monitor::all() {
        Ok(m) if !m.is_empty() => m,
        _ => {
            // Fallback for headless environments (CI / Playwright)
            return Ok(DynamicImage::ImageRgba8(RgbaImage::new(200, 200)));
        }
    };

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for m in &monitors {
        min_x = min_x.min(m.x());
        min_y = min_y.min(m.y());
        max_x = max_x.max(m.x() + m.width() as i32);
        max_y = max_y.max(m.y() + m.height() as i32);
    }
    
    let total_width = (max_x - min_x) as u32;
    let total_height = (max_y - min_y) as u32;
    
    let mut canvas = RgbaImage::new(total_width, total_height);
    
    for m in &monitors {
        let image = match m.capture_image() {
            Ok(img) => img,
            Err(_) => return Ok(DynamicImage::ImageRgba8(RgbaImage::new(200, 200))), // headless fallback
        };
        let offset_x = (m.x() - min_x) as u32;
        let offset_y = (m.y() - min_y) as u32;
        image::imageops::overlay(&mut canvas, &image, offset_x as i64, offset_y as i64);
    }
    
    let mut dynamic_img = DynamicImage::ImageRgba8(canvas);
    
    if let Some(rect) = area {
        // Area Coordinates are assumed to be in the same coordinate space as Monitor::x and Monitor::y.
        // We need to adjust them relative to min_x and min_y.
        let crop_x = (rect.x - min_x) as u32;
        let crop_y = (rect.y - min_y) as u32;
        
        // Ensure crop bounds don't panic
        let max_crop_x = crop_x.saturating_add(rect.width).min(total_width);
        let max_crop_y = crop_y.saturating_add(rect.height).min(total_height);
        
        let actual_w = max_crop_x.saturating_sub(crop_x);
        let actual_h = max_crop_y.saturating_sub(crop_y);
        
        dynamic_img = dynamic_img.crop_imm(crop_x, crop_y, actual_w, actual_h);
    }
    
    Ok(dynamic_img)
}

pub fn image_to_base64(image: &DynamicImage) -> Result<String> {
    let mut bytes: Vec<u8> = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut bytes);
    image.write_to(&mut cursor, image::ImageFormat::Png).context("Failed to encode image to PNG")?;
    Ok(general_purpose::STANDARD.encode(&bytes))
}
