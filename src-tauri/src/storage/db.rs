//! SQLite 持久化层：历史 / 集合 / 环境变量

use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::model::request::RequestConfig;
use crate::model::saved::{Collection, CollectionExport, EnvVar, HistoryItem, SavedRequest};

/// 数据库封装
pub struct Db {
    conn: Connection,
}

impl Db {
    /// 打开（不存在则创建）数据库并建表
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).context("打开数据库失败")?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS collections (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                parent_id INTEGER NOT NULL DEFAULT 0,
                name TEXT NOT NULL,
                sort INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS saved_requests (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                collection_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                config TEXT NOT NULL,
                sort INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                config TEXT NOT NULL,
                status_code INTEGER,
                duration_ms INTEGER,
                created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
            );

            CREATE TABLE IF NOT EXISTS environments (
                env_name TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                PRIMARY KEY (env_name, key)
            );

            CREATE INDEX IF NOT EXISTS idx_saved_requests_collection ON saved_requests(collection_id);
            CREATE INDEX IF NOT EXISTS idx_history_created ON history(created_at DESC);

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )
        .context("初始化数据库表失败")?;
        // 默认提供 global 环境
        conn.execute(
            "INSERT OR IGNORE INTO collections (id, name, sort) VALUES (1, '默认集合', 0)",
            [],
        )
        .ok();
        Ok(Self { conn })
    }

    // ---------- 集合 ----------

    pub fn list_collections(&self) -> Result<Vec<Collection>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, parent_id, name FROM collections ORDER BY sort, id")
            .context("查询集合失败")?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Collection {
                    id: r.get(0)?,
                    parent_id: r.get(1)?,
                    name: r.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn create_collection(&self, name: &str, parent_id: i64) -> Result<i64> {
        self.conn
            .execute(
                "INSERT INTO collections (parent_id, name) VALUES (?1, ?2)",
                params![parent_id, name],
            )
            .context("创建集合失败")?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn delete_collection(&self, id: i64) -> Result<()> {
        // 先删集合下的请求，再删集合
        self.conn
            .execute(
                "DELETE FROM saved_requests WHERE collection_id=?1",
                params![id],
            )
            .context("删除集合请求失败")?;
        self.conn
            .execute("DELETE FROM collections WHERE id=?1", params![id])
            .context("删除集合失败")?;
        Ok(())
    }

    // ---------- 保存的请求 ----------

    pub fn list_saved_requests(&self, collection_id: i64) -> Result<Vec<SavedRequest>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, collection_id, name, config FROM saved_requests WHERE collection_id=?1 ORDER BY sort, id")
            .context("查询保存请求失败")?;
        let rows = stmt
            .query_map(params![collection_id], |r| {
                let config: String = r.get(3)?;
                Ok(SavedRequest {
                    id: r.get(0)?,
                    collection_id: r.get(1)?,
                    name: r.get(2)?,
                    config: serde_json::from_str(&config).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e))
                    })?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn save_request(&self, collection_id: i64, name: &str, config: &RequestConfig) -> Result<i64> {
        let config_json = serde_json::to_string(config).context("序列化请求失败")?;
        self.conn
            .execute(
                "INSERT INTO saved_requests (collection_id, name, config) VALUES (?1, ?2, ?3)",
                params![collection_id, name, config_json],
            )
            .context("保存请求失败")?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn delete_saved_request(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM saved_requests WHERE id=?1", params![id])
            .context("删除请求失败")?;
        Ok(())
    }

    // ---------- 历史 ----------

    pub fn add_history(
        &self,
        config: &RequestConfig,
        status_code: Option<u16>,
        duration_ms: Option<u128>,
    ) -> Result<i64> {
        let config_json = serde_json::to_string(config).context("序列化请求失败")?;
        self.conn
            .execute(
                "INSERT INTO history (config, status_code, duration_ms) VALUES (?1, ?2, ?3)",
                params![
                    config_json,
                    status_code.map(|v| v as i64),
                    duration_ms.map(|v| v as i64)
                ],
            )
            .context("写入历史失败")?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_history(&self, limit: usize) -> Result<Vec<HistoryItem>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, config, status_code, duration_ms, created_at FROM history ORDER BY id DESC LIMIT ?1")
            .context("查询历史失败")?;
        let rows = stmt
            .query_map(params![limit as i64], |r| {
                let config: String = r.get(1)?;
                Ok(HistoryItem {
                    id: r.get(0)?,
                    config: serde_json::from_str(&config).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
                    })?,
                    status_code: r.get::<_, Option<i64>>(2)?.map(|v| v as u16),
                    duration_ms: r.get::<_, Option<i64>>(3)?.map(|v| v as u128),
                    created_at: r.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn delete_history(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM history WHERE id=?1", params![id])
            .context("删除历史失败")?;
        Ok(())
    }

    pub fn clear_history(&self) -> Result<()> {
        self.conn
            .execute("DELETE FROM history", [])
            .context("清空历史失败")?;
        Ok(())
    }

    /// 只保留最近 `keep` 条历史，避免库无限膨胀
    pub fn trim_history(&self, keep: i64) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY id DESC LIMIT ?1)",
                params![keep],
            )
            .context("清理历史失败")?;
        Ok(())
    }

    // ---------- 环境变量 ----------

    pub fn list_env_names(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT env_name FROM environments ORDER BY env_name")
            .context("查询环境列表失败")?;
        let rows = stmt
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<String>>>()?;
        Ok(rows)
    }

    pub fn list_env_vars(&self, env_name: &str) -> Result<Vec<EnvVar>> {
        let mut stmt = self
            .conn
            .prepare("SELECT env_name, key, value FROM environments WHERE env_name=?1 ORDER BY key")
            .context("查询环境变量失败")?;
        let rows = stmt
            .query_map(params![env_name], |r| {
                Ok(EnvVar {
                    env_name: r.get(0)?,
                    key: r.get(1)?,
                    value: r.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn set_env_var(&self, env_name: &str, key: &str, value: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO environments (env_name, key, value) VALUES (?1, ?2, ?3)
                 ON CONFLICT(env_name, key) DO UPDATE SET value=excluded.value",
                params![env_name, key, value],
            )
            .context("保存环境变量失败")?;
        Ok(())
    }

    pub fn delete_env_var(&self, env_name: &str, key: &str) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM environments WHERE env_name=?1 AND key=?2",
                params![env_name, key],
            )
            .context("删除环境变量失败")?;
        Ok(())
    }

    pub fn drop_env(&self, env_name: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM environments WHERE env_name=?1", params![env_name])
            .context("删除环境失败")?;
        Ok(())
    }

    // ---------- 设置 ----------

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                params![key, value],
            )
            .context("保存设置失败")?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM settings WHERE key=?1")
            .context("读取设置失败")?;
        let mut rows = stmt.query(params![key]).context("读取设置失败")?;
        if let Some(row) = rows.next().context("读取设置失败")? {
            Ok(Some(row.get(0).context("读取设置失败")?))
        } else {
            Ok(None)
        }
    }

    // ---------- 导入 / 导出 ----------

    /// 导出某个集合为 JSON 字符串
    pub fn export_collection(&self, collection_id: i64) -> Result<String> {
        let col = self
            .list_collections()?
            .into_iter()
            .find(|c| c.id == collection_id)
            .context("集合不存在")?;
        let reqs = self.list_saved_requests(collection_id)?;
        let export = CollectionExport {
            name: col.name,
            requests: reqs
                .into_iter()
                .map(|r| (r.name, r.config))
                .collect(),
        };
        let json = serde_json::to_string_pretty(&export).context("序列化导出失败")?;
        Ok(json)
    }

    /// 从 JSON 导入为新集合，返回新集合 id
    pub fn import_collection(&self, json: &str) -> Result<i64> {
        let export: CollectionExport =
            serde_json::from_str(json).context("JSON 解析失败，格式不正确")?;
        let id = self.create_collection(&export.name, 0)?;
        for (name, config) in export.requests {
            self.save_request(id, &name, &config)?;
        }
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::request::{BodyType, Method, RequestConfig};

    fn test_db() -> (Db, std::path::PathBuf) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "http_tool_test_{}_{}.db",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).expect("打开测试库失败");
        (db, path)
    }

    fn sample_config() -> RequestConfig {
        RequestConfig {
            method: Method::Post,
            url: "https://example.com/api/{{path}}".into(),
            body_type: BodyType::Json,
            body: "{\"a\":1}".into(),
            ..Default::default()
        }
    }

    #[test]
    fn test_collection_crud() {
        let (db, p) = test_db();
        // 集合列表（含默认集合）
        let cols = db.list_collections().unwrap();
        assert!(cols.iter().any(|c| c.name == "默认集合"));
        // 新建
        let id = db.create_collection("测试集合", 0).unwrap();
        assert!(id > 0);
        let cols = db.list_collections().unwrap();
        let c = cols.iter().find(|c| c.id == id).unwrap();
        assert_eq!(c.name, "测试集合");
        // 删除
        db.delete_collection(id).unwrap();
        let cols = db.list_collections().unwrap();
        assert!(!cols.iter().any(|c| c.id == id));
        drop(db);
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_file(format!("{}-wal", p.display()));
        let _ = std::fs::remove_file(format!("{}-shm", p.display()));
    }

    #[test]
    fn test_saved_request_crud() {
        let (db, p) = test_db();
        let cfg = sample_config();
        let col_id = db.create_collection("A", 0).unwrap();
        db.save_request(col_id, "请求1", &cfg).unwrap();
        let list = db.list_saved_requests(col_id).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "请求1");
        assert_eq!(list[0].config.method, Method::Post);
        assert_eq!(list[0].config.url, cfg.url);
        drop(db);
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_file(format!("{}-wal", p.display()));
        let _ = std::fs::remove_file(format!("{}-shm", p.display()));
    }

    #[test]
    fn test_history() {
        let (db, p) = test_db();
        let cfg = sample_config();
        db.add_history(&cfg, Some(200), Some(42)).unwrap();
        db.add_history(&cfg, Some(500), Some(10)).unwrap();
        let h = db.list_history(10).unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].status_code, Some(500)); // 最新在前
        db.delete_history(h[0].id).unwrap();
        let h = db.list_history(10).unwrap();
        assert_eq!(h.len(), 1);
        drop(db);
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_file(format!("{}-wal", p.display()));
        let _ = std::fs::remove_file(format!("{}-shm", p.display()));
    }

    #[test]
    fn test_history_trim() {
        let (db, p) = test_db();
        let cfg = sample_config();
        for i in 0..5 {
            db.add_history(&cfg, Some(200), Some(i)).unwrap();
        }
        db.trim_history(2).unwrap();
        let h = db.list_history(10).unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].duration_ms, Some(4)); // 保留最新的两条
        drop(db);
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_file(format!("{}-wal", p.display()));
        let _ = std::fs::remove_file(format!("{}-shm", p.display()));
    }

    #[test]
    fn test_env_vars() {
        let (db, p) = test_db();
        db.set_env_var("dev", "host", "example.com").unwrap();
        db.set_env_var("dev", "token", "abc").unwrap();
        db.set_env_var("prod", "host", "prod.example.com").unwrap();
        assert_eq!(db.list_env_names().unwrap(), vec!["dev", "prod"]);
        let vars = db.list_env_vars("dev").unwrap();
        assert_eq!(
            vars.iter().find(|v| v.key == "host").map(|v| v.value.as_str()),
            Some("example.com")
        );
        // 覆盖
        db.set_env_var("dev", "token", "xyz").unwrap();
        let vars = db.list_env_vars("dev").unwrap();
        assert_eq!(
            vars.iter().find(|v| v.key == "token").map(|v| v.value.as_str()),
            Some("xyz")
        );
        db.delete_env_var("dev", "token").unwrap();
        let vars = db.list_env_vars("dev").unwrap();
        assert!(!vars.iter().any(|v| v.key == "token"));
        db.drop_env("dev").unwrap();
        assert_eq!(db.list_env_names().unwrap(), vec!["prod"]);
        drop(db);
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_file(format!("{}-wal", p.display()));
        let _ = std::fs::remove_file(format!("{}-shm", p.display()));
    }

    #[test]
    fn test_export_import() {
        let (db, p) = test_db();
        let col_id = db.create_collection("导出集合", 0).unwrap();
        db.save_request(col_id, "r1", &sample_config()).unwrap();
        let json = db.export_collection(col_id).unwrap();
        // 导入到新库
        let (db2, p2) = test_db();
        let new_id = db2.import_collection(&json).unwrap();
        assert!(new_id > 0);
        let reqs = db2.list_saved_requests(new_id).unwrap();
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].name, "r1");
        drop(db);
        drop(db2);
        for p in [p, p2] {
            let _ = std::fs::remove_file(&p);
            let _ = std::fs::remove_file(format!("{}-wal", p.display()));
            let _ = std::fs::remove_file(format!("{}-shm", p.display()));
        }
    }
}
