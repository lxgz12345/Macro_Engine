use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HWND, POINT},
        Graphics::Gdi::ClientToScreen,
        UI::WindowsAndMessaging::{
            FindWindowW, GetClassNameW, GetForegroundWindow, GetSystemMetrics, SM_CXSCREEN,
            SM_CYSCREEN,
        },
    },
};

/// 获取当前前台窗口的类名
pub fn get_foreground_class() -> Option<String> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return None;
    }
    get_class_name(hwnd)
}

/// 获取指定窗口句柄的类名
pub fn get_class_name(hwnd: HWND) -> Option<String> {
    let mut buf = [0u16; 256];
    let len = unsafe { GetClassNameW(hwnd, &mut buf) };
    if len == 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buf[..len as usize]))
}

/// 按类名查找窗口
pub fn find_window_by_class(class_name: &str) -> Option<HWND> {
    let wide: Vec<u16> = class_name.encode_utf16().chain([0]).collect();
    // FindWindowW returns Result<HWND> in windows 0.62
    unsafe { FindWindowW(PCWSTR(wide.as_ptr()), PCWSTR::null()) }.ok()
}

/// 将窗口客户区坐标转换为屏幕坐标。
/// 若 target_class 为空或窗口未找到，原样返回。
pub fn client_to_screen(target_class: &str, x: i32, y: i32) -> (i32, i32) {
    if target_class.is_empty() {
        return (x, y);
    }
    let hwnd = match find_window_by_class(target_class) {
        Some(h) => h,
        None => return (x, y),
    };
    let mut pt = POINT { x: 0, y: 0 };
    let _ = unsafe { ClientToScreen(hwnd, &mut pt) };
    (x + pt.x, y + pt.y)
}

/// 获取主屏幕分辨率
#[allow(dead_code)]
pub fn get_screen_size() -> (i32, i32) {
    let w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    (w, h)
}

/// 判断目标窗口是否在前台
pub fn is_target_focused(target_class: &str) -> bool {
    if target_class.is_empty() {
        return true;
    }
    get_foreground_class().map_or(false, |c| c == target_class)
}
