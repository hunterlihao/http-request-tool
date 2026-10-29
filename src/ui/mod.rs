//! 顶层 UI 布局

pub mod request_area;
pub mod response;
pub mod sidebar;
pub mod widgets;

use eframe::egui::{self, Align2, Color32, RichText, Rounding, Stroke};

use crate::app::{AppState, ToastKind};
use crate::theme;

/// 绘制整个界面
pub fn draw(ctx: &egui::Context, state: &mut AppState) {
    // 全局快捷键：Ctrl + Enter 发送
    if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter)) {
        state.send_request(ctx);
    }

    top_bar(ctx, state);

    if state.show_sidebar {
        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(320.0)
            .width_range(250.0..=480.0)
            .frame(
                egui::Frame::none()
                    .fill(theme::panel())
                    .inner_margin(egui::Margin::symmetric(16.0, 14.0)),
            )
            .show(ctx, |ui| {
                sidebar::draw(ui, state);
            });
    }

    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(theme::canvas())
                .inner_margin(egui::Margin::same(14.0)),
        )
        .show(ctx, |ui| {
            let total_h = ui.available_height();
            let gap = 12.0;
            let max_req = (total_h - 200.0).max(240.0);
            let req_h = ((total_h - gap) * 0.52).clamp(240.0, max_req);

            // 请求卡片（固定高度）
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), req_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    theme::card_frame().show(ui, |ui| {
                        ui.set_min_height(ui.available_height());
                        request_area::draw(ui, state);
                    });
                },
            );

            ui.add_space(gap);

            // 响应卡片（占满剩余）
            theme::card_frame().show(ui, |ui| {
                ui.set_min_height(ui.available_height());
                response::draw(ui, state);
            });
        });

    save_dialog(ctx, state);
    toast(ctx, state);
}

/// 顶部栏
fn top_bar(ctx: &egui::Context, state: &mut AppState) {
    egui::TopBottomPanel::top("topbar")
        .frame(
            egui::Frame::none()
                .fill(theme::panel())
                .inner_margin(egui::Margin::symmetric(18.0, 12.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // 品牌标记
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, Rounding::same(4.0), theme::accent());

                ui.add_space(8.0);
                ui.label(
                    RichText::new("HTTP 请求工具")
                        .size(15.0)
                        .strong()
                        .color(theme::text()),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new("便携调试台")
                        .size(11.5)
                        .color(theme::text_muted()),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // 侧栏开关
                    let toggle = if state.show_sidebar { "收起侧栏" } else { "展开侧栏" };
                    if widgets::subtle_button(ui, toggle).clicked() {
                        state.show_sidebar = !state.show_sidebar;
                    }

                    ui.add_space(10.0);

                    // 环境快捷选择（等高下拉）
                    let items: Vec<(egui::WidgetText, bool)> = state
                        .env_names
                        .iter()
                        .map(|n| {
                            (
                                RichText::new(n).size(13.0).into(),
                                *n == state.current_env,
                            )
                        })
                        .collect();
                    let btn = widgets::select_button(
                        RichText::new(format!("环境 · {}  ▾", state.current_env)).size(12.5),
                        168.0,
                    );
                    if let Some(i) =
                        widgets::select_popup(ui, "topbar_env_select", 168.0, btn, &items)
                    {
                        let name = state.env_names[i].clone();
                        state.load_env(&name);
                    }
                });
            });
        });
}

/// 保存请求对话框
fn save_dialog(ctx: &egui::Context, state: &mut AppState) {
    if !state.save_dialog_open {
        return;
    }
    let mut open = state.save_dialog_open;
    egui::Window::new("保存请求到集合")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(380.0);
            widgets::hint(ui, "为这个请求起个名字，便于以后在集合中找到它");
            ui.add_space(8.0);
            let default_name = if state.config.url.is_empty() {
                "未命名请求".to_string()
            } else {
                state.config.url.clone()
            };
            let entry = ui.add_sized(
                [ui.available_width(), 32.0],
                egui::TextEdit::singleline(&mut state.save_name)
                    .hint_text(&default_name)
                    .vertical_align(egui::Align::Center),
            );

            ui.add_space(14.0);
            ui.horizontal(|ui| {
                if widgets::accent_button(ui, "保 存", true).clicked() {
                    let name = if state.save_name.trim().is_empty() {
                        default_name.clone()
                    } else {
                        state.save_name.trim().to_string()
                    };
                    state.save_current_to_collection(&name);
                }
                if widgets::subtle_button(ui, "取消").clicked() {
                    state.save_dialog_open = false;
                    state.save_name.clear();
                }
            });

            // 回车保存
            if entry.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let name = if state.save_name.trim().is_empty() {
                    default_name.clone()
                } else {
                    state.save_name.trim().to_string()
                };
                state.save_current_to_collection(&name);
            }
        });
    if !open {
        state.save_dialog_open = false;
    }
}

/// 轻提示
fn toast(ctx: &egui::Context, state: &AppState) {
    let Some(t) = &state.toast else { return };
    let color = match t.kind {
        ToastKind::Info => theme::ok(),
        ToastKind::Error => theme::error(),
    };
    egui::Area::new(egui::Id::new("toast_area"))
        .anchor(Align2::CENTER_BOTTOM, [0.0, -28.0])
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(theme::panel())
                .rounding(Rounding::same(10.0))
                .stroke(Stroke::new(1.0, theme::border()))
                .inner_margin(egui::Margin::symmetric(18.0, 10.0))
                .shadow(egui::epaint::Shadow {
                    offset: egui::vec2(0.0, 6.0),
                    blur: 20.0,
                    spread: 0.0,
                    color: Color32::from_black_alpha(30),
                })
                .show(ui, |ui| {
                    ui.label(RichText::new(&t.text).color(color).size(13.0).strong());
                });
        });
}
