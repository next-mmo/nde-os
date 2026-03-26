use crate::llm::ToolDef;
use crate::sandbox::Sandbox;
use crate::tools::Tool;
use anyhow::{Context, Result};
use async_trait::async_trait;

#[cfg(feature = "screenshot")]
pub struct ScreenshotTool;

#[cfg(feature = "screenshot")]
#[async_trait]
impl Tool for ScreenshotTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "nde_screenshot".into(),
            description: "Capture a screenshot of the user's desktop, optionally cropped to a specific area, and optionally extract text using OCR.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "ocr": {
                        "type": "boolean",
                        "description": "If true, extracts text from the screenshot using optical character recognition (OCR) and returns it instead of the image."
                    },
                    "x": {
                        "type": "integer",
                        "description": "Optional X coordinate for capturing a specific region."
                    },
                    "y": {
                        "type": "integer",
                        "description": "Optional Y coordinate for capturing a specific region."
                    },
                    "width": {
                        "type": "integer",
                        "description": "Width of the region. Required if x/y are provided."
                    },
                    "height": {
                        "type": "integer",
                        "description": "Height of the region. Required if x/y are provided."
                    }
                }
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let x = args.get("x").and_then(|v| v.as_i64()).map(|v| v as i32);
        let y = args.get("y").and_then(|v| v.as_i64()).map(|v| v as i32);
        let width = args.get("width").and_then(|v| v.as_u64()).map(|v| v as u32);
        let height = args.get("height").and_then(|v| v.as_u64()).map(|v| v as u32);
        let do_ocr = args.get("ocr").and_then(|v| v.as_bool()).unwrap_or(false);

        let area = if let (Some(x), Some(y), Some(w), Some(h)) = (x, y, width, height) {
            Some(crate::screenshot::capture::CaptureArea { x, y, width: w, height: h })
        } else {
            None
        };

        let image = crate::screenshot::capture::capture_screen(area)?;

        if do_ocr {
            let text = crate::screenshot::ocr::extract_text(&image).await?;
            if text.trim().is_empty() {
                Ok("No text detected in screenshot.".into())
            } else {
                Ok(text)
            }
        } else {
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let filename = format!("screenshot_{}.png", timestamp);
            
            // Save to sandbox root
            let save_path = sandbox.resolve(std::path::Path::new(&filename))?;
            
            // It's possible we want to just save it and return the base64 anyway, 
            // but saving it locally is safer. Let's return the path.
            image.save(&save_path).context("Failed to save screenshot image")?;
            
            Ok(format!("Screenshot captured successfully and saved to {}", filename))
        }
    }
}
