//! 应用主题：浅色调色板、间距、中文字体加载
//!
//! 设计方向「纸面仪表盘」：浅色面板 + 琥珀色签发动作 + 语义化状态色。

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, Rounding, Stroke};

/// 一套完整调色板
pub struct Palette {
    pub canvas: Color32,
    pub panel: Color32,
    pub sunken: Color32,
    pub raised: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub ok: Color32,
    pub redirect: Color32,
    pub warn: Color32,
    pub error: Color32,
    pub m_get: Color32,
    pub m_post: Color32,
    pub m_put: Color32,
    pub m_patch: Color32,
    pub m_delete: Color32,
    pub m_head: Color32,
    pub m_options: Color32,
}

/// 浅色：纸面仪表盘
const PALETTE: Palette = Palette {
    canvas: Color32::from_rgb(0xF2, 0xF4, 0xF7),
    panel: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    sunken: Color32::from_rgb(0xEB, 0xEE, 0xF3),
    raised: Color32::from_rgb(0xE5, 0xE9, 0xF0),
    border: Color32::from_rgb(0xD3, 0xD8, 0xE0),
    text: Color32::from_rgb(0x1F, 0x24, 0x30),
    text_dim: Color32::from_rgb(0x66, 0x70, 0x85),
    accent: Color32::from_rgb(0xD9, 0x77, 0x06),
    on_accent: Color32::WHITE,
    ok: Color32::from_rgb(0x12, 0x80, 0x5C),
    redirect: Color32::from_rgb(0x25, 0x63, 0xEB),
    warn: Color32::from_rgb(0xB4, 0x53, 0x09),
    error: Color32::from_rgb(0xDC, 0x26, 0x26),
    m_get: Color32::from_rgb(0x0F, 0x9D, 0x58),
    m_post: Color32::from_rgb(0xD9, 0x77, 0x06),
    m_put: Color32::from_rgb(0x25, 0x63, 0xEB),
    m_patch: Color32::from_rgb(0x7C, 0x3A, 0xED),
    m_delete: Color32::from_rgb(0xDC, 0x26, 0x26),
    m_head: Color32::from_rgb(0x66, 0x70, 0x85),
    m_options: Color32::from_rgb(0x08, 0x91, 0xB2),
};

fn pal() -> &'static Palette {
    &PALETTE
}

// ---------- 动态取色 ----------

pub fn canvas() -> Color32 {
    pal().canvas
}
pub fn panel() -> Color32 {
    pal().panel
}
pub fn raised() -> Color32 {
    pal().raised
}
pub fn border() -> Color32 {
    pal().border
}
pub fn text() -> Color32 {
    pal().text
}
pub fn text_dim() -> Color32 {
    pal().text_dim
}
pub fn accent() -> Color32 {
    pal().accent
}
pub fn on_accent() -> Color32 {
    pal().on_accent
}
pub fn ok() -> Color32 {
    pal().ok
}
pub fn redirect() -> Color32 {
    pal().redirect
}
pub fn error() -> Color32 {
    pal().error
}

/// 方法标签配色
pub fn method_color(method: &str) -> Color32 {
    let p = pal();
    match method {
        "GET" => p.m_get,
        "POST" => p.m_post,
        "PUT" => p.m_put,
        "PATCH" => p.m_patch,
        "DELETE" => p.m_delete,
        "HEAD" => p.m_head,
        "OPTIONS" => p.m_options,
        _ => p.text_dim,
    }
}

/// 状态码配色
pub fn status_color(code: u16) -> Color32 {
    let p = pal();
    match code {
        200..=299 => p.ok,
        300..=399 => p.redirect,
        400..=499 => p.warn,
        500..=599 => p.error,
        _ => p.text_dim,
    }
}

// ---------- 应用主题 ----------

/// 应用主题：加载字体 + 设置样式
pub fn apply(ctx: &egui::Context) {
    install_chinese_font(ctx);
    let mut style = (*ctx.style()).clone();
    style.visuals = make_visuals();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.menu_margin = egui::Margin::same(8.0);
    style.spacing.interact_size.y = 30.0;
    ctx.set_style(style);
}

/// 加载系统中文字体（Windows：优先雅黑 ttc，回退黑体 ttf）
fn install_chinese_font(ctx: &egui::Context) {
    const CANDIDATES: &[&str] = &[
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/msyhl.ttc",
        "C:/Windows/Fonts/simhei.ttf",
        "C:/Windows/Fonts/simsun.ttc",
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
        return;
    };

    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert("cjk".into(), FontData::from_owned(bytes));

    if let Some(list) = fonts.families.get_mut(&FontFamily::Proportional) {
        list.push("cjk".into());
    }
    if let Some(list) = fonts.families.get_mut(&FontFamily::Monospace) {
        list.push("cjk".into());
    }

    println!("[theme] 已加载中文字体: {path}");
    ctx.set_fonts(fonts);
}

/// 依据调色板构造视觉样式
fn make_visuals() -> egui::Visuals {
    let p = pal();
    let mut v = egui::Visuals::light();

    v.override_text_color = Some(p.text);
    v.panel_fill = p.panel;
    v.window_fill = p.panel;
    v.extreme_bg_color = p.sunken;
    v.faint_bg_color = p.raised;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_rounding = Rounding::same(10.0);
    v.menu_rounding = Rounding::same(8.0);

    v.selection.bg_fill = p.accent.gamma_multiply(0.30);
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.hyperlink_color = p.accent;

    let r = Rounding::same(7.0);

    v.widgets.noninteractive.bg_fill = p.panel;
    v.widgets.noninteractive.weak_bg_fill = p.panel;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text_dim);
    v.widgets.noninteractive.rounding = r;

    v.widgets.inactive.bg_fill = p.raised;
    v.widgets.inactive.weak_bg_fill = p.raised;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.inactive.rounding = r;

    let hover = Color32::from_rgb(0xDA, 0xDF, 0xE8);
    v.widgets.hovered.bg_fill = hover;
    v.widgets.hovered.weak_bg_fill = hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent.gamma_multiply(0.7));
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.hovered.rounding = r;

    v.widgets.active.bg_fill = p.accent.gamma_multiply(0.22);
    v.widgets.active.weak_bg_fill = p.accent.gamma_multiply(0.22);
    v.widgets.active.bg_stroke = Stroke::new(1.0, p.accent);
    v.widgets.active.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.active.rounding = r;

    v.widgets.open.bg_fill = p.raised;
    v.widgets.open.weak_bg_fill = p.raised;
    v.widgets.open.bg_stroke = Stroke::new(1.0, p.accent);
    v.widgets.open.rounding = r;

    v
}
