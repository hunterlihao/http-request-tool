//! 把请求配置导出为可复制的 cURL 命令

use crate::http::client::build_url;
use crate::model::request::{AuthType, BodyType, RequestConfig};

/// 单引号包裹（转义内部单引号）
fn q(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn has_header(config: &RequestConfig, name: &str) -> bool {
    config
        .headers
        .iter()
        .filter(|h| h.enabled)
        .any(|h| h.key.trim().eq_ignore_ascii_case(name))
}

/// 生成 cURL 命令
pub fn to_curl(config: &RequestConfig) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push("curl".to_string());
    parts.push("-X".to_string());
    parts.push(config.method.as_str().to_string());
    parts.push(q(&build_url(config)));

    // 用户请求头
    for h in config
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
    {
        parts.push("-H".to_string());
        parts.push(q(&format!("{}: {}", h.key.trim(), h.value)));
    }

    // 认证
    match config.auth.auth_type {
        AuthType::Basic => {
            parts.push("-u".to_string());
            parts.push(q(&format!(
                "{}:{}",
                config.auth.username, config.auth.password
            )));
        }
        AuthType::Bearer => {
            parts.push("-H".to_string());
            parts.push(q(&format!("Authorization: Bearer {}", config.auth.token)));
        }
        AuthType::CustomHeader => {
            if !config.auth.header_name.trim().is_empty() {
                parts.push("-H".to_string());
                parts.push(q(&format!(
                    "{}: {}",
                    config.auth.header_name.trim(),
                    config.auth.header_value
                )));
            }
        }
        AuthType::None => {}
    }

    // 请求体
    match config.body_type {
        BodyType::None => {}
        BodyType::Json | BodyType::Text => {
            if !config.body.trim().is_empty() {
                let ct = if config.body_type == BodyType::Json {
                    "application/json"
                } else {
                    "text/plain"
                };
                if !has_header(config, "content-type") {
                    parts.push("-H".to_string());
                    parts.push(q(&format!("Content-Type: {ct}")));
                }
                parts.push("--data-raw".to_string());
                parts.push(q(&config.body));
            }
        }
        BodyType::Form => {
            let mut has_any = false;
            for f in config
                .form_fields
                .iter()
                .filter(|f| f.enabled && !f.key.is_empty())
            {
                has_any = true;
                parts.push("--data-urlencode".to_string());
                parts.push(q(&format!("{}={}", f.key, f.value)));
            }
            if has_any && !has_header(config, "content-type") {
                parts.push("-H".to_string());
                parts.push(q("Content-Type: application/x-www-form-urlencoded"));
            }
        }
        BodyType::Multipart => {
            for f in config
                .form_fields
                .iter()
                .filter(|f| f.enabled && !f.key.is_empty())
            {
                parts.push("-F".to_string());
                parts.push(q(&format!("{}={}", f.key, f.value)));
            }
            if !config.file_path.trim().is_empty() {
                parts.push("-F".to_string());
                parts.push(q(&format!("file=@{}", config.file_path.trim())));
            }
        }
    }

    // 其他选项
    if config.insecure_skip_verify {
        parts.push("-k".to_string());
    }
    if !config.follow_redirects {
        parts.push("--max-redirs".to_string());
        parts.push("0".to_string());
    }
    if config.use_custom_proxy && !config.proxy.trim().is_empty() {
        parts.push("-x".to_string());
        parts.push(q(config.proxy.trim()));
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::request::{BodyType, Method};

    #[test]
    fn test_curl_basic_get() {
        let cfg = RequestConfig {
            method: Method::Get,
            url: "https://example.com/api".into(),
            ..Default::default()
        };
        let c = to_curl(&cfg);
        assert!(c.starts_with("curl -X GET "));
        assert!(c.contains("'https://example.com/api'"));
    }

    #[test]
    fn test_curl_json_post() {
        let cfg = RequestConfig {
            method: Method::Post,
            url: "https://example.com".into(),
            body_type: BodyType::Json,
            body: "{\"a\":1}".into(),
            ..Default::default()
        };
        let c = to_curl(&cfg);
        assert!(c.contains("-X POST"));
        assert!(c.contains("Content-Type: application/json"));
        assert!(c.contains("--data-raw '{\"a\":1}'"));
    }
}
