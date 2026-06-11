use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use mlua::prelude::*;
use rand::Rng;

use crate::{
    input, ocr, screen,
    types::{RunContext, ScriptMeta, ScriptSettings, TriggerMode},
    window,
};

// ─── 脚本 Meta 解析 ─────────────────────────────────────────────────────────

/// 解析 Lua 脚本中的 meta 和 settings 表，返回元数据
pub fn load_meta(script_path: &Path) -> anyhow::Result<(ScriptMeta, ScriptSettings)> {
    let lua = Lua::new();
    let code = std::fs::read_to_string(script_path)?;
    load_lua_and_exec(&lua, &code)?;

    let meta = parse_meta(&lua);
    let settings = parse_settings(&lua);
    Ok((meta, settings))
}

fn load_lua_and_exec(lua: &Lua, code: &str) -> anyhow::Result<()> {
    lua.load(code).exec().map_err(|e| anyhow::anyhow!("{}", e))
}

fn parse_meta(lua: &Lua) -> ScriptMeta {
    let globals = lua.globals();
    let Ok(tbl) = globals.get::<LuaTable>("meta") else {
        return ScriptMeta::default();
    };
    ScriptMeta {
        name: tbl.get("name").unwrap_or_default(),
        description: tbl.get("description").unwrap_or_default(),
        trigger_key: tbl.get::<String>("trigger_key").unwrap_or_else(|_| "F9".into()),
        trigger_mode: match tbl.get::<String>("trigger_mode").as_deref() {
            Ok("hold") => TriggerMode::Hold,
            Ok("once") => TriggerMode::Once,
            _ => TriggerMode::Toggle,
        },
        target_class: tbl.get("target_class").unwrap_or_default(),
    }
}

fn parse_settings(lua: &Lua) -> ScriptSettings {
    let globals = lua.globals();
    let Ok(tbl) = globals.get::<LuaTable>("settings") else {
        return ScriptSettings::default();
    };
    let mut s = ScriptSettings::default();
    s.default_click_hold = tbl.get("default_click_hold").unwrap_or(s.default_click_hold);
    s.default_post_delay = tbl.get("default_post_delay").unwrap_or(s.default_post_delay);
    s.global_speed = tbl.get("global_speed").unwrap_or(s.global_speed);
    s
}

// ─── 脚本执行 ───────────────────────────────────────────────────────────────

/// 在当前线程执行脚本（调用方应在专用线程上调用此函数）
pub fn execute_script(ctx: RunContext, script_path: std::path::PathBuf) {
    // 初始化 COM（OCR 需要）
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_MULTITHREADED,
        );
    }

    let result = run_script_inner(&ctx, &script_path);

    if let Err(e) = result {
        let msg = e.to_string();
        if !msg.contains("interrupted") {
            log::error!("[{}] 脚本错误: {}", ctx.meta.name, msg);
        }
    }

    ctx.running.store(false, Ordering::Relaxed);

    unsafe { windows::Win32::System::Com::CoUninitialize() };
}

fn run_script_inner(ctx: &RunContext, script_path: &Path) -> LuaResult<()> {
    let lua = Lua::new();
    register_api(&lua, ctx)?;

    let code = std::fs::read_to_string(script_path)
        .map_err(|e| LuaError::RuntimeError(e.to_string()))?;
    lua.load(&code).set_name(script_path.to_string_lossy()).exec()?;

    let globals = lua.globals();

    // 调用 on_start（可选）
    if let Ok(f) = globals.get::<LuaFunction>("on_start") {
        f.call::<()>(())?;
    }

    // 主循环
    let on_loop: LuaFunction = globals
        .get("on_loop")
        .map_err(|_| LuaError::RuntimeError("脚本中未找到 on_loop 函数".into()))?;

    loop {
        if !ctx.running.load(Ordering::Relaxed) {
            break;
        }
        // 若设置了目标窗口，等待其获得焦点
        if !ctx.meta.target_class.is_empty()
            && !window::is_target_focused(&ctx.meta.target_class)
        {
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }

        match on_loop.call::<()>(()) {
            Ok(_) => {}
            Err(LuaError::RuntimeError(ref msg)) if msg.contains("interrupted") => break,
            Err(e) => return Err(e),
        }

        if ctx.meta.trigger_mode == TriggerMode::Once {
            break;
        }
    }

    // 调用 on_release（可选）
    if let Ok(f) = globals.get::<LuaFunction>("on_release") {
        let _ = f.call::<()>(());
    }

    Ok(())
}

// ─── Lua API 注册 ────────────────────────────────────────────────────────────

fn register_api(lua: &Lua, ctx: &RunContext) -> LuaResult<()> {
    let g = lua.globals();

    // ── mouse_move(x, y, abs=true, delay_ms=nil) ──────────────────────────
    {
        let running = ctx.running.clone();
        let tc = ctx.meta.target_class.clone();
        let post = post_delay_ms(ctx);
        g.set(
            "mouse_move",
            lua.create_function(move |_, (x, y, abs, d): (i32, i32, Option<bool>, Option<u64>)| {
                input::mouse_move(x, y, abs.unwrap_or(true), &tc)
                    .map_err(LuaError::external)?;
                sleep_interruptible(d.unwrap_or(post), &running)?;
                Ok(())
            })?,
        )?;
    }

    // ── mouse_click(btn, hold_ms=30, delay_ms=nil) ────────────────────────
    {
        let running = ctx.running.clone();
        let hold_def = ctx.settings.default_click_hold;
        let post = post_delay_ms(ctx);
        g.set(
            "mouse_click",
            lua.create_function(
                move |_, (btn, hold, d): (String, Option<u32>, Option<u64>)| {
                    input::mouse_click(&btn, hold.unwrap_or(hold_def))
                        .map_err(LuaError::external)?;
                    sleep_interruptible(d.unwrap_or(post), &running)?;
                    Ok(())
                },
            )?,
        )?;
    }

    // ── mouse_down(btn) / mouse_up(btn) ───────────────────────────────────
    g.set(
        "mouse_down",
        lua.create_function(|_, btn: String| {
            input::mouse_down(&btn).map_err(LuaError::external)
        })?,
    )?;
    g.set(
        "mouse_up",
        lua.create_function(|_, btn: String| {
            input::mouse_up(&btn).map_err(LuaError::external)
        })?,
    )?;

    // ── key_click(key, hold_ms=30, delay_ms=nil) ──────────────────────────
    {
        let running = ctx.running.clone();
        let hold_def = ctx.settings.default_click_hold;
        let post = post_delay_ms(ctx);
        g.set(
            "key_click",
            lua.create_function(
                move |_, (key, hold, d): (String, Option<u32>, Option<u64>)| {
                    input::key_click(&key, hold.unwrap_or(hold_def))
                        .map_err(LuaError::external)?;
                    sleep_interruptible(d.unwrap_or(post), &running)?;
                    Ok(())
                },
            )?,
        )?;
    }

    // ── key_down(key) / key_up(key) ───────────────────────────────────────
    g.set(
        "key_down",
        lua.create_function(|_, key: String| {
            input::key_down(&key).map_err(LuaError::external)
        })?,
    )?;
    g.set(
        "key_up",
        lua.create_function(|_, key: String| {
            input::key_up(&key).map_err(LuaError::external)
        })?,
    )?;

    // ── delay(ms) ─────────────────────────────────────────────────────────
    {
        let running = ctx.running.clone();
        g.set(
            "delay",
            lua.create_function(move |_, ms: u64| sleep_interruptible(ms, &running))?,
        )?;
    }

    // ── delay_rand(min_ms, max_ms) ────────────────────────────────────────
    {
        let running = ctx.running.clone();
        g.set(
            "delay_rand",
            lua.create_function(move |_, (min_ms, max_ms): (u64, u64)| {
                let ms = if min_ms <= max_ms {
                    rand::thread_rng().gen_range(min_ms..=max_ms)
                } else {
                    min_ms
                };
                sleep_interruptible(ms, &running)
            })?,
        )?;
    }

    // ── mouse_click_rand(x1,y1,x2,y2 [,btn [,hold_ms [,delay_ms]]]) ─────
    {
        let running = ctx.running.clone();
        let tc = ctx.meta.target_class.clone();
        let hold_def = ctx.settings.default_click_hold;
        let post = post_delay_ms(ctx);
        g.set(
            "mouse_click_rand",
            lua.create_function(
                move |_,
                      (x1, y1, x2, y2, btn, hold, d): (
                    i32,
                    i32,
                    i32,
                    i32,
                    Option<String>,
                    Option<u32>,
                    Option<u64>,
                )| {
                    let mut rng = rand::thread_rng();
                    let x = rng.gen_range(x1.min(x2)..=x1.max(x2));
                    let y = rng.gen_range(y1.min(y2)..=y1.max(y2));
                    input::mouse_move(x, y, true, &tc).map_err(LuaError::external)?;
                    input::mouse_click(
                        &btn.unwrap_or_else(|| "left".into()),
                        hold.unwrap_or(hold_def),
                    )
                    .map_err(LuaError::external)?;
                    sleep_interruptible(d.unwrap_or(post), &running)?;
                    Ok(())
                },
            )?,
        )?;
    }

    // ── webhook(key, content [, mention]) ─────────────────────────────────
    g.set(
        "webhook",
        lua.create_function(|_, (key, content, mention): (String, String, Option<String>)| {
            let url = format!(
                "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key={}",
                key
            );
            let mention_target = mention.unwrap_or_else(|| "@all".into());
            let body = serde_json::json!({
                "msgtype": "text",
                "text": {
                    "content": content,
                    "mentioned_mobile_list": [mention_target]
                }
            });
            ureq::post(&url)
                .send_json(body)
                .map_err(|e| LuaError::RuntimeError(format!("webhook 请求失败: {}", e)))?;
            Ok(())
        })?,
    )?;

    // ── cooldown(tag, ms) ─────────────────────────────────────────────────
    {
        let cd = ctx.cooldowns.clone();
        g.set(
            "cooldown",
            lua.create_function(move |_, (tag, ms): (String, u64)| {
                let now = Instant::now();
                let mut map = cd.lock();
                if let Some(&last) = map.get(&tag) {
                    if now.duration_since(last) < Duration::from_millis(ms) {
                        return Ok(false); // 冷却中，跳过
                    }
                }
                map.insert(tag, now);
                Ok(true)
            })?,
        )?;
    }

    // ── get_color(x, y) → "#RRGGBB" ──────────────────────────────────────
    {
        let tc = ctx.meta.target_class.clone();
        g.set(
            "get_color",
            lua.create_function(move |_, (x, y): (i32, i32)| {
                let (sx, sy) = window::client_to_screen(&tc, x, y);
                let (r, g, b) =
                    screen::get_pixel_color(sx, sy).map_err(LuaError::external)?;
                Ok(screen::color_to_hex(r, g, b))
            })?,
        )?;
    }

    // ── wait_color(x, y, color, timeout_ms) → true / error ───────────────
    {
        let running = ctx.running.clone();
        let tc = ctx.meta.target_class.clone();
        g.set(
            "wait_color",
            lua.create_function(
                move |_, (x, y, color, timeout): (i32, i32, String, Option<u64>)| {
                    wait_for_color(&tc, x, y, &color, timeout.unwrap_or(5000), true, &running)
                },
            )?,
        )?;
    }

    // ── if_color(x, y, color, timeout_ms) → bool ─────────────────────────
    {
        let running = ctx.running.clone();
        let tc = ctx.meta.target_class.clone();
        g.set(
            "if_color",
            lua.create_function(
                move |_, (x, y, color, timeout): (i32, i32, String, Option<u64>)| {
                    wait_for_color(&tc, x, y, &color, timeout.unwrap_or(50), false, &running)
                },
            )?,
        )?;
    }

    // ── ocr_region(x1, y1, x2, y2) → string ─────────────────────────────────
    {
        let tc = ctx.meta.target_class.clone();
        g.set(
            "ocr_region",
            lua.create_function(move |_, (x1, y1, x2, y2): (i32, i32, i32, i32)| {
                let (sx, sy) = window::client_to_screen(&tc, x1, y1);
                let w = (x2 - x1).max(1);
                let h = (y2 - y1).max(1);
                ocr::recognize_region(sx, sy, w, h).map_err(LuaError::external)
            })?,
        )?;
    }

    // ── import(file) ──────────────────────────────────────────────────────
    {
        let scripts_dir = ctx.scripts_dir.clone();
        g.set(
            "import",
            lua.create_function(move |lua, file: String| {
                let path = scripts_dir.join(format!("{}.lua", file));
                let code = std::fs::read_to_string(&path).map_err(|e| {
                    LuaError::RuntimeError(format!("import '{}' 失败: {}", file, e))
                })?;
                lua.load(&code).set_name(&file).exec()
            })?,
        )?;
    }

    // ── log(msg) ──────────────────────────────────────────────────────────
    g.set(
        "log",
        lua.create_function(|_, msg: String| {
            println!("[LUA] {}", msg);
            Ok(())
        })?,
    )?;

    // ── stop() ────────────────────────────────────────────────────────────
    {
        let running = ctx.running.clone();
        g.set(
            "stop",
            lua.create_function(move |_, ()| {
                running.store(false, Ordering::Relaxed);
                Err::<(), LuaError>(LuaError::RuntimeError("interrupted".into()))
            })?,
        )?;
    }

    // ── exec(program [, args [, wait]]) → integer ─────────────────────────
    g.set(
        "exec",
        lua.create_function(|_, (prog, args, wait): (String, Option<LuaTable>, Option<bool>)| {
            let mut cmd = std::process::Command::new(&prog);
            if let Some(tbl) = args {
                for v in tbl.sequence_values::<String>() {
                    cmd.arg(v.map_err(LuaError::external)?);
                }
            }
            // 不继承父进程的控制台句柄，防止子进程导致控制台冻结
            use std::process::Stdio;
            cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            }
            if wait.unwrap_or(false) {
                let status = cmd
                    .status()
                    .map_err(|e| LuaError::RuntimeError(format!("exec 失败: {}", e)))?;
                Ok(status.code().unwrap_or(-1))
            } else {
                cmd.spawn()
                    .map_err(|e| LuaError::RuntimeError(format!("exec 启动失败: {}", e)))?;
                Ok(0)
            }
        })?,
    )?;

    // ── screenshot(path [, x, y, w, h]) → path ───────────────────────────
    {
        let tc = ctx.meta.target_class.clone();
        g.set(
            "screenshot",
            lua.create_function(
                move |_,
                      (path, x, y, w, h): (
                    String,
                    Option<i32>,
                    Option<i32>,
                    Option<i32>,
                    Option<i32>,
                )| {
                    let (sw, sh) = window::get_screen_size();
                    let rx = x.unwrap_or(0);
                    let ry = y.unwrap_or(0);
                    let rw = w.unwrap_or(sw);
                    let rh = h.unwrap_or(sh);
                    // 若指定了区域且有目标窗口，则进行客户区→屏幕坐标转换
                    let (sx, sy) = if x.is_some() && !tc.is_empty() {
                        window::client_to_screen(&tc, rx, ry)
                    } else {
                        (rx, ry)
                    };
                    screen::capture_screenshot(&path, sx, sy, rw, rh)
                        .map_err(LuaError::external)?;
                    Ok(path)
                },
            )?,
        )?;
    }

    // ── webhook_upload_file(key, file_path) → media_id ───────────────────
    g.set(
        "webhook_upload_file",
        lua.create_function(|_, (key, file_path): (String, String)| {
            upload_media(&key, &file_path)
                .map_err(|e| LuaError::RuntimeError(format!("webhook_upload_file 失败: {}", e)))
        })?,
    )?;

    // ── webhook_send_file(key, media_id) ─────────────────────────────────
    g.set(
        "webhook_send_file",
        lua.create_function(|_, (key, media_id): (String, String)| {
            let url = format!(
                "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key={}",
                key
            );
            let body = serde_json::json!({
                "msgtype": "file",
                "file": { "media_id": media_id }
            });
            ureq::post(&url)
                .send_json(body)
                .map_err(|e| LuaError::RuntimeError(format!("webhook_send_file 失败: {}", e)))?;
            Ok(())
        })?,
    )?;

    // ── webhook_file(key, file_path) → 上传并发送文件（便捷函数）─────────
    g.set(
        "webhook_file",
        lua.create_function(|_, (key, file_path): (String, String)| {
            let media_id = upload_media(&key, &file_path).map_err(|e| {
                LuaError::RuntimeError(format!("webhook_file 上传失败: {}", e))
            })?;
            let url = format!(
                "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key={}",
                key
            );
            let body = serde_json::json!({
                "msgtype": "file",
                "file": { "media_id": media_id }
            });
            ureq::post(&url)
                .send_json(body)
                .map_err(|e| LuaError::RuntimeError(format!("webhook_file 发送失败: {}", e)))?;
            Ok(())
        })?,
    )?;

    Ok(())
}

// ─── 内部工具函数 ────────────────────────────────────────────────────────────

fn post_delay_ms(ctx: &RunContext) -> u64 {
    (ctx.settings.default_post_delay as f64 / ctx.settings.global_speed.max(0.01)) as u64
}

/// 可中断的休眠：每 10ms 检查一次 running 标志
fn sleep_interruptible(ms: u64, running: &Arc<AtomicBool>) -> LuaResult<()> {
    if ms == 0 {
        return Ok(());
    }
    let end = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < end {
        if !running.load(Ordering::Relaxed) {
            return Err(LuaError::RuntimeError("interrupted".into()));
        }
        let rem = (end - Instant::now()).as_millis().min(10) as u64;
        if rem > 0 {
            std::thread::sleep(Duration::from_millis(rem));
        }
    }
    Ok(())
}

/// 等待指定坐标出现目标颜色
fn wait_for_color(
    target_class: &str,
    x: i32,
    y: i32,
    color: &str,
    timeout_ms: u64,
    error_on_timeout: bool,
    running: &Arc<AtomicBool>,
) -> LuaResult<bool> {
    let (tr, tg, tb) = screen::parse_color(color).map_err(LuaError::external)?;
    let (sx, sy) = window::client_to_screen(target_class, x, y);
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);

    loop {
        if !running.load(Ordering::Relaxed) {
            return Err(LuaError::RuntimeError("interrupted".into()));
        }
        match screen::get_pixel_color(sx, sy) {
            Ok((r, g, b)) if r == tr && g == tg && b == tb => return Ok(true),
            Ok(_) => {}
            Err(e) => return Err(LuaError::external(e)),
        }
        if Instant::now() >= deadline {
            return if error_on_timeout {
                Err(LuaError::RuntimeError(format!(
                    "wait_color 超时: {} 在 ({}, {}) 未出现",
                    color, x, y
                )))
            } else {
                Ok(false)
            };
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// 上传文件到企业微信 webhook，返回 media_id
fn upload_media(key: &str, file_path: &str) -> anyhow::Result<String> {
    let file_data = std::fs::read(file_path)?;
    let file_name = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");

    let boundary = "----MacroEngineBoundary7MA4YWxkTrZu0gW";
    let url = format!(
        "https://qyapi.weixin.qq.com/cgi-bin/webhook/upload_media?key={}&type=file",
        key
    );

    let mut body: Vec<u8> = Vec::new();
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"media\"; filename=\"{}\"; filelength={}\r\nContent-Type: application/octet-stream\r\n\r\n",
            file_name,
            file_data.len()
        )
        .as_bytes(),
    );
    body.extend_from_slice(&file_data);
    body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());

    let content_type = format!("multipart/form-data; boundary={}", boundary);

    let resp = ureq::post(&url)
        .set("Content-Type", &content_type)
        .send_bytes(&body)
        .map_err(|e| anyhow::anyhow!("HTTP 请求失败: {}", e))?;

    let json: serde_json::Value = resp
        .into_json()
        .map_err(|e| anyhow::anyhow!("JSON 解析失败: {}", e))?;

    if json["errcode"].as_i64().unwrap_or(-1) != 0 {
        return Err(anyhow::anyhow!(
            "upload_media 失败(errcode={}): {}",
            json["errcode"].as_i64().unwrap_or(-1),
            json["errmsg"].as_str().unwrap_or("unknown")
        ));
    }

    json["media_id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("响应中无 media_id"))
}
