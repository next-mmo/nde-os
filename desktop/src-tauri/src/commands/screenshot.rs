use ai_launcher_core::screenshot::capture::{capture_screen, image_to_base64, CaptureArea};
use ai_launcher_core::screenshot::ocr::extract_text;
use serde::Serialize;
use tauri::command;

#[derive(Serialize)]
pub struct ScreenshotResponse {
    pub base64_image: Option<String>,
    pub text: Option<String>,
}

#[command]
pub async fn capture_screenshot(
    x: Option<i32>,
    y: Option<i32>,
    width: Option<u32>,
    height: Option<u32>,
    ocr: bool,
) -> Result<ScreenshotResponse, String> {
    let area = if let (Some(x), Some(y), Some(w), Some(h)) = (x, y, width, height) {
        Some(CaptureArea {
            x,
            y,
            width: w,
            height: h,
        })
    } else {
        None
    };

    // Run capture on a blocking OS thread — Windows GDI (BitBlt) can return
    // black images when called from a Tokio async worker thread.
    let image = tokio::task::spawn_blocking(move || capture_screen(area))
        .await
        .map_err(|e| format!("Capture task failed: {}", e))?
        .map_err(|e| format!("Failed to capture screen: {}", e))?;

    let mut response = ScreenshotResponse {
        base64_image: None,
        text: None,
    };

    if ocr {
        let text: String = extract_text(&image)
            .await
            .map_err(|e: anyhow::Error| format!("Failed to extract text: {}", e))?;
        response.text = Some(text);
    } else {
        let base64: String = image_to_base64(&image).map_err(|e: anyhow::Error| format!("Failed to encode image: {}", e))?;
        response.base64_image = Some(format!("data:image/png;base64,{}", base64));
    }

    Ok(response)
}
