//! 应用状态与主循环

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use eframe::egui;

use crate::http::client;
use crate::model::request::{RequestConfig, ResponseData};
use crate::model::saved::{Collection, HistoryItem, SavedRequest};
use crate::storage::db::Db;
use crate::theme;
use crate::ui;
use crate::util::vars;

/// 请求配置区当前激活的分页
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestTab {
    Params,
    Headers,
    Body,
    Auth,
    Advanced,
}

/// 左侧栏分页
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideTab {
    History,
    Collections,
    Environments,
}

/// 响应区分页
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseTab {
    Body,
    Headers,
}

/// 轻提示类型
#[derive(Debug, Clone, Copy)]
pub enum ToastKind {
    Info,
    Error,
}

/// 轻提示（自动过期）
pub struct Toast {
    pub text: String,
    pub kind: ToastKind,
    pub created: std::time::Instant,
}

/// 应用全局状态
pub struct AppState {
    pub db: Db,

    // 当前编辑中的请求
    pub config: RequestConfig,
    pub response: Option<ResponseData>,
    pending: Option<Receiver<ResponseData>>,
    pub sending: bool,

    // 集合
    pub collections: Vec<Collection>,
    pub selected_collection: Option<i64>,
    pub saved_requests: Vec<SavedRequest>,
    pub save_dialog_open: bool,
    pub save_name: String,
    pub new_collection_name: String,

    // 历史
    pub history: Vec<HistoryItem>,
    pub history_search: String,

    // 环境
    pub env_names: Vec<String>,
    pub current_env: String,
    pub env_vars: Vec<(String, String)>,
    pub env_map: HashMap<String, String>,
    pub new_env_name: String,

    // UI 状态
    pub request_tab: RequestTab,
    pub side_tab: SideTab,
    pub response_tab: ResponseTab,
    pub show_sidebar: bool,
    pub toast: Option<Toast>,
    pub body_raw_view: bool,
}

impl AppState {
    pub fn new(db: Db) -> Self {
        let mut s = Self {
            db,
            config: RequestConfig::new_empty(),
            response: None,
            pending: None,
            sending: false,
            collections: Vec::new(),
            selected_collection: None,
            saved_requests: Vec::new(),
            save_dialog_open: false,
            save_name: String::new(),
            new_collection_name: String::new(),
            history: Vec::new(),
            history_search: String::new(),
            env_names: vec!["global".to_string()],
            current_env: "global".to_string(),
            env_vars: Vec::new(),
            env_map: HashMap::new(),
            new_env_name: String::new(),
            request_tab: RequestTab::Params,
            side_tab: SideTab::History,
            response_tab: ResponseTab::Body,
            show_sidebar: true,
            toast: None,
            body_raw_view: false,
        };
        s.reload_collections();
        s.reload_history();
        s.reload_envs();
        s.load_env("global");
        s
    }

    // ---------- 数据刷新 ----------

    pub fn reload_collections(&mut self) {
        match self.db.list_collections() {
            Ok(list) => {
                self.collections = list;
                if self.selected_collection.is_none() {
                    self.selected_collection = self.collections.first().map(|c| c.id);
                }
                self.reload_saved_requests();
            }
            Err(e) => self.toast_error(format!("加载集合失败：{e}")),
        }
    }

    pub fn reload_saved_requests(&mut self) {
        if let Some(cid) = self.selected_collection {
            match self.db.list_saved_requests(cid) {
                Ok(list) => self.saved_requests = list,
                Err(e) => self.toast_error(format!("加载请求失败：{e}")),
            }
        } else {
            self.saved_requests.clear();
        }
    }

    pub fn reload_history(&mut self) {
        match self.db.list_history(500) {
            Ok(list) => self.history = list,
            Err(e) => self.toast_error(format!("加载历史失败：{e}")),
        }
    }

    pub fn reload_envs(&mut self) {
        match self.db.list_env_names() {
            Ok(mut names) => {
                if !names.iter().any(|n| n == "global") {
                    names.insert(0, "global".to_string());
                }
                self.env_names = names;
            }
            Err(e) => self.toast_error(format!("加载环境失败：{e}")),
        }
    }

    /// 加载某环境到编辑区并生效
    pub fn load_env(&mut self, name: &str) {
        self.current_env = name.to_string();
        match self.db.list_env_vars(name) {
            Ok(list) => {
                self.env_vars = list.into_iter().map(|v| (v.key, v.value)).collect();
            }
            Err(e) => {
                self.env_vars.clear();
                self.toast_error(format!("加载环境变量失败：{e}"));
            }
        }
        self.rebuild_env_map();
    }

    /// 由编辑区重建生效 map 并持久化
    pub fn rebuild_env_map(&mut self) {
        self.env_map.clear();
        for (k, v) in &self.env_vars {
            if !k.trim().is_empty() {
                self.env_map.insert(k.trim().to_string(), v.clone());
            }
        }
        // 持久化
        for (k, v) in &self.env_vars {
            if !k.trim().is_empty() {
                if let Err(e) = self.db.set_env_var(&self.current_env, k.trim(), v) {
                    self.toast_error(format!("保存变量失败：{e}"));
                    break;
                }
            }
        }
    }

    // ---------- 发送请求 ----------

    pub fn send_request(&mut self, ctx: &egui::Context) {
        if self.sending {
            return;
        }
        let mut cfg = self.config.clone();
        // 变量替换（作用于 url / params / headers / body / 认证字段）
        let vars = &self.env_map;
        cfg.url = vars::replace_vars(&cfg.url, vars);
        for p in &mut cfg.params {
            p.key = vars::replace_vars(&p.key, vars);
            p.value = vars::replace_vars(&p.value, vars);
        }
        for h in &mut cfg.headers {
            h.key = vars::replace_vars(&h.key, vars);
            h.value = vars::replace_vars(&h.value, vars);
        }
        for f in &mut cfg.form_fields {
            f.key = vars::replace_vars(&f.key, vars);
            f.value = vars::replace_vars(&f.value, vars);
        }
        cfg.body = vars::replace_vars(&cfg.body, vars);
        cfg.auth.username = vars::replace_vars(&cfg.auth.username, vars);
        cfg.auth.password = vars::replace_vars(&cfg.auth.password, vars);
        cfg.auth.token = vars::replace_vars(&cfg.auth.token, vars);
        cfg.auth.header_value = vars::replace_vars(&cfg.auth.header_value, vars);

        let (tx, rx) = std::sync::mpsc::channel();
        self.pending = Some(rx);
        self.sending = true;
        let ctx2 = ctx.clone();
        std::thread::spawn(move || {
            let resp = client::send(&cfg);
            let _ = tx.send(resp);
            ctx2.request_repaint();
        });
    }

    /// 轮询后台请求结果
    fn poll_pending(&mut self) {
        let Some(rx) = &self.pending else { return };
        match rx.try_recv() {
            Ok(resp) => {
                let status = if resp.error.is_none() {
                    Some(resp.status_code)
                } else {
                    None
                };
                // 记录历史（保存原始配置，便于回放时仍含变量）
                if let Err(e) = self.db.add_history(&self.config, status, Some(resp.duration_ms)) {
                    self.toast_error(format!("写入历史失败：{e}"));
                }
                self.response = Some(resp);
                self.pending = None;
                self.sending = false;
                self.response_tab = ResponseTab::Body;
                self.reload_history();
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.pending = None;
                self.sending = false;
                self.toast_error("请求线程异常中断".to_string());
            }
        }
    }

    // ---------- 加载 / 保存 ----------

    /// 载入某条配置到编辑区
    pub fn apply_config(&mut self, cfg: RequestConfig) {
        self.config = cfg;
        self.response = None;
    }

    pub fn save_current_to_collection(&mut self, name: &str) {
        let Some(cid) = self.selected_collection else {
            self.toast_error("请先选择一个集合".to_string());
            return;
        };
        match self.db.save_request(cid, name, &self.config) {
            Ok(_) => {
                self.toast_info(format!("已保存「{name}」"));
                self.reload_saved_requests();
                self.save_dialog_open = false;
                self.save_name.clear();
            }
            Err(e) => self.toast_error(format!("保存失败：{e}")),
        }
    }

    pub fn create_collection(&mut self, name: &str) {
        if name.trim().is_empty() {
            self.toast_error("集合名称不能为空".to_string());
            return;
        }
        match self.db.create_collection(name.trim(), 0) {
            Ok(id) => {
                self.toast_info(format!("已创建集合「{name}」"));
                self.selected_collection = Some(id);
                self.reload_collections();
                self.new_collection_name.clear();
            }
            Err(e) => self.toast_error(format!("创建失败：{e}")),
        }
    }

    // ---------- 提示 ----------

    pub fn toast_info(&mut self, text: String) {
        self.toast = Some(Toast {
            text,
            kind: ToastKind::Info,
            created: std::time::Instant::now(),
        });
    }

    pub fn toast_error(&mut self, text: String) {
        self.toast = Some(Toast {
            text,
            kind: ToastKind::Error,
            created: std::time::Instant::now(),
        });
    }
}

impl eframe::App for AppState {
    /// 清屏色与画布一致，避免未填充区域露出黑色
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::from(theme::canvas()).to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_pending();
        ui::draw(ctx, self);

        // 提示 3 秒后自动消失
        if let Some(t) = &self.toast {
            if t.created.elapsed().as_secs_f32() > 3.0 {
                self.toast = None;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
            }
        }
    }
}

/// 数据库文件路径：便携版放 exe 同目录
fn db_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            return dir.join("http-tool.db");
        }
    }
    PathBuf::from("http-tool.db")
}

/// 应用入口
pub fn run() -> eframe::Result<()> {
    let path = db_path();
    let db = match Db::open(&path) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("数据库初始化失败（{}）：{e}", path.display());
            std::process::exit(1);
        }
    };
    let state = AppState::new(db);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1300.0, 880.0])
            .with_min_inner_size([980.0, 640.0])
            .with_title("HTTP 请求工具"),
        ..Default::default()
    };

    eframe::run_native(
        "http-request-tool",
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(state))
        }),
    )
}
