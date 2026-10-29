//! 响应展示区

use eframe::egui::{self, Color32, RichText, Rounding};

use crate::app::{AppState, ResponseTab};
use crate::model::request::ResponseData;
use crate::theme;
use crate::ui::widgets;

pub fn draw(ui: &mut egui::Ui, state: &mut AppState) {
    match state.response.clone() {
        None => {
            widgets::empty_state(
                ui,
                "尚无响应",
                "填写 URL 后点击「发送」，响应结果将显示在这里",
            );
        }
        Some(resp) => {
            if let Some(err) = &resp.error {
                error_view(ui, err, resp.duration_ms);
            } else {
                status_bar(ui, state, &resp);
            }
        }
    }
}

fn status_bar(ui: &mut egui::Ui, state: &mut AppState, resp: &ResponseData) {
    ui.horizontal(|ui| {
        let sc = theme::status_color(resp.status_code);
        widgets::badge(ui, &format!("{} {}", resp.status_code, resp.status_text), sc);
        widgets::badge(ui, &format!("{} ms", resp.duration_ms), theme::TEXT_DIM);
        widgets::badge(ui, &human_size(resp.body.len()), theme::TEXT_DIM);
        if !resp.http_version.is_empty() {
            widgets::badge(ui, &resp.http_version, theme::REDIRECT);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if widgets::subtle_button(ui, "复制").clicked() {
                let text = if state.body_raw_view {
                    resp.body_text()
                } else {
                    resp.formatted_body()
                };
                ui.ctx().output_mut(|o| o.copied_text = text);
                state.toast_info("已复制响应体".to_string());
            }

            // Body / Headers 切换
            let body_sel = state.response_tab == ResponseTab::Body;
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("Body")
                            .size(12.0)
                            .color(if body_sel { theme::ACCENT } else { theme::TEXT_DIM }),
                    )
                    .fill(if body_sel {
                        theme::ACCENT.gamma_multiply(0.12)
                    } else {
                        Color32::TRANSPARENT
                    })
                    .rounding(Rounding::same(6.0))
                    .frame(false),
                )
                .clicked()
            {
                state.response_tab = ResponseTab::Body;
            }
            let hdr_sel = state.response_tab == ResponseTab::Headers;
            if ui
                .add(
                    egui::Button::new(
                        RichText::new(format!("Headers ({})", resp.headers.len()))
                            .size(12.0)
                            .color(if hdr_sel { theme::ACCENT } else { theme::TEXT_DIM }),
                    )
                    .fill(if hdr_sel {
                        theme::ACCENT.gamma_multiply(0.12)
                    } else {
                        Color32::TRANSPARENT
                    })
                    .rounding(Rounding::same(6.0))
                    .frame(false),
                )
                .clicked()
            {
                state.response_tab = ResponseTab::Headers;
            }
        });
    });

    ui.add_space(6.0);

    match state.response_tab {
        ResponseTab::Body => body_view(ui, state, resp),
        ResponseTab::Headers => headers_view(ui, resp),
    }
}

fn body_view(ui: &mut egui::Ui, state: &mut AppState, resp: &ResponseData) {
    // 原始 / 格式化切换
    let is_json = serde_json::from_str::<serde_json::Value>(&resp.body_text()).is_ok();
    if is_json {
        ui.horizontal(|ui| {
            let fmt = !state.body_raw_view;
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("格式化").size(12.0).color(if fmt { theme::ACCENT } else { theme::TEXT_DIM }),
                    )
                    .fill(if fmt { theme::ACCENT.gamma_multiply(0.12) } else { Color32::TRANSPARENT })
                    .rounding(Rounding::same(6.0))
                    .frame(false),
                )
                .clicked()
            {
                state.body_raw_view = false;
            }
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("原始").size(12.0).color(if !fmt { theme::ACCENT } else { theme::TEXT_DIM }),
                    )
                    .fill(if !fmt { theme::ACCENT.gamma_multiply(0.12) } else { Color32::TRANSPARENT })
                    .rounding(Rounding::same(6.0))
                    .frame(false),
                )
                .clicked()
            {
                state.body_raw_view = true;
            }
        });
        ui.add_space(4.0);
    }

    let text = if state.body_raw_view {
        resp.body_text()
    } else {
        resp.formatted_body()
    };

    egui::ScrollArea::both()
        .id_salt("resp_body_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if text.trim().is_empty() {
                widgets::hint(ui, "（空响应体）");
            } else {
                // 只读的多行文本，等宽字体
                let mut display = text;
                ui.add(
                    egui::TextEdit::multiline(&mut display)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .interactive(false)
                        .frame(false),
                );
            }
        });
}

fn headers_view(ui: &mut egui::Ui, resp: &ResponseData) {
    egui::ScrollArea::vertical()
        .id_salt("resp_headers_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("resp_headers_grid")
                .num_columns(2)
                .spacing([16.0, 6.0])
                .striped(true)
                .show(ui, |ui| {
                    for (k, v) in &resp.headers {
                        ui.label(
                            RichText::new(k)
                                .monospace()
                                .size(12.0)
                                .color(theme::ACCENT),
                        );
                        ui.label(RichText::new(v).monospace().size(12.0).color(theme::TEXT));
                        ui.end_row();
                    }
                });
        });
}

fn error_view(ui: &mut egui::Ui, err: &str, duration_ms: u128) {
    ui.horizontal(|ui| {
        widgets::badge(ui, "请求失败", theme::ERROR);
        widgets::badge(ui, &format!("{} ms", duration_ms), theme::TEXT_DIM);
    });
    ui.add_space(10.0);
    egui::Frame::none()
        .fill(theme::ERROR.gamma_multiply(0.10))
        .rounding(Rounding::same(8.0))
        .inner_margin(egui::Margin::same(12.0))
        .show(ui, |ui| {
            ui.label(RichText::new(err).color(theme::ERROR).size(13.0));
        });
}

/// 人类可读体积
fn human_size(bytes: usize) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{:.1} {}", size, UNITS[unit])
    }
}
