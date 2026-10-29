//! HTTP 请求客户端封装（reqwest async）

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::multipart;
use reqwest::Client;
use tokio::sync::Notify;

use crate::model::request::{AuthType, BodyType, Method, RequestConfig, ResponseData};

/// 单个响应最多读取的字节数：超过后停止接收，避免超大响应把内存吃满
pub const MAX_BODY_READ: usize = 8 * 1024 * 1024;

/// 用户主动取消时的错误信息（前端据此区分普通失败）
pub const CANCELLED: &str = "请求已取消";

/// 构造基础 URL：params 追加到 query
pub(crate) fn build_url(config: &RequestConfig) -> String {
    let mut url = config.url.trim().to_string();
    if url.is_empty() {
        return url;
    }
    // 用 url crate 追加 query
    if let Ok(mut parsed) = url::Url::parse(&url) {
        let mut pairs: Vec<(String, String)> = parsed
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        for p in config.params.iter().filter(|p| p.enabled && !p.key.is_empty()) {
            pairs.push((p.key.clone(), p.value.clone()));
        }
        let mut qs = url::form_urlencoded::Serializer::new(String::new());
        for (k, v) in &pairs {
            qs.append_pair(k, v);
        }
        let qs = qs.finish();
        if qs.is_empty() {
            parsed.set_query(None);
        } else {
            parsed.set_query(Some(&qs));
        }
        url = parsed.to_string();
    }
    url
}

/// 构造（并复用）reqwest 客户端：相同连接参数共享连接池，避免每次请求重新握手
fn client_for(config: &RequestConfig) -> Result<Client> {
    static CACHE: OnceLock<Mutex<HashMap<String, Client>>> = OnceLock::new();

    let timeout = if config.timeout_secs == 0 {
        30
    } else {
        config.timeout_secs
    };
    let proxy = if config.use_custom_proxy {
        config.proxy.trim()
    } else {
        ""
    };
    let key = format!(
        "{timeout}|{}|{}|{proxy}",
        config.insecure_skip_verify, config.follow_redirects
    );

    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(map) = cache.lock() {
        if let Some(client) = map.get(&key) {
            return Ok(client.clone());
        }
    }

    let mut builder = Client::builder().timeout(Duration::from_secs(timeout));
    if config.insecure_skip_verify {
        builder = builder.danger_accept_invalid_certs(true);
    }
    if !config.follow_redirects {
        builder = builder.redirect(reqwest::redirect::Policy::none());
    }
    if !proxy.is_empty() {
        let proxy = reqwest::Proxy::all(proxy).context("代理地址无效，请使用 http://host:port 格式")?;
        builder = builder.proxy(proxy);
    }

    let client = builder.build().context("创建 HTTP 客户端失败")?;
    if let Ok(mut map) = cache.lock() {
        // 参数组合很少，超过阈值直接清空，避免缓存无限增长
        if map.len() >= 4 {
            map.clear();
        }
        map.insert(key, client.clone());
    }
    Ok(client)
}

/// 构造请求头（含认证注入）
fn apply_headers(builder: reqwest::RequestBuilder, config: &RequestConfig) -> reqwest::RequestBuilder {
    let mut headers = HeaderMap::new();
    // 用户自定义 headers
    for h in config.headers.iter().filter(|h| h.enabled && !h.key.trim().is_empty()) {
        match (
            HeaderName::from_bytes(h.key.trim().as_bytes()),
            HeaderValue::from_str(h.value.as_str()),
        ) {
            (Ok(name), Ok(value)) => {
                // 允许覆盖同名（以最后一个为准）
                headers.insert(name, value);
            }
            _ => {}
        }
    }
    // 认证
    match config.auth.auth_type {
        AuthType::None => {}
        AuthType::Basic => {
            let cred = format!("{}:{}", config.auth.username, config.auth.password);
            let encoded = base64_encode(cred.as_bytes());
            if let Ok(val) = HeaderValue::from_str(&format!("Basic {}", encoded)) {
                headers.insert(AUTHORIZATION, val);
            }
        }
        AuthType::Bearer => {
            if !config.auth.token.is_empty() {
                if let Ok(val) = HeaderValue::from_str(&format!("Bearer {}", config.auth.token)) {
                    headers.insert(AUTHORIZATION, val);
                }
            }
        }
        AuthType::CustomHeader => {
            if !config.auth.header_name.trim().is_empty() {
                if let Ok(name) = HeaderName::from_bytes(config.auth.header_name.trim().as_bytes()) {
                    if let Ok(val) = HeaderValue::from_str(&config.auth.header_value) {
                        headers.insert(name, val);
                    }
                }
            }
        }
    }

    // Content-Type 根据 body 类型自动设置（若用户未显式设置）
    if !headers.contains_key(CONTENT_TYPE) {
        let ct = match config.body_type {
            BodyType::Json => Some("application/json"),
            BodyType::Form => Some("application/x-www-form-urlencoded"),
            BodyType::Multipart => None, // multipart 客户端自动设置 boundary
            BodyType::Text => Some("text/plain"),
            BodyType::None => None,
        };
        if let Some(ct) = ct {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static(ct));
        }
    }

    builder.headers(headers)
}

/// 请求方法
fn method_of(m: Method) -> reqwest::Method {
    match m {
        Method::Get => reqwest::Method::GET,
        Method::Post => reqwest::Method::POST,
        Method::Put => reqwest::Method::PUT,
        Method::Patch => reqwest::Method::PATCH,
        Method::Delete => reqwest::Method::DELETE,
        Method::Head => reqwest::Method::HEAD,
        Method::Options => reqwest::Method::OPTIONS,
    }
}

/// 简易 base64（不做 padding 校验）
fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

/// 依 Content-Type（缺失时看内容）判断是否二进制
pub fn is_binary_body(content_type: &str, body: &[u8]) -> bool {
    let ct = content_type.to_lowercase();
    if ct.is_empty() {
        return body.contains(&0) || std::str::from_utf8(body).is_err();
    }
    const TEXTUAL: [&str; 9] = [
        "json",
        "text/",
        "xml",
        "javascript",
        "x-www-form-urlencoded",
        "html",
        "csv",
        "yaml",
        "graphql",
    ];
    !TEXTUAL.iter().any(|t| ct.contains(t))
}

/// 执行请求并读取响应体（最多 MAX_BODY_READ 字节）
async fn execute(builder: reqwest::RequestBuilder, start: Instant) -> ResponseData {
    let resp = match builder.send().await {
        Ok(r) => r,
        Err(e) => {
            let msg = if e.is_timeout() {
                "请求超时".to_string()
            } else if e.is_connect() {
                format!("连接失败：{e}")
            } else if e.is_builder() {
                format!("请求构建失败：{e}")
            } else {
                format!("请求失败：{e}")
            };
            return ResponseData {
                error: Some(msg),
                duration_ms: start.elapsed().as_millis(),
                ..Default::default()
            };
        }
    };

    let status_code = resp.status().as_u16();
    let status_text = resp.status().canonical_reason().unwrap_or("").to_string();
    let http_version = format!("{:?}", resp.version());
    let headers: Vec<(String, String)> = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let content_type = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.clone())
        .unwrap_or_default();

    // 流式读取：达到上限立即停止，避免超大响应占满内存
    let mut resp = resp;
    let mut body: Vec<u8> = Vec::new();
    let mut body_truncated = false;
    loop {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                let remain = MAX_BODY_READ.saturating_sub(body.len());
                if chunk.len() >= remain {
                    body.extend_from_slice(&chunk[..remain]);
                    body_truncated = true;
                    break;
                }
                body.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(e) => {
                // 已经拿到状态码和头，按“读取中断”处理，尽量保留已有内容
                return ResponseData {
                    status_code,
                    status_text,
                    http_version,
                    headers,
                    body,
                    is_binary: false,
                    body_truncated: true,
                    duration_ms: start.elapsed().as_millis(),
                    error: Some(format!("读取响应体失败：{e}")),
                };
            }
        }
    }

    let is_binary = is_binary_body(&content_type, &body);
    ResponseData {
        status_code,
        status_text,
        http_version,
        headers,
        body,
        is_binary,
        body_truncated,
        duration_ms: start.elapsed().as_millis(),
        error: None,
    }
}

/// 执行请求，返回响应数据（无论成功失败都返回 ResponseData）
///
/// `cancel` 被唤醒时会立刻放弃这次请求（未完成的 future 被 drop，连接随之关闭）。
pub async fn send(config: &RequestConfig, cancel: &Notify) -> ResponseData {
    let start = Instant::now();
    let client = match client_for(config) {
        Ok(c) => c,
        Err(e) => {
            return ResponseData {
                error: Some(e.to_string()),
                duration_ms: start.elapsed().as_millis(),
                ..Default::default()
            };
        }
    };

    let url = build_url(config);
    if url.is_empty() {
        return ResponseData {
            error: Some("URL 不能为空".to_string()),
            duration_ms: start.elapsed().as_millis(),
            ..Default::default()
        };
    }

    let method = method_of(config.method);
    let mut builder = client.request(method, &url);
    builder = apply_headers(builder, config);

    // Body
    match config.body_type {
        BodyType::None => {}
        BodyType::Json | BodyType::Text => {
            if !config.body.trim().is_empty() {
                builder = builder.body(config.body.clone());
            }
        }
        BodyType::Form => {
            let fields: Vec<(String, String)> = config
                .form_fields
                .iter()
                .filter(|f| f.enabled && !f.key.is_empty())
                .map(|f| (f.key.clone(), f.value.clone()))
                .collect();
            builder = builder.form(&fields);
        }
        BodyType::Multipart => {
            let mut form = multipart::Form::new();
            for f in config.form_fields.iter().filter(|f| f.enabled && !f.key.is_empty()) {
                form = form.text(f.key.clone(), f.value.clone());
            }
            if !config.file_path.trim().is_empty() {
                let path = config.file_path.trim();
                if std::path::Path::new(path).is_file() {
                    let fname = path
                        .rsplit(['/', '\\'])
                        .next()
                        .unwrap_or("file")
                        .to_string();
                    match tokio::fs::read(path).await {
                        Ok(bytes) => {
                            form = form.part("file", multipart::Part::bytes(bytes).file_name(fname));
                        }
                        Err(e) => {
                            return ResponseData {
                                error: Some(format!("读取上传文件失败：{e}")),
                                duration_ms: start.elapsed().as_millis(),
                                ..Default::default()
                            };
                        }
                    }
                }
            }
            builder = builder.multipart(form);
        }
    }

    tokio::select! {
        biased;
        _ = cancel.notified() => ResponseData {
            error: Some(CANCELLED.to_string()),
            duration_ms: start.elapsed().as_millis(),
            ..Default::default()
        },
        result = execute(builder, start) => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::request::{KVPair, RequestConfig};

    #[test]
    fn test_build_url_params() {
        let mut cfg = RequestConfig::default();
        cfg.url = "https://example.com/path?a=1".into();
        cfg.params = vec![
            KVPair {
                key: "b".into(),
                value: "2".into(),
                enabled: true,
            },
            KVPair {
                key: "disabled".into(),
                value: "x".into(),
                enabled: false,
            },
        ];
        let url = build_url(&cfg);
        assert!(url.contains("a=1"), "保留原始查询参数: {url}");
        assert!(url.contains("b=2"), "追加启用参数: {url}");
        assert!(!url.contains("disabled"), "停用参数不应拼接: {url}");
    }

    #[test]
    fn test_base64() {
        assert_eq!(base64_encode(b"user:pass"), "dXNlcjpwYXNz");
    }

    #[test]
    fn test_is_binary_body() {
        assert!(is_binary_body("image/png", b"\x89PNG"));
        assert!(is_binary_body("application/octet-stream", b".."));
        assert!(!is_binary_body("application/json; charset=utf-8", b"{\"a\":1}"));
        assert!(!is_binary_body("text/html", b"<p>x</p>"));
        // 无 Content-Type 时按内容判断
        assert!(is_binary_body("", &[0u8, 159, 146, 150]));
        assert!(!is_binary_body("", b"plain"));
    }
}
