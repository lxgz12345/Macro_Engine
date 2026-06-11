use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, OnceLock,
    Mutex,
};
use std::time::{Duration, Instant};
use chrono::Local;
use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, PeekMessageW, SetWindowsHookExW, TranslateMessage,
        UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG, MSLLHOOKSTRUCT, PM_REMOVE,
        WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP,
        WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP,
        WM_SYSKEYDOWN, WM_SYSKEYUP,
    },
};

// ─── 录制事件 ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum RecordedEvent {
    MouseMove { x: i32, y: i32, elapsed_ms: u64 },
    MouseDown { btn: &'static str, elapsed_ms: u64 },
    MouseUp { btn: &'static str, elapsed_ms: u64 },
    KeyDown { vk: u32, elapsed_ms: u64 },
    KeyUp { vk: u32, elapsed_ms: u64 },
}

struct RecordingState {
    events: Vec<RecordedEvent>,
    start: Instant,
}

// 全局录制状态（hook 回调需要访问）
static RECORDING: OnceLock<Mutex<Option<RecordingState>>> = OnceLock::new();

fn recording() -> &'static Mutex<Option<RecordingState>> {
    RECORDING.get_or_init(|| Mutex::new(None))
}

fn elapsed_ms() -> u64 {
    recording()
        .lock()
        .unwrap()
        .as_ref()
        .map(|r| r.start.elapsed().as_millis() as u64)
        .unwrap_or(0)
}

fn push_event(evt: RecordedEvent) {
    if let Ok(mut lock) = recording().lock() {
        if let Some(ref mut state) = *lock {
            state.events.push(evt);
        }
    }
}

// ─── Hook 回调 ──────────────────────────────────────────────────────────────

unsafe extern "system" fn mouse_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let ms = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        let ms_val = elapsed_ms();
        let evt = match wparam.0 as u32 {
            WM_MOUSEMOVE => Some(RecordedEvent::MouseMove {
                x: ms.pt.x,
                y: ms.pt.y,
                elapsed_ms: ms_val,
            }),
            WM_LBUTTONDOWN => Some(RecordedEvent::MouseDown { btn: "left", elapsed_ms: ms_val }),
            WM_LBUTTONUP => Some(RecordedEvent::MouseUp { btn: "left", elapsed_ms: ms_val }),
            WM_RBUTTONDOWN => Some(RecordedEvent::MouseDown { btn: "right", elapsed_ms: ms_val }),
            WM_RBUTTONUP => Some(RecordedEvent::MouseUp { btn: "right", elapsed_ms: ms_val }),
            WM_MBUTTONDOWN => {
                Some(RecordedEvent::MouseDown { btn: "middle", elapsed_ms: ms_val })
            }
            WM_MBUTTONUP => Some(RecordedEvent::MouseUp { btn: "middle", elapsed_ms: ms_val }),
            _ => None,
        };
        if let Some(e) = evt {
            push_event(e);
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

unsafe extern "system" fn keyboard_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let kb = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let ms_val = elapsed_ms();
        let evt = match wparam.0 as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                Some(RecordedEvent::KeyDown { vk: kb.vkCode, elapsed_ms: ms_val })
            }
            WM_KEYUP | WM_SYSKEYUP => {
                Some(RecordedEvent::KeyUp { vk: kb.vkCode, elapsed_ms: ms_val })
            }
            _ => None,
        };
        if let Some(e) = evt {
            push_event(e);
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

// ─── 录制控制 ───────────────────────────────────────────────────────────────

pub struct RecordingHandle {
    pub running: Arc<AtomicBool>,
    pub thread: Option<std::thread::JoinHandle<Vec<RecordedEvent>>>,
}

/// 启动录制（在后台线程安装 Hook 并运行消息循环）
pub fn start_recording() -> RecordingHandle {
    // 初始化全局录制状态
    {
        let mut lock = recording().lock().unwrap();
        *lock = Some(RecordingState { events: Vec::new(), start: Instant::now() });
    }

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    let thread = std::thread::spawn(move || {
        let mouse_hook = unsafe {
            SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), None, 0)
                .unwrap_or(HHOOK::default())
        };
        let keyboard_hook = unsafe {
            SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0)
                .unwrap_or(HHOOK::default())
        };

        let mut msg = MSG::default();
        loop {
            if !running_clone.load(Ordering::Relaxed) {
                break;
            }
            // 使用 PeekMessageW（非阻塞）驱动消息循环，使低级 Hook 回调得以运行
            while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
                if msg.message == WM_QUIT {
                    running_clone.store(false, Ordering::Relaxed);
                    break;
                }
                unsafe {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        if !mouse_hook.is_invalid() {
            let _ = unsafe { UnhookWindowsHookEx(mouse_hook) };
        }
        if !keyboard_hook.is_invalid() {
            let _ = unsafe { UnhookWindowsHookEx(keyboard_hook) };
        }

        // 取回录制的事件
        recording().lock().unwrap().take().map(|s| s.events).unwrap_or_default()
    });

    RecordingHandle { running, thread: Some(thread) }
}

/// 停止录制并等待线程结束，返回录制的事件列表
pub fn stop_recording(handle: &mut RecordingHandle) -> Vec<RecordedEvent> {
    handle.running.store(false, Ordering::Relaxed);
    handle.thread.take().and_then(|t| t.join().ok()).unwrap_or_default()
}

/// 将录制事件转换为 Lua 脚本字符串
pub fn events_to_lua(events: &[RecordedEvent]) -> String {
    let now = Local::now().format("%Y%m%d_%H%M%S");
    let name = format!("录制_{}", now);
    let mut out = format!(
        "meta = {{\n\
         \x20   name = \"{name}\",\n\
         \x20   trigger_key = \"F9\",\n\
         \x20   trigger_mode = \"once\",\n\
         \x20   target_class = \"\",\n\
         }}\n\n\
         function on_loop()\n"
    );

    let mut last_ms = 0u64;
    for evt in events {
        match evt {
            RecordedEvent::MouseMove { x, y, elapsed_ms } => {
                let d = elapsed_ms.saturating_sub(last_ms);
                if d > 0 {
                    out.push_str(&format!("    delay({})\n", d));
                }
                out.push_str(&format!("    mouse_move({}, {}, true)\n", x, y));
                last_ms = *elapsed_ms;
            }
            RecordedEvent::MouseDown { btn, elapsed_ms } => {
                let d = elapsed_ms.saturating_sub(last_ms);
                if d > 0 {
                    out.push_str(&format!("    delay({})\n", d));
                }
                out.push_str(&format!("    mouse_down(\"{}\")\n", btn));
                last_ms = *elapsed_ms;
            }
            RecordedEvent::MouseUp { btn, elapsed_ms } => {
                let d = elapsed_ms.saturating_sub(last_ms);
                if d > 0 {
                    out.push_str(&format!("    delay({})\n", d));
                }
                out.push_str(&format!("    mouse_up(\"{}\")\n", btn));
                last_ms = *elapsed_ms;
            }
            RecordedEvent::KeyDown { vk, elapsed_ms } => {
                let d = elapsed_ms.saturating_sub(last_ms);
                if d > 0 {
                    out.push_str(&format!("    delay({})\n", d));
                }
                out.push_str(&format!("    key_down(\"VK_{:02X}\")\n", vk));
                last_ms = *elapsed_ms;
            }
            RecordedEvent::KeyUp { vk, elapsed_ms } => {
                let d = elapsed_ms.saturating_sub(last_ms);
                if d > 0 {
                    out.push_str(&format!("    delay({})\n", d));
                }
                out.push_str(&format!("    key_up(\"VK_{:02X}\")\n", vk));
                last_ms = *elapsed_ms;
            }
        }
    }
    out.push_str("end\n");
    out
}
