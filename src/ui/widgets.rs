//! 可复用的 UI 小组件

use eframe::egui::{self, Color32, Margin, Response, RichText, Rounding, TextEdit};

use crate::model::request::KVPair;
use crate::theme;

/// 小节标题
pub fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(12.0)
            .color(theme::text_dim())
            .strong(),
    );
}

/// 彩色徽章
pub fn badge(ui: &mut egui::Ui, text: &str, color: Color32) -> Response {
    egui::Frame::none()
        .fill(color.gamma_multiply(0.16))
        .rounding(Rounding::same(6.0))
        .inner_margin(Margin::symmetric(8.0, 3.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(color).strong().monospace().size(12.0))
        })
        .response
}

/// 弱提示文本
pub fn hint(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(12.0).color(theme::text_dim()));
}

/// 空状态提示（居中、轻量的引导）
pub fn empty_state(ui: &mut egui::Ui, title: &str, desc: &str) {
    ui.add_space(24.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(title).size(14.0).color(theme::text_dim()).strong());
        ui.add_space(4.0);
        ui.label(RichText::new(desc).size(12.0).color(theme::text_dim()));
    });
    ui.add_space(16.0);
}

/// 主操作按钮（琥珀色签发）
pub fn accent_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> Response {
    let text_color = if enabled {
        theme::on_accent()
    } else {
        theme::text_dim()
    };
    let btn = egui::Button::new(RichText::new(text).strong().color(text_color))
        .fill(if enabled {
            theme::accent()
        } else {
            theme::raised()
        })
        .min_size(egui::vec2(100.0, 30.0))
        .rounding(Rounding::same(8.0));
    ui.add_enabled(enabled, btn)
}

/// 次要按钮
pub fn subtle_button(ui: &mut egui::Ui, text: &str) -> Response {
    let btn = egui::Button::new(RichText::new(text).color(theme::text()))
        .fill(theme::raised())
        .min_size(egui::vec2(0.0, 28.0))
        .rounding(Rounding::same(7.0));
    ui.add(btn)
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
    let key_w = 180.0;
    let del_w = 24.0;
    let spacing = 8.0;
    let row_h = 30.0;
    let val_w = (ui.available_width() - check_w - key_w - del_w - spacing * 3.0).max(120.0);

    fn header(ui: &mut egui::Ui, text: &str, w: f32) {
        ui.add_sized(
            [w, 16.0],
            egui::Label::new(RichText::new(text).size(11.0).color(theme::text_dim())),
        );
    }

    egui::Grid::new(format!("{id}_grid"))
        .num_columns(4)
        .spacing([spacing, 6.0])
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
                    TextEdit::singleline(&mut row.key).hint_text("key"),
                );
                ui.add_sized(
                    [val_w, row_h],
                    TextEdit::singleline(&mut row.value).hint_text("value"),
                );
                if ui
                    .add_sized(
                        [del_w, row_h],
                        egui::Button::new(RichText::new("✕").color(theme::text_dim()))
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

    ui.add_space(6.0);
    if subtle_button(ui, "+ 添加一行").clicked() {
        rows.push(KVPair {
            enabled: true,
            ..Default::default()
        });
    }
}
