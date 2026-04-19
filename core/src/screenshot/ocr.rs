use anyhow::{Context, Result};
use image::DynamicImage;

#[cfg(windows)]
pub async fn extract_text(image: &DynamicImage) -> Result<String> {
    use windows::core::HSTRING;
    use windows::Globalization::Language;
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};

    let rgba = image.to_rgba8();
    let width = rgba.width() as i32;
    let height = rgba.height() as i32;
    let bytes = rgba.into_raw();

    let stream = InMemoryRandomAccessStream::new().context("Failed to create stream")?;
    let writer = DataWriter::CreateDataWriter(&stream).context("Failed to create writer")?;
    writer.WriteBytes(&bytes).context("Failed to write bytes")?;
    writer
        .StoreAsync()
        .context("Failed to store stream")?
        .get()
        .context("Failed to await store")?;
    let buffer = writer.DetachBuffer().context("Failed to detach buffer")?;

    let bitmap =
        SoftwareBitmap::CreateCopyFromBuffer(&buffer, BitmapPixelFormat::Rgba8, width, height)
            .context("Failed to create SoftwareBitmap")?;

    // Create English OCR Engine for now (you could iterate installed languages)
    let lang =
        Language::CreateLanguage(&HSTRING::from("en-US")).context("Failed to create lang")?;
    let engine = OcrEngine::TryCreateFromLanguage(&lang).context("Failed to create OcrEngine")?;

    let result = engine
        .RecognizeAsync(&bitmap)
        .context("Failed to start recognize")?
        .get()
        .context("Failed to await recognize")?;
    let text = result.Text().context("Failed to get text")?;

    Ok(text.to_string())
}

#[cfg(not(windows))]
pub async fn extract_text(_image: &DynamicImage) -> Result<String> {
    Ok("OCR is only supported on Windows in this implementation.".to_string())
}
