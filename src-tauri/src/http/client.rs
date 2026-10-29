//! HTTP 请求客户端封装（reqwest blocking）

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use reqwest::blocking::{multipart, Client, ClientBuilder, RequestBuilder};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

use crate::model::request::{AuthType, BodyType, Method, RequestConfig, ResponseData};

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

/// 构造 reqwest 客户端
fn build_client(config: &RequestConfig) -> Result<Client> {
    let mut builder = ClientBuilder::new();

    // 超时
    let timeout = if config.timeout_secs == 0 {
        Duration::from_secs(30)
    } else {
        Duration::from_secs(config.timeout_secs)
    };
    builder = builder.timeout(timeout);

    // TLS：跳过证书校验
    if config.insecure_skip_verify {
        builder = builder.danger_accept_invalid_certs(true);
    }

    // 重定向
    if !config.follow_redirects {
        builder = builder.redirect(reqwest::redirect::Policy::none());
    }

    // 代理
    if config.use_custom_proxy && !config.proxy.trim().is_empty() {
        let proxy = reqwest::Proxy::all(config.proxy.trim())
            .context("代理地址无效，请使用 http://host:port 格式")?;
        builder = builder.proxy(proxy);
    }

    Ok(builder.build().context("创建 HTTP 客户端失败")?)
}

/// 构造请求头（含认证注入）
fn apply_headers(builder: RequestBuilder, config: &RequestConfig) -> RequestBuilder {
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

/// 执行请求，返回响应数据（无论成功失败都返回 ResponseData，失败时 error 字段填充）
pub fn send(config: &RequestConfig) -> ResponseData {
    let start = Instant::now();
    let client = match build_client(config) {
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
    let mut builder = client.request(method.clone(), &url);
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
                    if let Ok(part) = multipart::Part::file(path) {
                        form = form.part("file", part.file_name(fname));
                    }
                }
            }
            builder = builder.multipart(form);
        }
    }

    // 发送
    let resp_result = builder.send();
    match resp_result {
        Ok(resp) => {
            let status_code = resp.status().as_u16();
            let status_text = resp.status().canonical_reason().unwrap_or("").to_string();
            let http_version = format!("{:?}", resp.version());
            let headers: Vec<(String, String)> = resp
                .headers()
                .iter()
                .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            let body = match resp.bytes() {
                Ok(b) => b.to_vec(),
                Err(_) => Vec::new(),
            };
            ResponseData {
                status_code,
                status_text,
                http_version,
                headers,
                body,
                duration_ms: start.elapsed().as_millis(),
                error: None,
            }
        }
        Err(e) => {
            // 判定是否网络错误（连接失败、TLS 失败、超时等）
            let msg = if e.is_timeout() {
                format!("请求超时（{} 秒）", config.timeout_secs)
            } else if e.is_connect() {
                format!("连接失败：{}", e)
            } else if e.is_builder() {
                format!("请求构建失败：{}", e)
            } else {
                format!("请求失败：{}", e)
            };
            ResponseData {
                error: Some(msg),
                duration_ms: start.elapsed().as_millis(),
                ..Default::default()
            }
        }
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
}
