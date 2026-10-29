//! 请求编辑区：URL 栏 + 配置分页

use eframe::egui::{self, Color32, RichText, Rounding, TextEdit};

use crate::app::{AppState, RequestTab};
use crate::model::request::{AuthType, BodyType, KVPair, Method};
use crate::theme;
use crate::ui::widgets;

/// 整块请求编辑区
pub fn draw(ui: &mut egui::Ui, state: &mut AppState) {
    url_bar(ui, state);
    ui.add_space(10.0);
    tab_bar(ui, &mut state.request_tab);
    ui.add_space(8.0);

    egui::ScrollArea::vertical()
        .id_salt("request_tab_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| match state.request_tab {
            RequestTab::Params => params_tab(ui, state),
            RequestTab::Headers => headers_tab(ui, state),
            RequestTab::Body => body_tab(ui, state),
            RequestTab::Auth => auth_tab(ui, state),
            RequestTab::Advanced => advanced_tab(ui, state),
        });
}

/// URL + 方法 + 发送
fn url_bar(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        // 方法选择（与输入框等高）
        let mc = theme::method_color(state.config.method.as_str());
        let items: Vec<(egui::WidgetText, bool)> = Method::ALL
            .iter()
            .map(|m| {
                (
                    RichText::new(m.as_str())
                        .monospace()
                        .strong()
                        .color(theme::method_color(m.as_str()))
                        .into(),
                    *m == state.config.method,
                )
            })
            .collect();
        let btn = widgets::select_button(
            RichText::new(format!("{}  ▼", state.config.method.as_str()))
                .monospace()
                .strong()
                .color(mc)
                .size(13.0),
            104.0,
        );
        if let Some(i) = widgets::select_popup(ui, "method_select", 128.0, btn, &items) {
            state.config.method = Method::ALL[i];
        }

        // URL 输入
        let send_w = 108.0;
        let url_w = (ui.available_width() - send_w).max(140.0);
        ui.add_sized(
            [url_w, 32.0],
            TextEdit::singleline(&mut state.config.url)
                .hint_text("https://api.example.com/endpoint")
                .font(egui::TextStyle::Monospace)
                .vertical_align(egui::Align::Center),
        );

        // 发送按钮
        let enabled = !state.sending;
        let label = if state.sending { "发送中…" } else { "发 送" };
        if widgets::accent_button(ui, label, enabled).clicked() {
            let ctx = ui.ctx().clone();
            state.send_request(&ctx);
        }
    });

    // 快捷键提示 + 变量提示 + cURL
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        widgets::hint(ui, "Ctrl + Enter 快速发送");
        if !state.current_env.is_empty() {
            ui.add_space(8.0);
            widgets::hint(ui, &format!("· 当前环境：{}", state.current_env));
        }
        let missing = state.undefined_vars();
        if !missing.is_empty() {
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!("· 未定义变量：{}", missing.join("、")))
                    .size(11.5)
                    .color(theme::warn()),
            )
            .on_hover_text("这些变量在当前环境没有值，发送时会原样保留");
        }
        ui.add_space(12.0);
        if widgets::link_button(ui, "导入 cURL", theme::text_dim())
            .on_hover_text("粘贴 curl 命令并解析为请求")
            .clicked()
        {
            state.curl_import_open = true;
        }
        ui.add_space(4.0);
        if widgets::link_button(ui, "复制 cURL", theme::text_dim())
            .on_hover_text("将当前请求复制为 curl 命令")
            .clicked()
        {
            let curl = state.current_curl();
            ui.ctx().output_mut(|o| o.copied_text = curl);
            state.toast_info("已复制 cURL 命令".to_string());
        }
    });
}

/// 分页标签条
fn tab_bar(ui: &mut egui::Ui, active: &mut RequestTab) {
    let tabs = [
        (RequestTab::Params, "Params"),
        (RequestTab::Headers, "Headers"),
        (RequestTab::Body, "Body"),
        (RequestTab::Auth, "Auth"),
        (RequestTab::Advanced, "高级"),
    ];
    ui.horizontal(|ui| {
        for (tab, label) in tabs {
            let selected = *active == tab;
            let text = RichText::new(label)
                .size(13.0)
                .color(if selected { theme::accent() } else { theme::text_dim() })
                .strong();
            let btn = egui::Button::new(text)
                .fill(if selected {
                    theme::accent().gamma_multiply(0.12)
                } else {
                    Color32::TRANSPARENT
                })
                .rounding(Rounding::same(8.0))
                .min_size(egui::vec2(0.0, 30.0));
            if ui.add(btn).clicked() {
                *active = tab;
            }
        }
    });
}

fn params_tab(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        widgets::hint(ui, "查询参数将拼接到 URL 的 ? 之后");
        ui.add_space(10.0);
        if widgets::link_button(ui, "从 URL 导入", theme::accent())
            .on_hover_text("解析 URL 中的查询参数填入下方，并从 URL 中移除")
            .clicked()
        {
            import_params_from_url(state);
        }
    });
    ui.add_space(6.0);
    widgets::kv_editor(ui, "params", &mut state.config.params, "参数名", "值");
}

/// 解析 URL 中的 query，导入到 params 并从 URL 移除
fn import_params_from_url(state: &mut AppState) {
    let Ok(mut parsed) = url::Url::parse(state.config.url.trim()) else {
        state.toast_error("URL 无法解析".to_string());
        return;
    };
    let pairs: Vec<(String, String)> = parsed
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    if pairs.is_empty() {
        state.toast_info("URL 中没有查询参数".to_string());
        return;
    }
    for (k, v) in pairs {
        state.config.params.push(KVPair {
            key: k,
            value: v,
            enabled: true,
        });
    }
    parsed.set_query(None);
    state.config.url = parsed.to_string();
    state.toast_info("已从 URL 导入查询参数".to_string());
}

fn headers_tab(ui: &mut egui::Ui, state: &mut AppState) {
    widgets::hint(ui, "自定义请求头，同名以最后一条为准");
    ui.add_space(6.0);
    widgets::kv_editor(ui, "headers", &mut state.config.headers, "Header", "值");
}

fn body_tab(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        widgets::section_label(ui, "Body 类型");
        let items: Vec<(egui::WidgetText, bool)> = BodyType::ALL
            .iter()
            .map(|bt| {
                (
                    RichText::new(bt.label()).size(13.0).into(),
                    *bt == state.config.body_type,
                )
            })
            .collect();
        let btn = widgets::select_button(
            RichText::new(format!("{}  ▼", state.config.body_type.label())).size(13.0),
            180.0,
        );
        if let Some(i) = widgets::select_popup(ui, "body_type_select", 200.0, btn, &items) {
            state.config.body_type = BodyType::ALL[i];
        }

        if state.config.body_type == BodyType::Json && !state.config.body.trim().is_empty() {
            if widgets::subtle_button(ui, "格式化").clicked() {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&state.config.body) {
                    if let Ok(pretty) = serde_json::to_string_pretty(&v) {
                        state.config.body = pretty;
                    }
                }
            }
        }
    });
    ui.add_space(10.0);

    match state.config.body_type {
        BodyType::None => {
            widgets::empty_state(ui, "无请求体", "选择上方的 Body 类型以编辑请求内容");
        }
        BodyType::Json | BodyType::Text => {
            let hint = if state.config.body_type == BodyType::Json {
                "{\n  \"key\": \"value\"\n}"
            } else {
                "纯文本内容"
            };
            egui::ScrollArea::vertical()
                .id_salt("body_text_scroll")
                .max_height(250.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.add(
                        TextEdit::multiline(&mut state.config.body)
                            .desired_rows(10)
                            .desired_width(f32::INFINITY)
                            .font(egui::TextStyle::Monospace)
                            .hint_text(hint),
                    );
                });
        }
        BodyType::Form => {
            widgets::hint(ui, "application/x-www-form-urlencoded 键值对");
            ui.add_space(6.0);
            widgets::kv_editor(ui, "form", &mut state.config.form_fields, "字段", "值");
        }
        BodyType::Multipart => {
            widgets::hint(ui, "multipart/form-data 表单字段与文件");
            ui.add_space(6.0);
            widgets::kv_editor(ui, "multipart_fields", &mut state.config.form_fields, "字段", "值");
            ui.add_space(8.0);
            widgets::section_label(ui, "文件");
            ui.horizontal(|ui| {
                ui.add(
                    TextEdit::singleline(&mut state.config.file_path)
                        .desired_width(f32::INFINITY)
                        .hint_text("选择要上传的文件")
                        .vertical_align(egui::Align::Center),
                );
                if widgets::subtle_button(ui, "浏览…").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        state.config.file_path = path.display().to_string();
                    }
                }
            });
        }
    }
}

fn auth_tab(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        widgets::section_label(ui, "认证方式");
        let items: Vec<(egui::WidgetText, bool)> = AuthType::ALL
            .iter()
            .map(|at| {
                (
                    RichText::new(at.label()).size(13.0).into(),
                    *at == state.config.auth.auth_type,
                )
            })
            .collect();
        let btn = widgets::select_button(
            RichText::new(format!("{}  ▼", state.config.auth.auth_type.label())).size(13.0),
            200.0,
        );
        if let Some(i) = widgets::select_popup(ui, "auth_type_select", 220.0, btn, &items) {
            state.config.auth.auth_type = AuthType::ALL[i];
        }
    });
    ui.add_space(10.0);

    match state.config.auth.auth_type {
        AuthType::None => {
            widgets::empty_state(ui, "未启用认证", "选择认证方式后将自动附加到请求头");
        }
        AuthType::Basic => {
            egui::Grid::new("auth_basic_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label("用户名");
                    ui.add(
                        TextEdit::singleline(&mut state.config.auth.username)
                            .desired_width(320.0)
                            .vertical_align(egui::Align::Center),
                    );
                    ui.end_row();
                    ui.label("密码");
                    ui.add(
                        TextEdit::singleline(&mut state.config.auth.password)
                            .desired_width(320.0)
                            .password(true)
                            .vertical_align(egui::Align::Center),
                    );
                    ui.end_row();
                });
        }
        AuthType::Bearer => {
            ui.label("Token");
            ui.add(
                TextEdit::singleline(&mut state.config.auth.token)
                    .desired_width(f32::INFINITY)
                    .hint_text("Bearer Token")
                    .vertical_align(egui::Align::Center),
            );
        }
        AuthType::CustomHeader => {
            egui::Grid::new("auth_custom_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Header 名");
                    ui.add(
                        TextEdit::singleline(&mut state.config.auth.header_name)
                            .desired_width(320.0)
                            .hint_text("如 X-API-Key")
                            .vertical_align(egui::Align::Center),
                    );
                    ui.end_row();
                    ui.label("Header 值");
                    ui.add(
                        TextEdit::singleline(&mut state.config.auth.header_value)
                            .desired_width(320.0)
                            .vertical_align(egui::Align::Center),
                    );
                    ui.end_row();
                });
        }
    }
}

fn advanced_tab(ui: &mut egui::Ui, state: &mut AppState) {
    ui.add_space(8.0);
    egui::Grid::new("advanced_grid")
        .num_columns(2)
        .spacing([16.0, 12.0])
        .min_col_width(76.0)
        .show(ui, |ui| {
            ui.label("超时（秒）");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut state.config.timeout_secs).range(0..=600).speed(1));
                widgets::hint(ui, "0 表示默认 30 秒");
            });
            ui.end_row();

            ui.label("证书校验");
            ui.checkbox(&mut state.config.insecure_skip_verify, "跳过 TLS 证书校验（自签名调试）");
            ui.end_row();

            ui.label("重定向");
            ui.checkbox(&mut state.config.follow_redirects, "自动跟随 3xx 重定向");
            ui.end_row();

            ui.label("代理");
            ui.vertical(|ui| {
                ui.checkbox(&mut state.config.use_custom_proxy, "使用自定义代理");
                if state.config.use_custom_proxy {
                    ui.add(
                        TextEdit::singleline(&mut state.config.proxy)
                            .desired_width(360.0)
                            .hint_text("http://127.0.0.1:7890")
                            .vertical_align(egui::Align::Center),
                    );
                } else {
                    widgets::hint(ui, "默认跟随系统代理设置");
                }
            });
            ui.end_row();
        });
}
