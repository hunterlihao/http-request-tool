//! 左侧栏：历史 / 集合 / 环境变量

use eframe::egui::{self, Color32, RichText, Rounding, TextEdit};

use crate::app::{AppState, SideTab};
use crate::model::request::RequestConfig;
use crate::theme;
use crate::ui::widgets;

pub fn draw(ui: &mut egui::Ui, state: &mut AppState) {
    // 让侧栏内容铺满面板宽度，避免右侧露出未填充底色
    ui.set_min_width(ui.available_width());
    side_tab_bar(ui, &mut state.side_tab);
    ui.add_space(8.0);
    match state.side_tab {
        SideTab::History => history_panel(ui, state),
        SideTab::Collections => collections_panel(ui, state),
        SideTab::Environments => env_panel(ui, state),
    }
}

fn side_tab_bar(ui: &mut egui::Ui, active: &mut SideTab) {
    let tabs = [
        (SideTab::History, "历史"),
        (SideTab::Collections, "集合"),
        (SideTab::Environments, "环境"),
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

// ---------- 历史 ----------

fn history_panel(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.add(
            TextEdit::singleline(&mut state.history_search)
                .hint_text("搜索 URL 或方法")
                .desired_width(f32::INFINITY)
                .vertical_align(egui::Align::Center),
        );
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        widgets::hint(ui, &format!("共 {} 条", state.history.len()));
        widgets::right_aligned(ui, 32.0, |ui| {
            if widgets::subtle_button(ui, "清空").clicked() {
                if state.db.clear_history().is_ok() {
                    state.reload_history();
                    state.toast_info("已清空历史".to_string());
                }
            }
        });
    });
    ui.add_space(4.0);

    let search = state.history_search.to_lowercase();
    // 按 方法+URL 去重，保留最新一条
    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    let filtered: Vec<_> = state
        .history
        .iter()
        .filter(|h| {
            search.is_empty()
                || h.config.url.to_lowercase().contains(&search)
                || h.config.method.as_str().to_lowercase().contains(&search)
        })
        .filter(|h| {
            seen.insert((h.config.method.as_str().to_string(), h.config.url.clone()))
        })
        .cloned()
        .collect();

    if filtered.is_empty() {
        widgets::empty_state(ui, "暂无历史", "发送请求后会自动记录在这里");
        return;
    }

    let mut to_apply: Option<RequestConfig> = None;
    let mut to_delete: Option<i64> = None;

    egui::ScrollArea::vertical()
        .id_salt("history_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for item in &filtered {
                let row = ui.horizontal(|ui| {
                    ui.set_min_height(26.0);
                    let mc = theme::method_color(item.config.method.as_str());
                    ui.add_sized(
                        [46.0, 16.0],
                        egui::Label::new(
                            RichText::new(item.config.method.as_str())
                                .size(11.5)
                                .monospace()
                                .color(mc)
                                .strong(),
                        ),
                    );
                    let label = ui.add(
                        egui::Label::new(
                            RichText::new(truncate(&item.config.url, 34))
                                .size(13.0)
                                .color(theme::text()),
                        )
                        .sense(egui::Sense::click())
                        .truncate(),
                    );
                    if let Some(code) = item.status_code {
                        ui.label(
                            RichText::new(code.to_string())
                                .size(11.5)
                                .monospace()
                                .color(theme::status_color(code)),
                        );
                    }
                    if label.clicked() {
                        to_apply = Some(item.config.clone());
                    }
                    false
                });
                row.response.context_menu(|ui| {
                    if ui.button("载入此请求").clicked() {
                        to_apply = Some(item.config.clone());
                        ui.close_menu();
                    }
                    if ui.button("删除").clicked() {
                        to_delete = Some(item.id);
                        ui.close_menu();
                    }
                });
                ui.separator();
            }
        });

    if let Some(cfg) = to_apply {
        state.apply_config(cfg);
    }
    if let Some(id) = to_delete {
        let _ = state.db.delete_history(id);
        state.reload_history();
    }
}

// ---------- 集合 ----------

fn collections_panel(ui: &mut egui::Ui, state: &mut AppState) {
    // 集合选择
    ui.horizontal(|ui| {
        let current = state
            .collections
            .iter()
            .find(|c| Some(c.id) == state.selected_collection)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "（无集合）".to_string());
        let width = (ui.available_width() - 34.0).max(120.0);
        let items: Vec<(egui::WidgetText, bool)> = state
            .collections
            .iter()
            .map(|c| {
                (
                    RichText::new(&c.name).size(13.0).into(),
                    Some(c.id) == state.selected_collection,
                )
            })
            .collect();
        let btn = widgets::select_button(
            RichText::new(format!("{}  ▼", truncate(&current, 16))).size(13.0),
            width,
        );
        if let Some(i) = widgets::select_popup(ui, "collection_select", width, btn, &items) {
            let id = state.collections[i].id;
            state.selected_collection = Some(id);
            state.reload_saved_requests();
        }
        // 删除当前集合
        if ui
            .add(
                egui::Button::new(RichText::new("✕").size(13.0).color(theme::error()))
                    .fill(Color32::TRANSPARENT)
                    .frame(false),
            )
            .on_hover_text("删除当前集合")
            .clicked()
        {
            if let Some(cid) = state.selected_collection {
                match state.db.delete_collection(cid) {
                    Ok(_) => {
                        state.selected_collection = None;
                        state.reload_collections();
                        state.toast_info("已删除集合".to_string());
                    }
                    Err(e) => state.toast_error(format!("删除失败：{e}")),
                }
            }
        }
    });

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add(
            TextEdit::singleline(&mut state.new_collection_name)
                .hint_text("新集合名称")
                .desired_width(ui.available_width() - 76.0)
                .vertical_align(egui::Align::Center),
        );
        if widgets::subtle_button(ui, "新建").clicked() {
            let name = state.new_collection_name.clone();
            state.create_collection(&name);
        }
    });

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if widgets::subtle_button(ui, "保存当前请求").clicked() {
            state.save_dialog_open = true;
        }
        if widgets::subtle_button(ui, "导入").clicked() {
            import_collection(state);
        }
        if widgets::subtle_button(ui, "导出").clicked() {
            export_collection(state);
        }
    });

    ui.add_space(8.0);
    widgets::section_label(ui, &format!("已保存请求（{}）", state.saved_requests.len()));
    ui.add_space(2.0);

    if state.saved_requests.is_empty() {
        widgets::empty_state(ui, "集合为空", "点击「保存当前请求」把当前配置存入此集合");
        return;
    }

    let mut to_apply: Option<RequestConfig> = None;
    let mut to_delete: Option<i64> = None;

    egui::ScrollArea::vertical()
        .id_salt("saved_req_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for req in &state.saved_requests {
                ui.horizontal(|ui| {
                    ui.set_min_height(24.0);
                    let mc = theme::method_color(req.config.method.as_str());
                    ui.add_sized(
                        [46.0, 16.0],
                        egui::Label::new(
                            RichText::new(req.config.method.as_str())
                                .size(11.5)
                                .monospace()
                                .color(mc)
                                .strong(),
                        ),
                    );
                    let label = ui.add(
                        egui::Label::new(
                            RichText::new(truncate(&req.name, 30)).size(13.0).color(theme::text()),
                        )
                        .sense(egui::Sense::click())
                        .truncate(),
                    );
                    if label.clicked() {
                        to_apply = Some(req.config.clone());
                    }
                    widgets::right_aligned(ui, 24.0, |ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("✕").size(11.5).color(theme::text_dim()))
                                    .fill(Color32::TRANSPARENT)
                                    .frame(false),
                            )
                            .clicked()
                        {
                            to_delete = Some(req.id);
                        }
                    });
                });
            }
        });

    if let Some(cfg) = to_apply {
        state.apply_config(cfg);
        state.toast_info("已载入请求".to_string());
    }
    if let Some(id) = to_delete {
        let _ = state.db.delete_saved_request(id);
        state.reload_saved_requests();
    }
}

fn import_collection(state: &mut AppState) {
    let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() else {
        return;
    };
    match std::fs::read_to_string(&path) {
        Ok(json) => {
            match state.db.import_collection(&json) {
                Ok(id) => {
                    state.selected_collection = Some(id);
                    state.reload_collections();
                    state.toast_info("导入成功".to_string());
                }
                Err(e) => state.toast_error(format!("导入失败：{e}")),
            }
        }
        Err(e) => state.toast_error(format!("读取文件失败：{e}")),
    }
}

fn export_collection(state: &mut AppState) {
    let Some(cid) = state.selected_collection else {
        state.toast_error("请先选择集合".to_string());
        return;
    };
    let json = match state.db.export_collection(cid) {
        Ok(j) => j,
        Err(e) => {
            state.toast_error(format!("导出失败：{e}"));
            return;
        }
    };
    let Some(path) = rfd::FileDialog::new()
        .add_filter("JSON", &["json"])
        .set_file_name("collection.json")
        .save_file()
    else {
        return;
    };
    match std::fs::write(&path, json) {
        Ok(_) => state.toast_info("导出成功".to_string()),
        Err(e) => state.toast_error(format!("写入失败：{e}")),
    }
}

// ---------- 环境变量 ----------

fn env_panel(ui: &mut egui::Ui, state: &mut AppState) {
    // 环境选择
    ui.horizontal(|ui| {
        let width = (ui.available_width() - 34.0).max(120.0);
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
            RichText::new(format!("{}  ▼", truncate(&state.current_env, 16))).size(13.0),
            width,
        );
        if let Some(i) = widgets::select_popup(ui, "env_select", width, btn, &items) {
            let name = state.env_names[i].clone();
            state.load_env(&name);
        }
        // 删除当前环境（global 不可删）
        if state.current_env != "global"
            && ui
                .add(
                    egui::Button::new(RichText::new("✕").size(13.0).color(theme::error()))
                        .fill(Color32::TRANSPARENT)
                        .frame(false),
                )
                .on_hover_text("删除当前环境")
                .clicked()
        {
            let name = state.current_env.clone();
            match state.db.drop_env(&name) {
                Ok(_) => {
                    state.load_env("global");
                    state.reload_envs();
                    state.toast_info(format!("已删除环境「{name}」"));
                }
                Err(e) => state.toast_error(format!("删除失败：{e}")),
            }
        }
    });

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add(
            TextEdit::singleline(&mut state.new_env_name)
                .hint_text("新环境名")
                .desired_width(ui.available_width() - 76.0)
                .vertical_align(egui::Align::Center),
        );
        if widgets::subtle_button(ui, "新建").clicked() {
            let name = state.new_env_name.trim().to_string();
            if name.is_empty() {
                state.toast_error("环境名不能为空".to_string());
            } else if state.env_names.contains(&name) {
                state.toast_error("环境已存在".to_string());
            } else {
                state.env_names.push(name.clone());
                state.new_env_name.clear();
                state.load_env(&name);
                state.toast_info(format!("已创建环境「{name}」"));
            }
        }
    });

    ui.add_space(8.0);
    widgets::hint(ui, "在 URL / Header / Body 中用 {{变量名}} 引用");
    ui.add_space(6.0);

    // 变量编辑（直接编辑，变更即持久化）
    let before = state.env_vars.clone();
    let mut remove: Option<usize> = None;

    let key_w = 110.0;
    let del_w = 24.0;
    let spacing = 8.0;
    let row_h = 30.0;

    egui::ScrollArea::vertical()
        .id_salt("env_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let val_w = (ui.available_width() - key_w - del_w - spacing * 2.0).max(100.0);
            egui::Grid::new("env_grid")
                .num_columns(3)
                .spacing([spacing, 6.0])
                .show(ui, |ui| {
                    ui.add_sized(
                        [key_w, 16.0],
                        egui::Label::new(RichText::new("变量").size(11.5).color(theme::text_dim())),
                    );
                    ui.add_sized(
                        [val_w, 16.0],
                        egui::Label::new(RichText::new("值").size(11.5).color(theme::text_dim())),
                    );
                    ui.label("");
                    ui.end_row();

                    for (i, (k, v)) in state.env_vars.iter_mut().enumerate() {
                        ui.add_sized(
                            [key_w, row_h],
                            TextEdit::singleline(k)
                                .hint_text("key")
                                .font(egui::TextStyle::Monospace)
                                .vertical_align(egui::Align::Center),
                        );
                        ui.add_sized(
                            [val_w, row_h],
                            TextEdit::singleline(v)
                                .hint_text("value")
                                .font(egui::TextStyle::Monospace)
                                .vertical_align(egui::Align::Center),
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
        });

    if let Some(i) = remove {
        let (k, _) = state.env_vars[i].clone();
        state.env_vars.remove(i);
        if !k.trim().is_empty() {
            let _ = state.db.delete_env_var(&state.current_env, k.trim());
        }
        state.rebuild_env_map();
    }

    ui.add_space(6.0);
    if widgets::subtle_button(ui, "+ 添加变量").clicked() {
        state.env_vars.push((String::new(), String::new()));
    }

    // 编辑后自动持久化
    if state.env_vars != before {
        state.rebuild_env_map();
    }
}

/// 截断长文本（带省略号）
fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{truncated}…")
    }
}
