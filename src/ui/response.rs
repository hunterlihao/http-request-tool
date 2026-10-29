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
        widgets::badge(ui, &format!("{} ms", resp.duration_ms), theme::text_dim());
        widgets::badge(ui, &human_size(resp.body.len()), theme::text_dim());
        if !resp.http_version.is_empty() {
            widgets::badge(ui, &resp.http_version, theme::redirect());
        }

        widgets::right_aligned(ui, 34.0, |ui| {
            if widgets::subtle_button(ui, "复制").clicked() {
                let text = if state.body_raw_view {
                    resp.body_text()
                } else {
                    resp.formatted_body()
                };
                ui.ctx().output_mut(|o| o.copied_text = text);
                state.toast_info("已复制响应体".to_string());
            }
            if widgets::subtle_button(ui, "下载").clicked() {
                download_body(state, resp);
            }

            // Body / Headers 切换
            let body_sel = state.response_tab == ResponseTab::Body;
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("Body")
                            .size(13.0)
                            .color(if body_sel { theme::accent() } else { theme::text_dim() }),
                    )
                    .fill(if body_sel {
                        theme::accent().gamma_multiply(0.12)
                    } else {
                        Color32::TRANSPARENT
                    })
                    .rounding(Rounding::same(8.0))
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
                            .size(13.0)
                            .color(if hdr_sel { theme::accent() } else { theme::text_dim() }),
                    )
                    .fill(if hdr_sel {
                        theme::accent().gamma_multiply(0.12)
                    } else {
                        Color32::TRANSPARENT
                    })
                    .rounding(Rounding::same(8.0))
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

/// 响应体展示上限（超过则截断，避免文本渲染卡顿）
const MAX_DISPLAY: usize = 1024 * 1024;

fn body_view(ui: &mut egui::Ui, state: &mut AppState, resp: &ResponseData) {
    let truncated = resp.body.len() > MAX_DISPLAY;

    // 原始 / 格式化切换（截断时不格式化）
    let is_json = !truncated && serde_json::from_str::<serde_json::Value>(&resp.body_text()).is_ok();
    if is_json {
        ui.horizontal(|ui| {
            let fmt = !state.body_raw_view;
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("格式化").size(13.0).color(if fmt { theme::accent() } else { theme::text_dim() }),
                    )
                    .fill(if fmt { theme::accent().gamma_multiply(0.12) } else { Color32::TRANSPARENT })
                    .rounding(Rounding::same(8.0))
                    .frame(false),
                )
                .clicked()
            {
                state.body_raw_view = false;
            }
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("原始").size(13.0).color(if !fmt { theme::accent() } else { theme::text_dim() }),
                    )
                    .fill(if !fmt { theme::accent().gamma_multiply(0.12) } else { Color32::TRANSPARENT })
                    .rounding(Rounding::same(8.0))
                    .frame(false),
                )
                .clicked()
            {
                state.body_raw_view = true;
            }
        });
        ui.add_space(4.0);
    }

    if truncated {
        ui.horizontal(|ui| {
            widgets::badge(ui, "内容过大", theme::warn());
            widgets::hint(
                ui,
                &format!(
                    "共 {}，仅显示前 {}；完整内容请点「下载」",
                    human_size(resp.body.len()),
                    human_size(MAX_DISPLAY)
                ),
            );
        });
        ui.add_space(4.0);
    }

    let text = if truncated {
        let mut end = MAX_DISPLAY.min(resp.body.len());
        // 回退到 UTF-8 字符边界，避免截断后出现乱码
        while end > 0 && end < resp.body.len() && (resp.body[end] & 0xC0) == 0x80 {
            end -= 1;
        }
        String::from_utf8_lossy(&resp.body[..end]).to_string()
    } else if state.body_raw_view {
        resp.body_text()
    } else {
        resp.formatted_body()
    };

    let use_hl = !truncated && !state.body_raw_view && is_json && text.len() < 200_000;

    egui::ScrollArea::both()
        .id_salt("resp_body_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if text.trim().is_empty() {
                widgets::hint(ui, "（空响应体）");
            } else if use_hl {
                ui.add(egui::Label::new(crate::ui::json_hl::json_job(&text)));
            } else {
                let mut display = text;
                ui.add(
                    egui::TextEdit::multiline(&mut display)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .interactive(false),
                );
            }
        });
}

/// 把响应体保存到文件
fn download_body(state: &mut AppState, resp: &ResponseData) {
    let name = guess_filename(resp);
    let Some(path) = rfd::FileDialog::new().set_file_name(&name).save_file() else {
        return;
    };
    match std::fs::write(&path, &resp.body) {
        Ok(_) => state.toast_info(format!("已保存到 {}", path.display())),
        Err(e) => state.toast_error(format!("保存失败：{e}")),
    }
}

/// 依据 Content-Type 推断下载文件名后缀
fn guess_filename(resp: &ResponseData) -> String {
    let ct = resp
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.to_lowercase())
        .unwrap_or_default();
    let ext = if ct.contains("json") {
        "json"
    } else if ct.contains("html") {
        "html"
    } else if ct.contains("xml") {
        "xml"
    } else if ct.contains("javascript") {
        "js"
    } else if ct.contains("image/png") {
        "png"
    } else if ct.contains("image/jpeg") {
        "jpg"
    } else if ct.contains("image/svg") {
        "svg"
    } else if ct.contains("pdf") {
        "pdf"
    } else {
        "txt"
    };
    format!("response.{ext}")
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
                                .size(13.0)
                                .color(theme::accent()),
                        );
                        ui.label(RichText::new(v).monospace().size(13.0).color(theme::text()));
                        ui.end_row();
                    }
                });
        });
}

fn error_view(ui: &mut egui::Ui, err: &str, duration_ms: u128) {
    ui.horizontal(|ui| {
        widgets::badge(ui, "请求失败", theme::error());
        widgets::badge(ui, &format!("{} ms", duration_ms), theme::text_dim());
    });
    ui.add_space(10.0);
    egui::Frame::none()
        .fill(theme::error().gamma_multiply(0.10))
        .rounding(Rounding::same(8.0))
        .inner_margin(egui::Margin::same(12.0))
        .show(ui, |ui| {
            ui.label(RichText::new(err).color(theme::error()).size(13.0));
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
