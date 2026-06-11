use anyhow::{anyhow, Result};
use std::{thread, time::Duration};
use windows::Win32::{
    Foundation::POINT,
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
            KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, MOUSE_EVENT_FLAGS, MOUSEEVENTF_LEFTDOWN,
            MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
            MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, VIRTUAL_KEY, VK_BACK,
            VK_CAPITAL, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F10,
            VK_F11, VK_F12, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_HOME,
            VK_INSERT, VK_LBUTTON, VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_MBUTTON,
            VK_MENU, VK_NEXT, VK_NUMPAD0, VK_NUMPAD1, VK_NUMPAD2, VK_NUMPAD3, VK_NUMPAD4,
            VK_NUMPAD5, VK_NUMPAD6, VK_NUMPAD7, VK_NUMPAD8, VK_NUMPAD9, VK_PAUSE, VK_PRIOR,
            VK_RBUTTON, VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_SHIFT,
            VK_SPACE, VK_TAB, VK_UP, VK_XBUTTON1, VK_XBUTTON2,
        },
        WindowsAndMessaging::{GetCursorPos, SetCursorPos},
    },
};

use crate::window;

// ─── 鼠标操作 ───────────────────────────────────────────────────────────────

pub fn mouse_move(x: i32, y: i32, abs: bool, target_class: &str) -> Result<()> {
    let (sx, sy) = if abs {
        window::client_to_screen(target_class, x, y)
    } else {
        let mut cur = POINT { x: 0, y: 0 };
        let _ = unsafe { GetCursorPos(&mut cur) };
        (cur.x + x, cur.y + y)
    };
    unsafe { SetCursorPos(sx, sy) }.map_err(|e| anyhow!("SetCursorPos({}, {}): {}", sx, sy, e))
}

pub fn mouse_click(btn: &str, hold_ms: u32) -> Result<()> {
    let (down, up) = btn_flags(btn)?;
    send_mouse_event(down)?;
    thread::sleep(Duration::from_millis(hold_ms as u64));
    send_mouse_event(up)?;
    Ok(())
}

pub fn mouse_down(btn: &str) -> Result<()> {
    send_mouse_event(btn_flags(btn)?.0)
}

pub fn mouse_up(btn: &str) -> Result<()> {
    send_mouse_event(btn_flags(btn)?.1)
}

fn btn_flags(btn: &str) -> Result<(MOUSE_EVENT_FLAGS, MOUSE_EVENT_FLAGS)> {
    Ok(match btn.to_ascii_lowercase().as_str() {
        "left" | "lbutton" => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        "right" | "rbutton" => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
        "middle" | "mbutton" => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
        _ => return Err(anyhow!("未知鼠标按键: '{}'", btn)),
    })
}

fn send_mouse_event(flags: MOUSE_EVENT_FLAGS) -> Result<()> {
    let inp = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let sent = unsafe { SendInput(&[inp], std::mem::size_of::<INPUT>() as i32) };
    if sent == 0 {
        Err(anyhow!("SendInput 鼠标事件失败"))
    } else {
        Ok(())
    }
}

// ─── 键盘操作 ───────────────────────────────────────────────────────────────

pub fn key_click(key: &str, hold_ms: u32) -> Result<()> {
    key_down(key)?;
    thread::sleep(Duration::from_millis(hold_ms as u64));
    key_up(key)?;
    Ok(())
}

pub fn key_down(key: &str) -> Result<()> {
    send_key_event(key_to_vk(key)?, false)
}

pub fn key_up(key: &str) -> Result<()> {
    send_key_event(key_to_vk(key)?, true)
}

fn send_key_event(vk: VIRTUAL_KEY, is_up: bool) -> Result<()> {
    let flags = if is_up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) };
    let inp = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let sent = unsafe { SendInput(&[inp], std::mem::size_of::<INPUT>() as i32) };
    if sent == 0 {
        Err(anyhow!("SendInput 键盘事件失败"))
    } else {
        Ok(())
    }
}

/// 检测某虚拟键是否当前被按下
pub fn key_is_down(vk: VIRTUAL_KEY) -> bool {
    unsafe { (GetAsyncKeyState(vk.0 as i32) as u16 & 0x8000) != 0 }
}

/// 将按键名称字符串转换为 Windows 虚拟键码
pub fn key_to_vk(name: &str) -> Result<VIRTUAL_KEY> {
    let up = name.to_ascii_uppercase();
    let vk = match up.as_str() {
        "LBUTTON" => VK_LBUTTON,
        "RBUTTON" => VK_RBUTTON,
        "MBUTTON" => VK_MBUTTON,
        "XBUTTON1" => VK_XBUTTON1,
        "XBUTTON2" => VK_XBUTTON2,
        "BACK" | "BACKSPACE" => VK_BACK,
        "TAB" => VK_TAB,
        "ENTER" | "RETURN" => VK_RETURN,
        "SHIFT" => VK_SHIFT,
        "CTRL" | "CONTROL" => VK_CONTROL,
        "ALT" | "MENU" => VK_MENU,
        "PAUSE" => VK_PAUSE,
        "CAPITAL" | "CAPS" | "CAPSLOCK" => VK_CAPITAL,
        "ESC" | "ESCAPE" => VK_ESCAPE,
        "SPACE" => VK_SPACE,
        "PGUP" | "PRIOR" => VK_PRIOR,
        "PGDN" | "NEXT" => VK_NEXT,
        "END" => VK_END,
        "HOME" => VK_HOME,
        "LEFT" => VK_LEFT,
        "UP" => VK_UP,
        "RIGHT" => VK_RIGHT,
        "DOWN" => VK_DOWN,
        "INS" | "INSERT" => VK_INSERT,
        "DEL" | "DELETE" => VK_DELETE,
        "F1" => VK_F1,
        "F2" => VK_F2,
        "F3" => VK_F3,
        "F4" => VK_F4,
        "F5" => VK_F5,
        "F6" => VK_F6,
        "F7" => VK_F7,
        "F8" => VK_F8,
        "F9" => VK_F9,
        "F10" => VK_F10,
        "F11" => VK_F11,
        "F12" => VK_F12,
        "LSHIFT" => VK_LSHIFT,
        "RSHIFT" => VK_RSHIFT,
        "LCTRL" | "LCONTROL" => VK_LCONTROL,
        "RCTRL" | "RCONTROL" => VK_RCONTROL,
        "LALT" | "LMENU" => VK_LMENU,
        "RALT" | "RMENU" => VK_RMENU,
        "NUMPAD0" => VK_NUMPAD0,
        "NUMPAD1" => VK_NUMPAD1,
        "NUMPAD2" => VK_NUMPAD2,
        "NUMPAD3" => VK_NUMPAD3,
        "NUMPAD4" => VK_NUMPAD4,
        "NUMPAD5" => VK_NUMPAD5,
        "NUMPAD6" => VK_NUMPAD6,
        "NUMPAD7" => VK_NUMPAD7,
        "NUMPAD8" => VK_NUMPAD8,
        "NUMPAD9" => VK_NUMPAD9,
        k if k.len() == 1 => {
            let c = k.chars().next().unwrap();
            if c.is_ascii_alphanumeric() {
                VIRTUAL_KEY(c.to_ascii_uppercase() as u16)
            } else {
                return Err(anyhow!("未知按键: '{}'", name));
            }
        }
        _ => return Err(anyhow!("未知按键: '{}'", name)),
    };
    Ok(vk)
}
