-- ============================================================
-- 脚本名称：逃离塔克夫物品购买 v3
-- 说明：依次巡检 Prapor / Skier / Peacekeeper / Jaeger 各商人，
--       尝试购买指定子弹与武器，完成后进行 10 分钟分段等待，
--       最后检测游戏是否出现 ERROR。
--       仅适用于4K屏幕 缩放150%
-- ============================================================

meta = {
    name         = "逃离塔克夫物品购买v3",
    description  = "Tarkov 商人子弹/武器批量购买 + 10min 分段等待 + 报错检测",
    trigger_key  = "F9",
    trigger_mode = "toggle",
    target_class = "UnityWndClass",
}

settings = {
    default_click_hold = 30,
    default_post_delay = 0,   -- 所有延迟均手动控制
    global_speed       = 1.0,
}

-- ── 配置 ──────────────────────────────────────────────────────
local WEBHOOK_KEY = "***REDACTED***"

-- ── 辅助函数 ──────────────────────────────────────────────────

-- 普通随机点击（移动 → 50~200ms → 点击 → 1000~1500ms）
local function rand_click(x1, y1, x2, y2)
    local x = math.random(x1, x2)
    local y = math.random(y1, y2)
    mouse_move(x, y)
    delay_rand(50, 200)
    mouse_click("left")
    delay_rand(1000, 1500)
end

-- 商人点击（移动 → 50~200ms → 点击 → 3000~3500ms）
local function merchant_click(x1, y1, x2, y2)
    local x = math.random(x1, x2)
    local y = math.random(y1, y2)
    mouse_move(x, y)
    delay_rand(50, 200)
    mouse_click("left")
    delay_rand(3000, 3500)
end

-- 购买：按 3 次 9，空格，Y，等待 5500~6000ms
local function buy()
    mouse_move(2110, 767)
    mouse_click("left")
    key_click("9", 30, 50)
    key_click("9", 30, 50)
    key_click("9", 30, 50)
    key_click("SPACE", 30, 200)
    key_click("Y", 30, 200)
    delay_rand(5500, 6000)
end

-- 点击皇冠分区（{1242,465}~{1290,516}）
local function click_crown_section()
    rand_click(1242, 465, 1290, 516)
end

-- 点击无分区（{969,465}~{1019,513}）
local function click_no_section()
    rand_click(969, 465, 1019, 513)
end

-- 切换子弹分类（{1308,939}~{1358,990}）
local function click_bullet_category()
    rand_click(1308, 939, 1358, 990)
end

-- 切换武器分类（{1310,804}~{1355,852}）
local function click_weapon_category()
    rand_click(1310, 804, 1355, 852)
end

-- ── 启动 ──────────────────────────────────────────────────────
function on_start()
    log("逃离塔克夫物品购买 已启动")
end

-- ── 主循环（once 模式下仅执行一次）──────────────────────────
function on_loop()

    -- 1、等待商人界面加载
    log("1、等待商人界面加载")
    if not if_color(2555, 2111, "#9F9D90", 60000) then
        pcall(webhook, WEBHOOK_KEY, "【Macro_Engine】界面加载超时，已中止！")
        delay(10000)
        exec("taskkill", {"/F", "/IM", "EscapeFromTarkov.exe"})
        stop()
    end
    delay_rand(1000, 1500)

    -- 2、点击商人 Prapor
    log("2、点击商人 Prapor")
    merchant_click(27, 114, 242, 420)

    -- 3、检查子弹 5.45x39mm BT
    log("3、检查子弹 5.45x39mm BT")
    click_bullet_category()
    rand_click(654, 537, 760, 642)
    buy()

    -- 4、检查子弹 9x39mm SP6
    log("4、检查子弹 9x39mm SP6")
    rand_click(24, 919, 125, 1016)
    buy()

    -- 5、检查子弹 7.62x54R 7N1
    log("5、检查子弹 7.62x54R 7N1")
    rand_click(528, 916, 631, 1019)
    buy()

    -- 6、检查子弹 12.7x55mm PS12B
    log("6、检查子弹 12.7x55mm PS12B")
    rand_click(277, 1043, 378, 1141)
    buy()

    -- 7、检查子弹 9x19mm 7N31
    log("7、检查子弹 9x19mm 7N31")
    rand_click(907, 1043, 1009, 1147)
    buy()

    -- 8、检查子弹 5.45x39mm 7N40
    log("8、检查子弹 5.45x39mm 7N40")
    rand_click(1156, 1043, 1265, 1150)
    buy()

    -- 9、检查子弹 7.62x39mm PP gzh
    log("9、检查子弹 7.62x39mm PP gzh")
    rand_click(25, 1169, 127, 1270)
    buy()

    -- 10、检查武器 火箭筒
    log("10、检查武器 火箭筒")
    click_crown_section()
    click_weapon_category()
    rand_click(27, 1799, 501, 1894)
    buy()
    click_no_section()

    -- 11、点击商人 Skier
    log("11、点击商人 Skier")
    merchant_click(776, 117, 987, 423)

    -- 12、检查武器 TRG M10
    log("12、检查武器 TRG M10")
    click_crown_section()
    click_weapon_category()
    rand_click(25, 1550, 750, 1765)
    buy()
    click_no_section()

    -- 13、点击商人 Peacekeeper
    log("13、点击商人 Peacekeeper")
    merchant_click(1027, 118, 1233, 421)

    -- 14、检查子弹 7.62x51mm M80
    log("14、检查子弹 7.62x51mm M80")
    click_bullet_category()
    rand_click(272, 533, 382, 644)
    buy()

    -- 15、检查子弹 5.56x45mm M856A1
    log("15、检查子弹 5.56x45mm M856A1")
    rand_click(527, 534, 631, 642)
    buy()

    -- 16、检查子弹 6.8x51mm SIG FMJ
    log("16、检查子弹 6.8x51mm SIG FMJ")
    rand_click(148, 789, 254, 894)
    buy()

    -- 17、检查子弹 .50 BMG M21
    log("17、检查子弹 .50 BMG M21")
    rand_click(400, 790, 505, 894)
    buy()

    -- 18、检查子弹 .50 BMG M33
    log("18、检查子弹 .50 BMG M33")
    rand_click(529, 789, 631, 892)
    buy()

    -- 19、点击商人 Jaeger
    log("19、点击商人 Jaeger")
    merchant_click(1776, 113, 1982, 428)

    -- 20、检查子弹 12/70 RIP
    log("20、检查子弹 12/70 RIP")
    click_bullet_category()
    rand_click(781, 667, 879, 764)
    buy()

    -- 21、检查子弹 12/70 8.5mm 鹿弹
    log("21、检查子弹 12/70 8.5mm 鹿弹")
    rand_click(1160, 664, 1263, 768)
    buy()

    -- 22、检查子弹 12/70 箭形弹
    log("22、检查子弹 12/70 箭形弹")
    rand_click(1034, 792, 1134, 889)
    buy()

    -- 23、检查子弹 .338 Lapua Magnum FMJ
    log("23、检查子弹 .338 Lapua Magnum FMJ")
    rand_click(1034, 918, 1136, 1018)
    buy()

    -- 24、检查子弹 12/70 食人鱼
    log("24、检查子弹 12/70 食人鱼")
    rand_click(1032, 1042, 1135, 1146)
    buy()

    -- 25、判断游戏是否存在报错
    log("25、判断游戏是否存在报错")
    local text = ocr_region(1400, 867, 2433, 1371)
    -- 去除所有空白字符后再做匹配
    local clean = text:gsub("%s+", "")
    if clean:find("严重错误") or clean:find("ERROR") or clean:find("error") then
        log("检测到 严重错误：" .. clean)
        pcall(webhook, WEBHOOK_KEY, "【Macro_Engine】检测到错误：" .. clean)
        delay(10000)
        exec("taskkill", {"/F", "/IM", "EscapeFromTarkov.exe"})
        stop()
    end
    
    -- 26、进行 10 分钟分段等待
    log("26、进行10分钟分段等待")
    local wait_points = {
        {2400, 2120},
        {2800, 2120},
        {3000, 2120},
        {3200, 2120},
        {2570, 2120},
        {2400, 2120},
        {2800, 2120},
        {3000, 2120},
        {3200, 2120},
        {2570, 2120},
    }
    for i, pt in ipairs(wait_points) do
        log("  分段等待 " .. i .. "/10，等待 60 秒...")
        delay(60000)
        mouse_click_rand(pt[1] - 50, pt[2] - 20, pt[1] + 50, pt[2] + 20)
        delay(1000)
        key_click("ESC")
        delay(1000)
    end


    log("完成")
    stop()
end

-- ── 停止 ──────────────────────────────────────────────────────
function on_release()
    log("逃离塔克夫物品购买 v3 已停止")
end


--[[
v1原始提示词：

请你帮我写一个脚本，脚本指南请看“.\docs\脚本编写指南.md”，脚本具体内容大致如下：

脚本名称：Tarkov_Buy_Item_v1.lua

name = "逃离塔克夫物品购买"
target_class = "UnityWndClass"

脚本步骤及log名称：
```
1、等待商人界面加载
2、点击商人 Prapor
3、检查子弹 5.45x39mm BT
4、检查子弹 9x39mm SP6
5、检查子弹 7.62x54R 7N1
6、检查子弹 12.7x55mm PS12B
7、检查子弹 9x19mm 7N31
8、检查子弹 5.45x39mm 7N40
9、检查子弹 7.62x39mm PP gzh
10、检查武器 火箭筒
11、点击商人 Skier
12、检查武器 TRG M10
13、点击商人 Peacekeeper
14、检查子弹 7.62x51mm M80
15、检查子弹 5.56x45mm M856A1
16、检查子弹 6.8x51mm SIG FMJ
17、检查子弹 .50 BMG M21
18、检查子弹 .50 BMG M33
19、点击商人 Jaeger
20、检查子弹 12/70 RIP
21、检查子弹 12/70 8.5mm 鹿弹
22、检查子弹 12/70 箭形弹
23、检查子弹 .338 Lapua Magnum FMJ
24、检查子弹 12/70 食人鱼
25、进行10分钟分段等待
26、判断游戏是否存在报错
```

每个步骤的对应操作：
```
未说明延时使用1000~1500ms的随机延时，移动鼠标和左键点击之间使用50~200ms的随机延时。

购买：在点击对应位置后，按下3次9，按下空格，按下Y，等待5500~6000ms随机延时。
点击皇冠分区：在{1242,465}~{1290,516}区域内点击。
点击无分区：在{969,465}~{1019,513}区域内点击。
切换子弹分类：在{1308,939}~{1358,990}区域内点击。
切换武器分类：在{1310,804}~{1355,852}区域内点击。

1、在{2555,2111}等待#9F9D90颜色60s，失败发送webhook。
2、在{27,114}~{242,420}区域内点击，等待3000~3500ms随机延时。
3、切换子弹分类，在{654,537}~{760,642}区域内点击，点击购买。
4、在{24,919}~{125,1016}区域内点击，点击购买。
5、在{528,916}~{631,1019}区域内点击，点击购买。
6、在{277,1043}~{378,1141}区域内点击，点击购买。
7、在{907,1043}~{1009,1147}区域内点击，点击购买。
8、在{1156,1043}~{1265,1150}区域内点击，点击购买。
9、在{25,1169}~{127,1270}区域内点击，点击购买。
10、点击皇冠分区，切换武器分类，在{27,1799}~{501,1894}区域内点击，点击购买，点击无分区。
11、在{776,117}~{987,423}区域内点击，等待3000~3500ms随机延时。
12、点击皇冠分区，切换武器分类，在{25,1550}~{750,1765}区域内点击，点击购买，点击无分区。
13、在{1027,118}~{1233,421}区域内点击，等待3000~3500ms随机延时。
14、切换子弹分类，在{272,533}~{382,644}区域内点击，点击购买。
15、在{527,534}~{631,642}区域内点击，点击购买。
16、在{148,789}~{254,894}区域内点击，点击购买。
17、在{400,790}~{505,894}区域内点击，点击购买。
18、在{529,789}~{631,892}区域内点击，点击购买。
19、在{1776,113}~{1982,428}区域内点击，等待3000~3500ms随机延时。
20、切换子弹分类，在{781,667}~{879,764}区域内点击，点击购买。
21、在{1160,664}~{1263,768}区域内点击，点击购买。
22、在{1034,792}~{1134,889}区域内点击，点击购买。
23、在{1034,918}~{1136,1018}区域内点击，点击购买。
24、在{1032,1042}~{1135,1146}区域内点击，点击购买。
25、每1分钟点击一个位置，依次是{2400,2120},{2800,2120},{3000,2120},{3200,2120},{2570,2120},{2400,2120},{2800,2120},{3000,2120},{3200,2120},{2570,2120}，每点击一个坐标都需要按下1次ESC。这10个坐标使用mouse_click_rand，区域全部相对其X±50,Y±20，例如{2570,2120}就是{2520,2100}~{2620,2140}。步骤1s延时。
26、在{1200,867}~{2693,1365}区域内识别是否存在“ERROR”，若存在则报错。
```


]]