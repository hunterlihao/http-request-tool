//! 应用入口：初始化数据库、注册命令、启动窗口

pub mod commands;
pub mod http;
pub mod model;
pub mod storage;
pub mod util;

use std::path::PathBuf;
use std::sync::Mutex;

use commands::AppState;
use storage::db::Db;

/// 数据库文件路径：便携版放 exe 同目录
fn db_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            return dir.join("http-tool.db");
        }
    }
    PathBuf::from("http-tool.db")
}

/// 启动桌面应用
pub fn run() {
    let path = db_path();
    let db = match Db::open(&path) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("数据库初始化失败（{}）：{e}", path.display());
            std::process::exit(1);
        }
    };

    let state = AppState {
        db: Mutex::new(db),
        last_response: Mutex::new(None),
        active_cancel: Mutex::new(None),
    };

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::send_request,
            commands::cancel_request,
            commands::undefined_vars,
            commands::save_response_file,
            commands::get_last_config,
            commands::set_last_config,
            commands::history_list,
            commands::history_clear,
            commands::history_delete,
            commands::collections_list,
            commands::collection_create,
            commands::collection_delete,
            commands::collection_export,
            commands::collection_export_file,
            commands::collection_import_file,
            commands::saved_list,
            commands::saved_save,
            commands::saved_delete,
            commands::env_names,
            commands::env_vars,
            commands::env_set,
            commands::env_delete,
            commands::env_drop,
            commands::curl_export,
            commands::curl_import,
            commands::pick_file,
        ])
        .run(tauri::generate_context!())
        .expect("启动桌面应用失败");
}
