use anyhow::{anyhow, Result};
use windows::{
    Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap},
    Media::Ocr::OcrEngine,
    Storage::Streams::DataWriter,
};

use crate::screen;

/// 识别屏幕指定区域中的文字（使用 Windows 内置 OCR）
pub fn recognize_region(x: i32, y: i32, w: i32, h: i32) -> Result<String> {
    let bgra = screen::capture_region(x, y, w, h)?;
    recognize_bgra(&bgra, w, h)
}

fn recognize_bgra(bgra: &[u8], w: i32, h: i32) -> Result<String> {
    // 将原始 BGRA 字节写入 DataWriter → 获取 IBuffer
    let writer = DataWriter::new().map_err(|e| anyhow!("DataWriter::new 失败: {}", e))?;
    writer
        .WriteBytes(bgra)
        .map_err(|e| anyhow!("WriteBytes 失败: {}", e))?;
    let buffer = writer
        .DetachBuffer()
        .map_err(|e| anyhow!("DetachBuffer 失败: {}", e))?;

    // 从 BGRA 字节创建 SoftwareBitmap
    let bitmap = SoftwareBitmap::CreateCopyFromBuffer(&buffer, BitmapPixelFormat::Bgra8, w, h)
        .map_err(|e| anyhow!("CreateCopyFromBuffer 失败: {}", e))?;

    // 使用系统语言包创建 OCR 引擎
    let engine = OcrEngine::TryCreateFromUserProfileLanguages()
        .map_err(|e| anyhow!("创建 OCR 引擎失败: {}", e))?;

    // 异步识别（阻塞等待）
    let result = engine
        .RecognizeAsync(&bitmap)
        .map_err(|e| anyhow!("RecognizeAsync 失败: {}", e))?
        .join()
        .map_err(|e| anyhow!("OCR 等待结果失败: {}", e))?;

    let text = result
        .Text()
        .map_err(|e| anyhow!("获取 OCR 文字失败: {}", e))?;

    Ok(text.to_string())
}

/// 检测 OCR 功能是否可用
pub fn is_available() -> bool {
    OcrEngine::TryCreateFromUserProfileLanguages().is_ok()
}
