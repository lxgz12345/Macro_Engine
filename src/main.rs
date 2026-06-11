mod engine;
mod input;
mod lua_api;
mod ocr;
mod recorder;
mod screen;
mod types;
mod window;

use std::path::PathBuf;

use anyhow::Result;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE,
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, DPI_AWARENESS_CONTEXT_SYSTEM_AWARE,
};

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    set_dpi_awareness();

    let scripts_dir = resolve_scripts_dir();
    if !scripts_dir.exists() {
        std::fs::create_dir_all(&scripts_dir)?;
        println!("已创建脚本目录: {}", scripts_dir.display());
    }

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║          MacroEngine (Rust) — Lua 脚本引擎              ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!("脚本目录: {}", scripts_dir.display());
    if ocr::is_available() {
        println!("OCR 功能: ✓ 可用");
    } else {
        println!("OCR 功能: ✗ 不可用（需安装 Windows 语言包）");
    }

    let mut engine = engine::Engine::new(&scripts_dir)?;
    engine.run()
}

fn set_dpi_awareness() {
    let contexts = [
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE,
        DPI_AWARENESS_CONTEXT_SYSTEM_AWARE,
    ];
    for ctx in contexts {
        if unsafe { SetProcessDpiAwarenessContext(ctx) }.is_ok() {
            return;
        }
    }
}

fn resolve_scripts_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("scripts")))
        .unwrap_or_else(|| PathBuf::from("scripts"))
}
