use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use anyhow::Result;
use parking_lot::Mutex;

use crate::{
    input::{key_is_down, key_to_vk},
    lua_api,
    recorder::{self, RecordingHandle},
    types::{RunContext, ScriptMeta, ScriptSettings, TriggerMode},
};

// ─── 脚本条目 ────────────────────────────────────────────────────────────────

pub struct ScriptEntry {
    pub path: PathBuf,
    pub meta: ScriptMeta,
    pub settings: ScriptSettings,
}

// ─── 引擎 ────────────────────────────────────────────────────────────────────

pub struct Engine {
    scripts: Vec<ScriptEntry>,
    current: usize,
    scripts_dir: PathBuf,
    config_path: PathBuf,

    // 运行状态
    running: Arc<AtomicBool>,
    script_thread: Option<thread::JoinHandle<()>>,

    // 录制状态
    recording_handle: Option<RecordingHandle>,
}

impl Engine {
    /// 创建引擎，从 scripts_dir 目录加载所有 .lua 脚本
    pub fn new(scripts_dir: impl AsRef<Path>) -> Result<Self> {
        let scripts_dir = scripts_dir.as_ref().to_path_buf();
        let config_path = scripts_dir
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.join("last_script.txt"))
            .unwrap_or_else(|| PathBuf::from("last_script.txt"));

        let scripts = load_scripts(&scripts_dir);
        let current = load_last_script(&config_path, &scripts);

        Ok(Engine {
            scripts,
            current,
            scripts_dir,
            config_path,
            running: Arc::new(AtomicBool::new(false)),
            script_thread: None,
            recording_handle: None,
        })
    }

    /// 主循环：轮询热键，管理脚本执行
    #[allow(unreachable_code)]
    pub fn run(&mut self) -> Result<()> {
        self.print_status();
        //println!("  F8=录制  F10=重载脚本  F12=切换脚本  ESC=退出");
        println!("  F8=录制  F10=重载脚本  F12=切换脚本");
        println!("{}", "─".repeat(60));

        let mut trigger_prev = false;
        let mut f8_prev = false;
        let mut f10_prev = false;
        let mut f12_prev = false;
        // let mut esc_prev = false;

        loop {
            let trigger_down = self.trigger_is_down();
            let f8_down = key_is_down(key_to_vk("F8").unwrap());
            let f10_down = key_is_down(key_to_vk("F10").unwrap());
            let f12_down = key_is_down(key_to_vk("F12").unwrap());
            // let esc_down = key_is_down(key_to_vk("ESC").unwrap());

            // // ESC 退出
            // if esc_down && !esc_prev {
            //     self.stop_script();
            //     println!("已退出。");
            //     break;
            // }

            // F10 重载脚本列表
            if f10_down && !f10_prev {
                self.stop_script();
                self.scripts = load_scripts(&self.scripts_dir);
                self.current = load_last_script(&self.config_path, &self.scripts);
                println!("脚本已重载。");
                self.print_status();
            }

            // F12 切换脚本
            if f12_down && !f12_prev {
                self.stop_script();
                if !self.scripts.is_empty() {
                    self.current = (self.current + 1) % self.scripts.len();
                    save_last_script(&self.config_path, &self.scripts, self.current);
                }
                clear_console();
                self.print_status();
                println!("  F8=录制  F10=重载脚本  F12=切换脚本");
                println!("{}", "─".repeat(60));
            }

            // F8 开始/停止录制
            if f8_down && !f8_prev {
                if self.recording_handle.is_some() {
                    self.stop_recording_and_save();
                } else {
                    println!("[F8] 开始录制……（再按 F8 停止）");
                    self.recording_handle = Some(recorder::start_recording());
                }
            }

            // 触发键逻辑
            if let Some(entry) = self.scripts.get(self.current) {
                match entry.meta.trigger_mode {
                    TriggerMode::Hold => {
                        if trigger_down && !trigger_prev {
                            self.start_script();
                        } else if !trigger_down && trigger_prev {
                            self.stop_script();
                        }
                    }
                    TriggerMode::Toggle | TriggerMode::Once => {
                        if trigger_down && !trigger_prev {
                            if self.is_script_running() {
                                self.stop_script();
                            } else {
                                self.start_script();
                            }
                        }
                    }
                }
            }

            // 检测脚本线程是否自然结束
            if let Some(ref t) = self.script_thread {
                if t.is_finished() {
                    self.script_thread = None;
                    self.running.store(false, Ordering::Relaxed);
                    println!("[{}] 执行完毕。", self.current_name());
                }
            }

            trigger_prev = trigger_down;
            f8_prev = f8_down;
            f10_prev = f10_down;
            f12_prev = f12_down;
            // esc_prev = esc_down;

            thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }

    // ─── 脚本控制 ─────────────────────────────────────────────────────────

    fn start_script(&mut self) {
        if self.scripts.is_empty() {
            println!("无可用脚本。");
            return;
        }
        let entry = &self.scripts[self.current];
        println!("[{}] 开始执行……", entry.meta.name);

        let running = Arc::new(AtomicBool::new(true));
        self.running = running.clone();

        let ctx = RunContext {
            running,
            meta: entry.meta.clone(),
            settings: entry.settings.clone(),
            scripts_dir: self.scripts_dir.clone(),
            cooldowns: Arc::new(Mutex::new(HashMap::new())),
        };
        let path = entry.path.clone();

        self.script_thread = Some(thread::spawn(move || {
            lua_api::execute_script(ctx, path);
        }));
    }

    fn stop_script(&mut self) {
        if self.is_script_running() {
            self.running.store(false, Ordering::Relaxed);
            println!("[{}] 已停止。", self.current_name());
        }
        // 等待线程结束（最多 500ms）
        if let Some(t) = self.script_thread.take() {
            let _ = t.join();
        }
    }

    fn is_script_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
            && self.script_thread.as_ref().map_or(false, |t| !t.is_finished())
    }

    // ─── 录制控制 ─────────────────────────────────────────────────────────

    fn stop_recording_and_save(&mut self) {
        if let Some(mut handle) = self.recording_handle.take() {
            let events = recorder::stop_recording(&mut handle);
            let lua_code = recorder::events_to_lua(&events);
            let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let filename = format!("录制_{}.lua", ts);
            let path = self.scripts_dir.join(&filename);
            match std::fs::write(&path, &lua_code) {
                Ok(_) => {
                    println!("[F8] 录制完成，已保存 {} 个事件 → {}", events.len(), filename);
                    // 重载脚本列表
                    self.scripts = load_scripts(&self.scripts_dir);
                }
                Err(e) => eprintln!("[F8] 保存失败: {}", e),
            }
        }
    }

    // ─── 辅助 ─────────────────────────────────────────────────────────────

    fn trigger_is_down(&self) -> bool {
        let Some(entry) = self.scripts.get(self.current) else {
            return false;
        };
        match key_to_vk(&entry.meta.trigger_key) {
            Ok(vk) => key_is_down(vk),
            Err(_) => false,
        }
    }

    fn current_name(&self) -> String {
        self.scripts
            .get(self.current)
            .map(|e| e.meta.name.clone())
            .unwrap_or_else(|| "无脚本".into())
    }

    fn print_status(&self) {
        println!("\n{}", "─".repeat(60));
        if self.scripts.is_empty() {
            println!("  【无脚本】请将 .lua 文件放入 scripts/ 目录");
            return;
        }
        println!("  当前脚本列表：");
        for (i, e) in self.scripts.iter().enumerate() {
            let marker = if i == self.current { "▶" } else { " " };
            println!(
                "  {} [{}] {} (触发: {} / {})",
                marker,
                i + 1,
                e.meta.name,
                e.meta.trigger_key,
                trigger_mode_str(&e.meta.trigger_mode),
            );
        }
        println!("{}", "─".repeat(60));
    }
}

// ─── 工具函数 ─────────────────────────────────────────────────────────────────

fn load_scripts(dir: &Path) -> Vec<ScriptEntry> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut scripts = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("lua") {
            continue;
        }
        match lua_api::load_meta(&path) {
            Ok((meta, settings)) => scripts.push(ScriptEntry { path, meta, settings }),
            Err(e) => log::warn!("跳过 {:?}: {}", path.file_name().unwrap_or_default(), e),
        }
    }
    // 按文件名排序，使列表稳定
    scripts.sort_by(|a, b| a.path.cmp(&b.path));
    scripts
}

fn trigger_mode_str(m: &TriggerMode) -> &'static str {
    match m {
        TriggerMode::Hold => "按住",
        TriggerMode::Toggle => "切换",
        TriggerMode::Once => "单次",
    }
}

/// 清除控制台屏幕（ANSI 转义）
fn clear_console() {
    print!("\x1B[2J\x1B[H");
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

/// 从配置文件读取上次脚本名称，返回匹配的脚本索引，未找到则返回 0
fn load_last_script(config_path: &Path, scripts: &[ScriptEntry]) -> usize {
    let Ok(last) = std::fs::read_to_string(config_path) else {
        return 0;
    };
    let last = last.trim();
    scripts
        .iter()
        .position(|s| {
            s.path
                .file_name()
                .and_then(|n| n.to_str())
                .map_or(false, |n| n == last)
        })
        .unwrap_or(0)
}

/// 将当前选中的脚本文件名保存到配置文件
fn save_last_script(config_path: &Path, scripts: &[ScriptEntry], current: usize) {
    if let Some(name) = scripts
        .get(current)
        .and_then(|e| e.path.file_name())
        .and_then(|n| n.to_str())
    {
        let _ = std::fs::write(config_path, name);
    }
}
