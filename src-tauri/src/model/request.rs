//! 请求相关数据结构

use serde::{Deserialize, Serialize};

/// HTTP 方法
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl Method {
    pub fn as_str(&self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Delete => "DELETE",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
        }
    }

    pub const ALL: [Method; 7] = [
        Method::Get,
        Method::Post,
        Method::Put,
        Method::Patch,
        Method::Delete,
        Method::Head,
        Method::Options,
    ];
}

impl Default for Method {
    fn default() -> Self {
        Method::Get
    }
}

/// 键值对（Params / Headers / form 共用）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KVPair {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub value: String,
    /// 是否启用（允许临时停用某一条）
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Body 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BodyType {
    #[default]
    None,
    Json,
    Form,
    Text,
    Multipart,
}

impl BodyType {
    pub const ALL: [BodyType; 5] = [
        BodyType::None,
        BodyType::Json,
        BodyType::Form,
        BodyType::Text,
        BodyType::Multipart,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            BodyType::None => "none",
            BodyType::Json => "JSON",
            BodyType::Form => "Form URL Encoded",
            BodyType::Text => "Text",
            BodyType::Multipart => "Multipart Form",
        }
    }
}

/// 认证方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AuthType {
    #[default]
    None,
    Basic,
    Bearer,
    /// 自定义 Header（如 API-Key）
    CustomHeader,
}

impl AuthType {
    pub const ALL: [AuthType; 4] = [
        AuthType::None,
        AuthType::Basic,
        AuthType::Bearer,
        AuthType::CustomHeader,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            AuthType::None => "无",
            AuthType::Basic => "Basic Auth",
            AuthType::Bearer => "Bearer Token",
            AuthType::CustomHeader => "自定义 Header",
        }
    }
}

/// 认证配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Auth {
    #[serde(default)]
    pub auth_type: AuthType,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub header_name: String,
    #[serde(default)]
    pub header_value: String,
}

/// 一组请求配置（可保存到集合 / 历史，也可作为当前编辑状态）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestConfig {
    pub method: Method,
    pub url: String,
    #[serde(default)]
    pub params: Vec<KVPair>,
    #[serde(default)]
    pub headers: Vec<KVPair>,
    #[serde(default)]
    pub body_type: BodyType,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub form_fields: Vec<KVPair>,
    #[serde(default)]
    pub auth: Auth,
    /// 是否跳过 TLS 证书校验
    #[serde(default)]
    pub insecure_skip_verify: bool,
    /// 是否跟随重定向
    #[serde(default = "default_true")]
    pub follow_redirects: bool,
    /// 请求超时（秒）
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub proxy: String,
    /// multipart 文件路径
    #[serde(default)]
    pub file_path: String,
    /// 是否启用 proxy（空字符串视为未启用）
    #[serde(default)]
    pub use_custom_proxy: bool,
}

fn default_timeout() -> u64 {
    30
}

impl Default for RequestConfig {
    fn default() -> Self {
        Self {
            method: Method::Get,
            url: String::new(),
            params: Vec::new(),
            headers: Vec::new(),
            body_type: BodyType::None,
            body: String::new(),
            form_fields: Vec::new(),
            auth: Auth::default(),
            insecure_skip_verify: false,
            follow_redirects: true,
            timeout_secs: default_timeout(),
            proxy: String::new(),
            file_path: String::new(),
            use_custom_proxy: false,
        }
    }
}

impl RequestConfig {
    /// 新建一个默认 GET 请求
    pub fn new_empty() -> Self {
        Self::default()
    }
}

/// 发送请求后的响应结果
#[derive(Debug, Clone, Default)]
pub struct ResponseData {
    pub status_code: u16,
    pub status_text: String,
    pub http_version: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// 响应体是否为二进制（图片 / 压缩包 / 字体等，不应按文本渲染）
    pub is_binary: bool,
    /// 响应体是否因超过读取上限被截断
    pub body_truncated: bool,
    /// 请求总耗时（毫秒）
    pub duration_ms: u128,
    /// 错误信息（请求失败时填充）
    pub error: Option<String>,
}

impl ResponseData {
    pub fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }

    /// 推断响应体类型：优先 JSON 格式化
    pub fn formatted_body(&self) -> String {
        let text = self.body_text();
        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or(text),
            Err(_) => text,
        }
    }
}
