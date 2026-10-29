//! 可复用的 UI 小组件

use eframe::egui::{self, Color32, Margin, Response, RichText, Rounding, Stroke, TextEdit};

use crate::model::request::KVPair;
use crate::theme;

/// 小节标题
pub fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(11.5)
            .color(theme::text_dim())
            .strong(),
    );
}

/// 彩色徽章
pub fn badge(ui: &mut egui::Ui, text: &str, color: Color32) -> Response {
    egui::Frame::none()
        .fill(color.gamma_multiply(0.10))
        .rounding(Rounding::same(6.0))
        .inner_margin(Margin::symmetric(8.0, 2.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .color(color)
                    .strong()
                    .monospace()
                    .size(11.5),
            )
        })
        .response
}

/// 弱提示文本
pub fn hint(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(11.5).color(theme::text_dim()));
}

/// 空状态提示（居中、轻量的引导）
pub fn empty_state(ui: &mut egui::Ui, title: &str, desc: &str) {
    ui.add_space(28.0);
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new(title)
                .size(13.5)
                .color(theme::text_dim())
                .strong(),
        );
        ui.add_space(6.0);
        ui.label(RichText::new(desc).size(12.0).color(theme::text_muted()));
    });
    ui.add_space(16.0);
}

/// 主操作按钮（橙色签发，全屏唯一高饱和元素）
pub fn accent_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> Response {
    let (fill, txt) = if enabled {
        (theme::accent(), theme::on_accent())
    } else {
        (theme::raised(), theme::text_muted())
    };
    let btn = egui::Button::new(RichText::new(text).strong().color(txt))
        .fill(fill)
        .stroke(Stroke::NONE)
        .rounding(Rounding::same(8.0))
        .min_size(egui::vec2(100.0, 32.0));
    ui.add_enabled(enabled, btn)
}

/// 次要按钮（白底细描边）
pub fn subtle_button(ui: &mut egui::Ui, text: &str) -> Response {
    let btn = egui::Button::new(RichText::new(text).color(theme::text()).size(12.5))
        .fill(theme::panel())
        .stroke(Stroke::new(1.0, theme::border()))
        .rounding(Rounding::same(8.0))
        .min_size(egui::vec2(0.0, 32.0));
    ui.add(btn)
}

/// 构造等高下拉按钮（32 高，与输入框一致）
pub fn select_button(text: impl Into<egui::WidgetText>, width: f32) -> egui::Button<'static> {
    egui::Button::new(text)
        .fill(theme::panel())
        .stroke(Stroke::new(1.0, theme::border()))
        .rounding(Rounding::same(8.0))
        .min_size(egui::vec2(width, 32.0))
}

/// 等高下拉选择器（替代 ComboBox，保证与输入框同高）
pub fn select_popup(
    ui: &mut egui::Ui,
    id: &str,
    width: f32,
    button: egui::Button<'static>,
    items: &[(egui::WidgetText, bool)],
) -> Option<usize> {
    let popup_id = ui.make_persistent_id(id);
    let resp = ui.add(button);
    if resp.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }

    let mut picked = None;
    egui::popup::popup_below_widget(
        ui,
        popup_id,
        &resp,
        egui::popup::PopupCloseBehavior::CloseOnClick,
        |ui| {
            ui.set_min_width(width);
            ui.spacing_mut().item_spacing.y = 2.0;
            for (i, (label, selected)) in items.iter().enumerate() {
                let item = egui::Button::new(label.clone())
                    .fill(if *selected {
                        theme::accent().gamma_multiply(0.10)
                    } else {
                        Color32::TRANSPARENT
                    })
                    .frame(false)
                    .min_size(egui::vec2(width, 30.0));
                if ui.add(item).clicked() {
                    picked = Some(i);
                    ui.memory_mut(|m| m.close_popup());
                }
            }
        },
    );
    picked
}

/// 在行内靠右放置内容（可靠地占满剩余宽度）
pub fn right_aligned(ui: &mut egui::Ui, height: f32, add: impl FnOnce(&mut egui::Ui)) {
    let w = ui.available_width();
    if w <= 1.0 {
        return;
    }
    ui.allocate_ui_with_layout(
        egui::vec2(w, height),
        egui::Layout::right_to_left(egui::Align::Center),
        add,
    );
}

/// 裸文字链接按钮
pub fn link_button(ui: &mut egui::Ui, text: &str, color: Color32) -> Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(11.5).color(color))
            .fill(Color32::TRANSPARENT)
            .frame(false),
    )
}

/// 键值对编辑器（Params / Headers / Form 共用）
pub fn kv_editor(
    ui: &mut egui::Ui,
    id: &str,
    rows: &mut Vec<KVPair>,
    key_hint: &str,
    val_hint: &str,
) {
    let mut remove: Option<usize> = None;

    let check_w = 24.0;
    let key_w = 200.0;
    let del_w = 24.0;
    let spacing = 10.0;
    let row_h = 32.0;
    let val_w = (ui.available_width() - check_w - key_w - del_w - spacing * 3.0).max(120.0);

    fn header(ui: &mut egui::Ui, text: &str, w: f32) {
        ui.add_sized(
            [w, 16.0],
            egui::Label::new(RichText::new(text).size(11.0).color(theme::text_dim())),
        );
    }

    egui::Grid::new(format!("{id}_grid"))
        .num_columns(4)
        .spacing([spacing, 8.0])
        .show(ui, |ui| {
            header(ui, "启用", check_w);
            header(ui, key_hint, key_w);
            header(ui, val_hint, val_w);
            ui.label("");
            ui.end_row();

            for (i, row) in rows.iter_mut().enumerate() {
                ui.add_sized(
                    [check_w, row_h],
                    egui::Checkbox::without_text(&mut row.enabled),
                );
                ui.add_sized(
                    [key_w, row_h],
                    TextEdit::singleline(&mut row.key)
                        .hint_text("key")
                        .vertical_align(egui::Align::Center),
                );
                ui.add_sized(
                    [val_w, row_h],
                    TextEdit::singleline(&mut row.value)
                        .hint_text("value")
                        .vertical_align(egui::Align::Center),
                );
                if ui
                    .add_sized(
                        [del_w, row_h],
                        egui::Button::new(RichText::new("✕").color(theme::text_muted()))
                            .fill(Color32::TRANSPARENT)
                            .frame(false),
                    )
                    .clicked()
                {
                    remove = Some(i);
                }
                ui.end_row();
            }
        });

    if let Some(i) = remove {
        rows.remove(i);
    }

    ui.add_space(8.0);
    if subtle_button(ui, "+ 添加一行").clicked() {
        rows.push(KVPair {
            enabled: true,
            ..Default::default()
        });
    }
}
