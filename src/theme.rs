//! 应用主题：精密工作台风格
//!
//! 中性冷灰画布 + 纯白卡片，橙色作为唯一高饱和签名色。

use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, FontId, Rounding, Stroke, TextStyle,
};

/// 一套完整调色板
pub struct Palette {
    pub canvas: Color32,
    pub panel: Color32,
    pub sunken: Color32,
    pub raised: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub text_muted: Color32,
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

const PALETTE: Palette = Palette {
    canvas: Color32::from_rgb(0xF4, 0xF5, 0xF7),
    panel: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    sunken: Color32::from_rgb(0xF9, 0xFA, 0xFB),
    raised: Color32::from_rgb(0xF3, 0xF4, 0xF6),
    border: Color32::from_rgb(0xE5, 0xE7, 0xEB),
    text: Color32::from_rgb(0x11, 0x18, 0x27),
    text_dim: Color32::from_rgb(0x6B, 0x72, 0x80),
    text_muted: Color32::from_rgb(0x9C, 0xA3, 0xAF),
    accent: Color32::from_rgb(0xEA, 0x58, 0x0C),
    on_accent: Color32::WHITE,
    ok: Color32::from_rgb(0x05, 0x96, 0x69),
    redirect: Color32::from_rgb(0x25, 0x63, 0xEB),
    warn: Color32::from_rgb(0xB4, 0x53, 0x09),
    error: Color32::from_rgb(0xDC, 0x26, 0x26),
    m_get: Color32::from_rgb(0x05, 0x96, 0x69),
    m_post: Color32::from_rgb(0xEA, 0x58, 0x0C),
    m_put: Color32::from_rgb(0x25, 0x63, 0xEB),
    m_patch: Color32::from_rgb(0x7C, 0x3A, 0xED),
    m_delete: Color32::from_rgb(0xDC, 0x26, 0x26),
    m_head: Color32::from_rgb(0x6B, 0x72, 0x80),
    m_options: Color32::from_rgb(0x08, 0x91, 0xB2),
};

fn pal() -> &'static Palette {
    &PALETTE
}

// ---------- 取色 ----------

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
pub fn text_muted() -> Color32 {
    pal().text_muted
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
pub fn warn() -> Color32 {
    pal().warn
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
    if std::env::var_os("HTTP_TOOL_NO_CJK").is_none() {
        install_chinese_font(ctx);
    }

    let mut style = (*ctx.style()).clone();
    style.visuals = make_visuals();

    // 间距节奏
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 6.0);
    style.spacing.menu_margin = egui::Margin::same(8.0);
    style.spacing.interact_size = egui::vec2(40.0, 32.0);
    style.spacing.indent = 16.0;

    // 字号层级
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(16.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(13.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(11.5, FontFamily::Proportional),
        ),
        (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
    ]
    .into();

    ctx.set_style(style);
}

/// 加载系统中文字体（Windows：优先雅黑 ttc，回退黑体 ttf）
fn install_chinese_font(ctx: &egui::Context) {
    const CANDIDATES: &[&str] = &[
        "C:/Windows/Fonts/simhei.ttf",
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/msyhl.ttc",
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

    // 泄漏为 'static：epaint 对 Cow::Owned 会按每个字号克隆一份字体数据，
    // 而 Cow::Borrowed 只在内存中保留一份并以借用方式解析。
    let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());

    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert("cjk".into(), FontData::from_static(leaked));

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
    v.extreme_bg_color = p.sunken; // 输入框底色
    v.faint_bg_color = p.raised;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_rounding = Rounding::same(12.0);
    v.menu_rounding = Rounding::same(10.0);

    v.selection.bg_fill = p.accent.gamma_multiply(0.18);
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.hyperlink_color = p.accent;

    let r = Rounding::same(8.0);

    // 非交互
    v.widgets.noninteractive.bg_fill = p.panel;
    v.widgets.noninteractive.weak_bg_fill = p.panel;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text_dim);
    v.widgets.noninteractive.rounding = r;

    // 未交互（按钮底色用 raised，输入框用 extreme_bg_color）
    v.widgets.inactive.bg_fill = p.raised;
    v.widgets.inactive.weak_bg_fill = p.raised;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.inactive.rounding = r;

    // 悬停
    let hover_bg = Color32::from_rgb(0xEC, 0xEE, 0xF1);
    v.widgets.hovered.bg_fill = hover_bg;
    v.widgets.hovered.weak_bg_fill = hover_bg;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0xD1, 0xD5, 0xDB));
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.hovered.rounding = r;

    // 激活 / 聚焦
    v.widgets.active.bg_fill = p.accent.gamma_multiply(0.10);
    v.widgets.active.weak_bg_fill = p.accent.gamma_multiply(0.10);
    v.widgets.active.bg_stroke = Stroke::new(1.5, p.accent);
    v.widgets.active.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.active.rounding = r;

    // 展开的下拉
    v.widgets.open.bg_fill = p.panel;
    v.widgets.open.weak_bg_fill = p.panel;
    v.widgets.open.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.open.rounding = r;

    v
}

/// 统一的卡片外框
pub fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(panel())
        .stroke(Stroke::new(1.0, border()))
        .rounding(Rounding::same(12.0))
        .inner_margin(egui::Margin::same(16.0))
}
