//! 集合与历史的数据结构

use serde::{Deserialize, Serialize};

use super::request::RequestConfig;

/// 集合（分组），支持父子层级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: i64,
    pub parent_id: i64,
    pub name: String,
}

/// 集合内保存的请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedRequest {
    pub id: i64,
    pub collection_id: i64,
    pub name: String,
    pub config: RequestConfig,
}

/// 历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: i64,
    pub config: RequestConfig,
    pub created_at: String,
    pub status_code: Option<u16>,
    pub duration_ms: Option<u128>,
}

/// 导出 / 导入用（存整个集合和其请求）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionExport {
    pub name: String,
    pub requests: Vec<(String, RequestConfig)>,
}

/// 环境变量表项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub env_name: String,
    pub key: String,
    pub value: String,
}
