//! Tauri 命令层：复用原有业务逻辑（client / db / curl / vars），向前端暴露接口

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::State;
use tokio::sync::Notify;

use crate::http::{client, curl};
use crate::model::request::{RequestConfig, ResponseData};
use crate::model::saved::{Collection, EnvVar, HistoryItem, SavedRequest};
use crate::storage::db::Db;
use crate::util::vars;

/// 应用共享状态
pub struct AppState {
    pub db: Mutex<Db>,
    /// 最近一次响应（保留完整字节，供「下载」使用）
    pub last_response: Mutex<Option<ResponseData>>,
    /// 正在进行中的请求的取消信号
    pub active_cancel: Mutex<Option<Arc<Notify>>>,
}

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

/// 响应体展示上限（超出部分不返回给界面，避免前端卡死）
const MAX_BODY: usize = 1024 * 1024;

/// 历史最多保留的条数
const HISTORY_LIMIT: i64 = 500;

/// 返回给前端的响应视图
#[derive(Serialize)]
pub struct ResponseView {
    pub status_code: u16,
    pub status_text: String,
    pub http_version: String,
    pub headers: Vec<(String, String)>,
    /// 实际发出的 URL（变量替换后），便于排查
    pub request_url: String,
    /// 原文（超过上限时截断）
    pub body_text: String,
    /// JSON 美化后的文本（无法解析时与原文一致）
    pub body_pretty: String,
    pub body_len: usize,
    pub truncated: bool,
    /// 二进制响应（不做文本渲染，只提示大小并允许下载）
    pub is_binary: bool,
    pub duration_ms: u64,
    pub content_type: String,
    pub error: Option<String>,
}

/// 应用环境变量替换
fn resolve_config(config: &RequestConfig, env: &HashMap<String, String>) -> RequestConfig {
    let mut cfg = config.clone();
    cfg.url = vars::replace_vars(&cfg.url, env);
    for p in &mut cfg.params {
        p.key = vars::replace_vars(&p.key, env);
        p.value = vars::replace_vars(&p.value, env);
    }
    for h in &mut cfg.headers {
        h.key = vars::replace_vars(&h.key, env);
        h.value = vars::replace_vars(&h.value, env);
    }
    for f in &mut cfg.form_fields {
        f.key = vars::replace_vars(&f.key, env);
        f.value = vars::replace_vars(&f.value, env);
    }
    cfg.body = vars::replace_vars(&cfg.body, env);
    cfg.auth.username = vars::replace_vars(&cfg.auth.username, env);
    cfg.auth.password = vars::replace_vars(&cfg.auth.password, env);
    cfg.auth.token = vars::replace_vars(&cfg.auth.token, env);
    cfg.auth.header_value = vars::replace_vars(&cfg.auth.header_value, env);
    cfg
}

fn to_view(resp: &ResponseData, request_url: String) -> ResponseView {
    let truncated = resp.body.len() > MAX_BODY;
    let mut end = MAX_BODY.min(resp.body.len());
    while end > 0 && end < resp.body.len() && (resp.body[end] & 0xC0) == 0x80 {
        end -= 1;
    }
    // 二进制内容不转文本，避免整屏乱码
    let (text, pretty) = if resp.is_binary {
        (String::new(), String::new())
    } else {
        let text = String::from_utf8_lossy(&resp.body[..end]).to_string();
        let pretty = if truncated {
            text.clone()
        } else {
            resp.formatted_body()
        };
        (text, pretty)
    };
    let content_type = resp
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.clone())
        .unwrap_or_default();

    ResponseView {
        status_code: resp.status_code,
        status_text: resp.status_text.clone(),
        http_version: resp.http_version.clone(),
        headers: resp.headers.clone(),
        request_url,
        body_text: text,
        body_pretty: pretty,
        body_len: resp.body.len(),
        truncated: truncated || resp.body_truncated,
        is_binary: resp.is_binary,
        duration_ms: resp.duration_ms as u64,
        content_type,
        error: resp.error.clone(),
    }
}

/// 依据 Content-Type 推断下载文件名后缀
fn guess_filename(content_type: &str) -> String {
    let ct = content_type.to_lowercase();
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

// ---------- 请求 ----------

/// 去掉认证信息后的副本：历史只记录请求内容，不记录 token / 密码
fn without_secrets(config: &RequestConfig) -> RequestConfig {
    let mut cfg = config.clone();
    cfg.auth = Default::default();
    cfg
}

/// 发送请求：变量替换 → 发送 → 写历史 → 记住当前请求
#[tauri::command]
pub async fn send_request(
    state: State<'_, AppState>,
    config: RequestConfig,
    env_vars: HashMap<String, String>,
) -> Result<ResponseView, String> {
    let resolved = resolve_config(&config, &env_vars);
    let request_url = client::build_url(&resolved);

    let cancel = Arc::new(Notify::new());
    {
        *state.active_cancel.lock().map_err(err)? = Some(cancel.clone());
    }
    let resp = client::send(&resolved, &cancel).await;
    {
        *state.active_cancel.lock().map_err(err)? = None;
    }

    {
        let db = state.db.lock().map_err(err)?;
        // 当前编辑态保留认证信息，下次打开可继续用
        if let Ok(json) = serde_json::to_string(&config) {
            let _ = db.set_setting("last_config", &json);
        }
        // 主动取消不算一次真实请求，不写历史
        if resp.error.as_deref() != Some(client::CANCELLED) {
            let status = if resp.error.is_none() {
                Some(resp.status_code)
            } else {
                None
            };
            let _ = db.add_history(&without_secrets(&config), status, Some(resp.duration_ms));
            // 历史只保留最近若干条
            let _ = db.trim_history(HISTORY_LIMIT);
        }
    }

    let view = to_view(&resp, request_url);
    *state.last_response.lock().map_err(err)? = Some(resp);
    Ok(view)
}

/// 取消正在进行的请求（放弃未完成的 future，连接随之关闭）
#[tauri::command]
pub fn cancel_request(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(cancel) = state.active_cancel.lock().map_err(err)?.take() {
        cancel.notify_one();
    }
    Ok(())
}

/// 当前请求中引用、但当前环境未定义的变量
#[tauri::command]
pub fn undefined_vars(config: RequestConfig, env_vars: HashMap<String, String>) -> Vec<String> {
    let mut texts: Vec<&str> = vec![
        &config.url,
        &config.body,
        &config.auth.token,
        &config.auth.username,
        &config.auth.password,
        &config.auth.header_value,
    ];
    for p in &config.params {
        texts.push(&p.key);
        texts.push(&p.value);
    }
    for h in &config.headers {
        texts.push(&h.key);
        texts.push(&h.value);
    }
    for f in &config.form_fields {
        texts.push(&f.key);
        texts.push(&f.value);
    }

    let mut out: Vec<String> = Vec::new();
    for t in texts {
        for n in vars::collect_var_names(t) {
            if !env_vars.contains_key(&n) && !out.contains(&n) {
                out.push(n);
            }
        }
    }
    out
}

/// 把最近的响应体保存到文件，返回保存路径
#[tauri::command]
pub fn save_response_file(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let guard = state.last_response.lock().map_err(err)?;
    let Some(resp) = guard.as_ref() else {
        return Err("没有可保存的响应".to_string());
    };
    let content_type = resp
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.clone())
        .unwrap_or_default();
    let name = guess_filename(&content_type);

    let Some(path) = rfd::FileDialog::new().set_file_name(&name).save_file() else {
        return Ok(None);
    };
    std::fs::write(&path, &resp.body).map_err(err)?;
    Ok(Some(path.display().to_string()))
}

// ---------- 设置 ----------

#[tauri::command]
pub fn get_last_config(state: State<'_, AppState>) -> Result<Option<RequestConfig>, String> {
    let db = state.db.lock().map_err(err)?;
    let raw = db.get_setting("last_config").map_err(err)?;
    Ok(raw.and_then(|json| serde_json::from_str::<RequestConfig>(&json).ok()))
}

#[tauri::command]
pub fn set_last_config(state: State<'_, AppState>, config: RequestConfig) -> Result<(), String> {
    let db = state.db.lock().map_err(err)?;
    let json = serde_json::to_string(&config).map_err(err)?;
    db.set_setting("last_config", &json).map_err(err)
}

// ---------- 历史 ----------

#[tauri::command]
pub fn history_list(state: State<'_, AppState>, limit: usize) -> Result<Vec<HistoryItem>, String> {
    let db = state.db.lock().map_err(err)?;
    db.list_history(limit).map_err(err)
}

#[tauri::command]
pub fn history_clear(state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(err)?;
    db.clear_history().map_err(err)
}

#[tauri::command]
pub fn history_delete(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(err)?;
    db.delete_history(id).map_err(err)
}

// ---------- 集合 ----------

#[tauri::command]
pub fn collections_list(state: State<'_, AppState>) -> Result<Vec<Collection>, String> {
    let db = state.db.lock().map_err(err)?;
    db.list_collections().map_err(err)
}

#[tauri::command]
pub fn collection_create(state: State<'_, AppState>, name: String, parent_id: i64) -> Result<i64, String> {
    let db = state.db.lock().map_err(err)?;
    db.create_collection(name.trim(), parent_id).map_err(err)
}

#[tauri::command]
pub fn collection_delete(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(err)?;
    db.delete_collection(id).map_err(err)
}

#[tauri::command]
pub fn collection_export(state: State<'_, AppState>, id: i64) -> Result<String, String> {
    let db = state.db.lock().map_err(err)?;
    db.export_collection(id).map_err(err)
}

/// 导出到用户选择的文件，返回保存路径
#[tauri::command]
pub fn collection_export_file(state: State<'_, AppState>, id: i64, name: String) -> Result<Option<String>, String> {
    let json = {
        let db = state.db.lock().map_err(err)?;
        db.export_collection(id).map_err(err)?
    };
    let Some(path) = rfd::FileDialog::new()
        .add_filter("JSON", &["json"])
        .set_file_name(format!("{name}.json"))
        .save_file()
    else {
        return Ok(None);
    };
    std::fs::write(&path, json).map_err(err)?;
    Ok(Some(path.display().to_string()))
}

/// 从用户选择的文件导入，返回新集合 id
#[tauri::command]
pub fn collection_import_file(state: State<'_, AppState>) -> Result<Option<i64>, String> {
    let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() else {
        return Ok(None);
    };
    let json = std::fs::read_to_string(&path).map_err(err)?;
    let db = state.db.lock().map_err(err)?;
    let id = db.import_collection(&json).map_err(err)?;
    Ok(Some(id))
}

#[tauri::command]
pub fn saved_list(state: State<'_, AppState>, collection_id: i64) -> Result<Vec<SavedRequest>, String> {
    let db = state.db.lock().map_err(err)?;
    db.list_saved_requests(collection_id).map_err(err)
}

#[tauri::command]
pub fn saved_save(
    state: State<'_, AppState>,
    collection_id: i64,
    name: String,
    config: RequestConfig,
) -> Result<i64, String> {
    let db = state.db.lock().map_err(err)?;
    db.save_request(collection_id, &name, &config).map_err(err)
}

#[tauri::command]
pub fn saved_delete(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(err)?;
    db.delete_saved_request(id).map_err(err)
}

// ---------- 环境变量 ----------

#[tauri::command]
pub fn env_names(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let db = state.db.lock().map_err(err)?;
    let mut names = db.list_env_names().map_err(err)?;
    if !names.iter().any(|n| n == "global") {
        names.insert(0, "global".to_string());
    }
    Ok(names)
}

#[tauri::command]
pub fn env_vars(state: State<'_, AppState>, env_name: String) -> Result<Vec<EnvVar>, String> {
    let db = state.db.lock().map_err(err)?;
    db.list_env_vars(&env_name).map_err(err)
}

#[tauri::command]
pub fn env_set(state: State<'_, AppState>, env_name: String, key: String, value: String) -> Result<(), String> {
    let db = state.db.lock().map_err(err)?;
    db.set_env_var(&env_name, key.trim(), &value).map_err(err)
}

#[tauri::command]
pub fn env_delete(state: State<'_, AppState>, env_name: String, key: String) -> Result<(), String> {
    let db = state.db.lock().map_err(err)?;
    db.delete_env_var(&env_name, key.trim()).map_err(err)
}

#[tauri::command]
pub fn env_drop(state: State<'_, AppState>, env_name: String) -> Result<(), String> {
    let db = state.db.lock().map_err(err)?;
    db.drop_env(&env_name).map_err(err)
}

// ---------- cURL ----------

#[tauri::command]
pub fn curl_export(config: RequestConfig, env_vars: HashMap<String, String>) -> String {
    curl::to_curl(&resolve_config(&config, &env_vars))
}

#[tauri::command]
pub fn curl_import(text: String) -> Result<RequestConfig, String> {
    curl::parse_curl(&text).map_err(err)
}

// ---------- 文件选择 ----------

#[tauri::command]
pub fn pick_file() -> Option<String> {
    rfd::FileDialog::new()
        .pick_file()
        .map(|p| p.display().to_string())
}
