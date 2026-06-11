use anyhow::{anyhow, Result};
use std::io::BufWriter;
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
    GetDIBits, GetPixel, ReleaseDC, SelectObject, BI_RGB, BITMAPINFO, BITMAPINFOHEADER,
    CLR_INVALID, DIB_RGB_COLORS, HBITMAP, HGDIOBJ, RGBQUAD, SRCCOPY,
};
/// 获取屏幕指定坐标的像素颜色，返回 (R, G, B)
pub fn get_pixel_color(x: i32, y: i32) -> Result<(u8, u8, u8)> {
    unsafe {
        let hdc = GetDC(None);
        if hdc.is_invalid() {
            return Err(anyhow!("GetDC 失败"));
        }
        let color = GetPixel(hdc, x, y);
        ReleaseDC(None, hdc);
        if color.0 == CLR_INVALID {
            return Err(anyhow!("GetPixel 失败: ({}, {})", x, y));
        }
        Ok(colorref_to_rgb(color.0))
    }
}

fn colorref_to_rgb(c: u32) -> (u8, u8, u8) {
    ((c & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, ((c >> 16) & 0xFF) as u8)
}

/// 解析 "#RRGGBB" 格式颜色字符串
pub fn parse_color(hex: &str) -> Result<(u8, u8, u8)> {
    let s = hex.trim_start_matches('#');
    if s.len() != 6 {
        return Err(anyhow!("无效颜色格式: '{}'，应为 #RRGGBB", hex));
    }
    let r = u8::from_str_radix(&s[0..2], 16)?;
    let g = u8::from_str_radix(&s[2..4], 16)?;
    let b = u8::from_str_radix(&s[4..6], 16)?;
    Ok((r, g, b))
}

pub fn color_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

/// 截取屏幕区域，返回 BGRA 格式字节数组（用于 OCR）
pub fn capture_region(x: i32, y: i32, w: i32, h: i32) -> Result<Vec<u8>> {
    if w <= 0 || h <= 0 {
        return Err(anyhow!("无效区域尺寸: {}x{}", w, h));
    }
    unsafe {
        let hdc_screen = GetDC(None);
        if hdc_screen.is_invalid() {
            return Err(anyhow!("GetDC(screen) 失败"));
        }
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        if hdc_mem.is_invalid() {
            ReleaseDC(None, hdc_screen);
            return Err(anyhow!("CreateCompatibleDC 失败"));
        }
        let hbmp: HBITMAP = CreateCompatibleBitmap(hdc_screen, w, h);
        if hbmp.is_invalid() {
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(None, hdc_screen);
            return Err(anyhow!("CreateCompatibleBitmap 失败"));
        }
        let _ = SelectObject(hdc_mem, HGDIOBJ(hbmp.0));
        if let Err(e) = BitBlt(hdc_mem, 0, 0, w, h, Some(hdc_screen), x, y, SRCCOPY) {
            let _ = DeleteObject(HGDIOBJ(hbmp.0));
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(None, hdc_screen);
            return Err(anyhow!("BitBlt 失败: {}", e));
        }

        let bi_hdr = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // 负数 = 从上到下
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };
        let mut bmi = BITMAPINFO { bmiHeader: bi_hdr, bmiColors: [RGBQUAD::default()] };
        let mut buf = vec![0u8; (w * h * 4) as usize];
        GetDIBits(
            hdc_mem,
            hbmp,
            0,
            h as u32,
            Some(buf.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        let _ = DeleteObject(HGDIOBJ(hbmp.0));
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(None, hdc_screen);
        Ok(buf)
    }
}

/// 截取指定屏幕区域并以 PNG 格式保存到文件
pub fn capture_screenshot(path: &str, x: i32, y: i32, w: i32, h: i32) -> Result<()> {
    let bgra = capture_region(x, y, w, h)?;

    // Windows GDI 返回 BGRA，PNG 需要 RGBA
    let mut rgba = vec![0u8; bgra.len()];
    for i in (0..bgra.len()).step_by(4) {
        rgba[i] = bgra[i + 2]; // R
        rgba[i + 1] = bgra[i + 1]; // G
        rgba[i + 2] = bgra[i]; // B
        rgba[i + 3] = 255; // A（GDI 的 alpha 通道可能为 0，强制不透明）
    }

    let file = std::fs::File::create(path)
        .map_err(|e| anyhow!("无法创建截图文件 '{}': {}", path, e))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), w as u32, h as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| anyhow!("PNG 写入头失败: {}", e))?;
    writer
        .write_image_data(&rgba)
        .map_err(|e| anyhow!("PNG 写入数据失败: {}", e))?;

    Ok(())
}
