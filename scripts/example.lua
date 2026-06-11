-- ============================================================
-- MacroEngine (Rust) — 示例脚本
-- 展示所有可用 API 的用法
-- ============================================================

meta = {
    name        = "示例脚本",
    description = "演示所有 API",
    trigger_key  = "F9",
    trigger_mode = "toggle",   -- hold | toggle | once
    target_class = "",         -- 空 = 全局生效，填 "UnityWndClass" 等限定窗口
}

settings = {
    default_click_hold  = 30,   -- 默认按键按住时长 (ms)
    default_post_delay  = 15,   -- 每个动作后默认等待 (ms)
    global_speed        = 1.0,  -- 速度系数 (2.0 = 快一倍)
}

-- 启动时执行一次（可选）
function on_start()
    log("脚本启动！")
end

-- 主循环，反复执行直到停止
function on_loop()
    -- ── 鼠标操作 ────────────────────────────────────────────
    mouse_move(500, 400, true)          -- 绝对移动（屏幕/窗口客户区坐标）
    mouse_move(10, 0, false)            -- 相对移动（从当前位置偏移 +10, 0）
    mouse_click("left", 30, 100)        -- 左键点击：按住 30ms，之后等 100ms
    mouse_down("right")                 -- 按下右键（不松开）
    delay(200)
    mouse_up("right")                   -- 松开右键

    -- ── 键盘操作 ────────────────────────────────────────────
    key_click("F", 50, 100)             -- 按 F 键，按住 50ms，之后等 100ms
    key_click("ESC", 30)
    key_down("SHIFT")
    key_click("A", 30)                  -- Shift+A
    key_up("SHIFT")

    -- ── 延迟与冷却 ───────────────────────────────────────────
    delay(500)                          -- 等待 500ms（可被停止键中断）
    if cooldown("my_tag", 3000) then    -- 3 秒冷却：返回 true 表示可执行
        log("冷却完毕，执行动作")
        key_click("E", 30)
    end

    -- ── 颜色检测 ────────────────────────────────────────────
    local color = get_color(960, 540)   -- 获取该坐标像素颜色 "#RRGGBB"
    log("像素颜色: " .. color)

    -- wait_color: 等到该坐标出现指定颜色，超时则报错停止
    -- wait_color(960, 540, "#FF0000", 3000)

    -- if_color: 检测颜色，超时不报错，返回 false
    if if_color(100, 100, "#FFFFFF", 50) then
        log("检测到白色，捡取道具")
        key_click("F", 30)
    end

    -- ── OCR 文字识别 ─────────────────────────────────────────
    local text = ocr_region(600, 50, 1000, 130)  -- 识别屏幕区域 (x1, y1, x2, y2)
    if text:find("确认") then
        log("检测到确认按钮，点击")
        mouse_move(800, 90, true)
        mouse_click("left", 30)
    end

    -- ── 模块导入 ─────────────────────────────────────────────
    -- import("子模块/购买流程")  -- 执行 scripts/子模块/购买流程.lua

    delay(1000)
end

-- 停止时执行（可选）
function on_release()
    log("脚本停止，执行清理")
end
