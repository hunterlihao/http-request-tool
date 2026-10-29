//! 顶层 UI 布局

pub mod request_area;
pub mod response;
pub mod sidebar;
pub mod widgets;

use eframe::egui::{self, Align2, Color32, RichText, Rounding};

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
            .default_width(310.0)
            .width_range(240.0..=460.0)
            .frame(
                egui::Frame::none()
                    .fill(theme::panel())
                    .inner_margin(egui::Margin::symmetric(12.0, 10.0)),
            )
            .show(ctx, |ui| {
                sidebar::draw(ui, state);
            });
    }

    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(theme::canvas())
                .inner_margin(egui::Margin::symmetric(16.0, 14.0)),
        )
        .show(ctx, |ui| {
            let total_h = ui.available_height();
            let req_h = (total_h * 0.52).clamp(260.0, total_h - 180.0);

            // 请求编辑区（固定高度）
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), req_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    request_area::draw(ui, state);
                },
            );

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            // 响应区（占剩余空间）
            response::draw(ui, state);
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
                .inner_margin(egui::Margin::symmetric(16.0, 10.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // logo 标记
                let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, Rounding::same(3.0), theme::accent());

                ui.add_space(4.0);
                ui.label(
                    RichText::new("HTTP 请求工具")
                        .size(15.0)
                        .strong()
                        .color(theme::text()),
                );
                widgets::hint(ui, "· 便携版");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // 侧栏开关
                    let toggle = if state.show_sidebar { "◀ 收起" } else { "▶ 侧栏" };
                    if widgets::subtle_button(ui, toggle).clicked() {
                        state.show_sidebar = !state.show_sidebar;
                    }

                    ui.add_space(8.0);

                    // 主题切换
                    let theme_label = if state.light { "深色模式" } else { "浅色模式" };
                    if widgets::subtle_button(ui, theme_label)
                        .on_hover_text("切换明暗主题")
                        .clicked()
                    {
                        state.light = !state.light;
                        theme::set_mode(ctx, state.light);
                    }

                    ui.add_space(8.0);

                    // 环境快捷选择
                    egui::ComboBox::from_id_salt("topbar_env")
                        .width(150.0)
                        .selected_text(
                            RichText::new(format!("环境：{}", state.current_env))
                                .size(12.0)
                                .color(theme::text()),
                        )
                        .show_ui(ui, |ui| {
                            let names: Vec<_> = state.env_names.clone();
                            for n in names {
                                if ui
                                    .selectable_label(state.current_env == n, &n)
                                    .clicked()
                                {
                                    state.load_env(&n);
                                }
                            }
                        });
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
            ui.set_min_width(360.0);
            widgets::hint(ui, "为这个请求起个名字，便于以后在集合中找到它");
            ui.add_space(6.0);
            let default_name = if state.config.url.is_empty() {
                "未命名请求".to_string()
            } else {
                state.config.url.clone()
            };
            let entry = ui.add(
                egui::TextEdit::singleline(&mut state.save_name)
                    .hint_text(&default_name)
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(10.0);
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
                .fill(theme::raised())
                .rounding(Rounding::same(8.0))
                .stroke(egui::Stroke::new(1.0, theme::border()))
                .inner_margin(egui::Margin::symmetric(16.0, 10.0))
                .shadow(egui::epaint::Shadow {
                    offset: egui::vec2(0.0, 4.0),
                    blur: 16.0,
                    spread: 0.0,
                    color: Color32::from_black_alpha(120),
                })
                .show(ui, |ui| {
                    ui.label(RichText::new(&t.text).color(color).size(13.0));
                });
        });
}
