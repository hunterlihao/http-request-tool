//! 应用主题：配色、间距、中文字体加载
//!
//! 设计方向「信号分析仪」：冷灰仪器面板 + 琥珀色签发动作 + 语义化状态色。

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, Rounding, Stroke};

// ---------- 配色令牌 ----------

/// 主画布背景（最底层）
pub const CANVAS: Color32 = Color32::from_rgb(0x12, 0x14, 0x18);
/// 面板背景
pub const PANEL: Color32 = Color32::from_rgb(0x1A, 0x1D, 0x23);
/// 卡片 / 输入框凹槽背景
pub const SUNKEN: Color32 = Color32::from_rgb(0x10, 0x12, 0x16);
/// 分组 / 悬浮背景
pub const RAISED: Color32 = Color32::from_rgb(0x23, 0x27, 0x2F);
/// 边框线
pub const BORDER: Color32 = Color32::from_rgb(0x2E, 0x33, 0x3C);
/// 主文本
pub const TEXT: Color32 = Color32::from_rgb(0xE6, 0xE9, 0xEE);
/// 次要文本
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x8B, 0x93, 0x9F);
/// 签名色（签发动作 / 焦点）
pub const ACCENT: Color32 = Color32::from_rgb(0xFF, 0x9F, 0x43);

/// 2xx
pub const OK: Color32 = Color32::from_rgb(0x3D, 0xD6, 0x8C);
/// 3xx
pub const REDIRECT: Color32 = Color32::from_rgb(0x5B, 0x8D, 0xEF);
/// 4xx
pub const WARN: Color32 = Color32::from_rgb(0xFF, 0xB0, 0x20);
/// 5xx / 错误
pub const ERROR: Color32 = Color32::from_rgb(0xFF, 0x5C, 0x5C);

/// 方法标签配色
pub fn method_color(method: &str) -> Color32 {
    match method {
        "GET" => Color32::from_rgb(0x6B, 0xD9, 0xA8),
        "POST" => Color32::from_rgb(0xFF, 0x9F, 0x43),
        "PUT" => Color32::from_rgb(0x5B, 0x8D, 0xEF),
        "PATCH" => Color32::from_rgb(0xB4, 0x8C, 0xFF),
        "DELETE" => Color32::from_rgb(0xFF, 0x5C, 0x5C),
        "HEAD" => Color32::from_rgb(0x8B, 0x93, 0x9F),
        "OPTIONS" => Color32::from_rgb(0x7F, 0xD4, 0xE5),
        _ => TEXT_DIM,
    }
}

/// 状态码配色
pub fn status_color(code: u16) -> Color32 {
    match code {
        200..=299 => OK,
        300..=399 => REDIRECT,
        400..=499 => WARN,
        500..=599 => ERROR,
        _ => TEXT_DIM,
    }
}

/// 应用主题（字体 + 视觉风格）
pub fn apply(ctx: &egui::Context) {
    install_chinese_font(ctx);
    ctx.set_visuals(make_visuals());

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.menu_margin = egui::Margin::same(8.0);
    style.visuals = make_visuals();
    ctx.set_style(style);
}

/// 加载系统中文字体（Windows：优先雅黑 ttc，回退黑体 ttf）
fn install_chinese_font(ctx: &egui::Context) {
    // 常见 Windows 中文字体路径
    const CANDIDATES: &[&str] = &[
        "C:/Windows/Fonts/msyh.ttc",   // 微软雅黑
        "C:/Windows/Fonts/msyhl.ttc",  // 微软雅黑 Light
        "C:/Windows/Fonts/simhei.ttf", // 黑体
        "C:/Windows/Fonts/simsun.ttc", // 宋体
    ];

    let mut chosen: Option<(String, Vec<u8>)> = None;
    for path in CANDIDATES {
        if let Ok(bytes) = std::fs::read(path) {
            if bytes.len() > 1024 {
                chosen = Some((path.to_string(), bytes));
                break;
            }
        }
    }

    let Some((path, bytes)) = chosen else {
        return; // 无中文字体时不阻塞，拉丁字照常显示
    };

    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert("cjk".into(), FontData::from_owned(bytes));

    // 拉丁字体在前（优先用于拉丁字符），中文字体在后作为回退
    if let Some(list) = fonts.families.get_mut(&FontFamily::Proportional) {
        list.push("cjk".into());
    }
    if let Some(list) = fonts.families.get_mut(&FontFamily::Monospace) {
        list.push("cjk".into());
    }

    // 记录选中的字体路径，便于排查
    println!("[theme] 已加载中文字体: {path}");
    ctx.set_fonts(fonts);
}

/// 构造自定义视觉样式
fn make_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();

    v.override_text_color = Some(TEXT);
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.extreme_bg_color = SUNKEN;
    v.faint_bg_color = RAISED;
    v.window_stroke = Stroke::new(1.0, BORDER);
    v.window_rounding = Rounding::same(10.0);
    v.menu_rounding = Rounding::same(8.0);

    v.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.hyperlink_color = ACCENT;

    let r = Rounding::same(7.0);

    // 非交互（标签、背景）
    v.widgets.noninteractive.bg_fill = PANEL;
    v.widgets.noninteractive.weak_bg_fill = PANEL;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DIM);
    v.widgets.noninteractive.rounding = r;

    // 未交互控件（输入框、按钮）
    v.widgets.inactive.bg_fill = RAISED;
    v.widgets.inactive.weak_bg_fill = RAISED;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.inactive.rounding = r;

    // 悬停
    v.widgets.hovered.bg_fill = Color32::from_rgb(0x2C, 0x31, 0x3B);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x2C, 0x31, 0x3B);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT.gamma_multiply(0.7));
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    v.widgets.hovered.rounding = r;

    // 激活 / 按下
    v.widgets.active.bg_fill = ACCENT.gamma_multiply(0.25);
    v.widgets.active.weak_bg_fill = ACCENT.gamma_multiply(0.25);
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    v.widgets.active.rounding = r;

    // 打开的下拉
    v.widgets.open.bg_fill = RAISED;
    v.widgets.open.weak_bg_fill = RAISED;
    v.widgets.open.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.open.rounding = r;

    v
}
