use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
    time::Instant,
};
use parking_lot::Mutex;

#[derive(Debug, Clone, PartialEq)]
pub enum TriggerMode {
    Hold,   // 按住时执行，松开停止
    Toggle, // 按一次开始，再按停止
    Once,   // 按一次执行一遍后自动停止
}

impl Default for TriggerMode {
    fn default() -> Self { TriggerMode::Toggle }
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ScriptMeta {
    pub name: String,
    pub description: String,
    pub trigger_key: String,
    pub trigger_mode: TriggerMode,
    pub target_class: String,
}

#[derive(Debug, Clone)]
pub struct ScriptSettings {
    pub default_click_hold: u32,
    pub default_post_delay: u32,
    pub global_speed: f64,
}

impl Default for ScriptSettings {
    fn default() -> Self {
        Self {
            default_click_hold: 30,
            default_post_delay: 15,
            global_speed: 1.0,
        }
    }
}

/// 脚本执行上下文，可在线程间共享
#[derive(Clone)]
pub struct RunContext {
    pub running: Arc<AtomicBool>,
    pub meta: ScriptMeta,
    pub settings: ScriptSettings,
    pub scripts_dir: PathBuf,
    pub cooldowns: Arc<Mutex<HashMap<String, Instant>>>,
}
