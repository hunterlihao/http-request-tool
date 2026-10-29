//! 请求配置与 cURL 命令互转

use anyhow::{bail, Result};

use crate::http::client::build_url;
use crate::model::request::{AuthType, BodyType, KVPair, Method, RequestConfig};

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

/// 把命令行拆分为参数（支持单/双引号，双引号内支持反斜杠转义）
fn tokenize(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            Some(qc) => {
                if c == qc {
                    quote = None;
                } else if c == '\\' && qc == '"' {
                    if let Some(n) = chars.next() {
                        cur.push(n);
                    }
                } else {
                    cur.push(c);
                }
            }
            None => {
                if c == '\'' || c == '"' {
                    quote = Some(c);
                } else if c.is_whitespace() {
                    if !cur.is_empty() {
                        out.push(std::mem::take(&mut cur));
                    }
                } else {
                    cur.push(c);
                }
            }
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn parse_method(s: &str) -> Method {
    match s.to_uppercase().as_str() {
        "POST" => Method::Post,
        "PUT" => Method::Put,
        "PATCH" => Method::Patch,
        "DELETE" => Method::Delete,
        "HEAD" => Method::Head,
        "OPTIONS" => Method::Options,
        _ => Method::Get,
    }
}

fn header_value<'a>(config: &'a RequestConfig, name: &str) -> Option<&'a str> {
    config
        .headers
        .iter()
        .find(|h| h.enabled && h.key.trim().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}

/// 解析 cURL 命令为请求配置
pub fn parse_curl(input: &str) -> Result<RequestConfig> {
    // 去掉续行反斜杠与换行
    let cleaned = input
        .replace("\\\r\n", " ")
        .replace("\\\n", " ")
        .replace(['\r', '\n'], " ");

    let tokens = tokenize(&cleaned);
    if tokens.is_empty() {
        bail!("命令为空");
    }

    let mut cfg = RequestConfig::default();
    cfg.method = Method::Get;
    let mut method_set = false;
    let mut data: Option<String> = None;
    let mut url: Option<String> = None;

    let mut i = 0;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        let arg = || tokens.get(i + 1).cloned();
        match t {
            "curl" => {}
            "-X" | "--request" => {
                if let Some(v) = arg() {
                    cfg.method = parse_method(&v);
                    method_set = true;
                    i += 1;
                }
            }
            "-H" | "--header" => {
                if let Some(v) = arg() {
                    if let Some((k, val)) = v.split_once(':') {
                        cfg.headers.push(KVPair {
                            key: k.trim().to_string(),
                            value: val.trim().to_string(),
                            enabled: true,
                        });
                    }
                    i += 1;
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-ascii" => {
                if let Some(v) = arg() {
                    data = Some(v);
                    i += 1;
                }
            }
            "-u" | "--user" => {
                if let Some(v) = arg() {
                    if let Some((u, p)) = v.split_once(':') {
                        cfg.auth.auth_type = AuthType::Basic;
                        cfg.auth.username = u.to_string();
                        cfg.auth.password = p.to_string();
                    }
                    i += 1;
                }
            }
            "-F" | "--form" => {
                if let Some(v) = arg() {
                    cfg.body_type = BodyType::Multipart;
                    if let Some((k, val)) = v.split_once('=') {
                        if let Some(path) = val.strip_prefix('@') {
                            cfg.file_path = path.to_string();
                        } else {
                            cfg.form_fields.push(KVPair {
                                key: k.to_string(),
                                value: val.to_string(),
                                enabled: true,
                            });
                        }
                    }
                    i += 1;
                }
            }
            "-k" | "--insecure" => cfg.insecure_skip_verify = true,
            "--max-redirs" => {
                if let Some(v) = arg() {
                    if v == "0" {
                        cfg.follow_redirects = false;
                    }
                    i += 1;
                }
            }
            "-x" | "--proxy" => {
                if let Some(v) = arg() {
                    cfg.proxy = v;
                    cfg.use_custom_proxy = true;
                    i += 1;
                }
            }
            "--url" => {
                if let Some(v) = arg() {
                    url = Some(v);
                    i += 1;
                }
            }
            other if !other.starts_with('-') => {
                if url.is_none() && (other.contains("://") || other.starts_with("http")) {
                    url = Some(other.to_string());
                }
            }
            _ => {}
        }
        i += 1;
    }

    let Some(url) = url else {
        bail!("未找到 URL");
    };
    cfg.url = url;

    if let Some(d) = data {
        // 依据 Content-Type 或内容推断类型
        let looks_json = {
            let s = d.trim_start();
            s.starts_with('{') || s.starts_with('[')
        };
        cfg.body_type = match header_value(&cfg, "content-type") {
            Some(ct) if ct.contains("json") => BodyType::Json,
            Some(ct) if ct.contains("x-www-form-urlencoded") => {
                // 表单：拆分成键值对
                split_form(&d, &mut cfg);
                BodyType::Form
            }
            Some(ct) if ct.contains("text/") => BodyType::Text,
            Some(_) => BodyType::Text,
            None => {
                if looks_json {
                    BodyType::Json
                } else {
                    BodyType::Text
                }
            }
        };
        if cfg.body_type != BodyType::Form {
            cfg.body = d;
        }
    }

    // 有 body 但未显式指定方法时默认 POST
    if !method_set && cfg.body_type != BodyType::None {
        cfg.method = Method::Post;
    }

    Ok(cfg)
}

fn split_form(data: &str, cfg: &mut RequestConfig) {
    for pair in data.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        cfg.form_fields.push(KVPair {
            key: k.to_string(),
            value: v.to_string(),
            enabled: true,
        });
    }
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

    #[test]
    fn test_parse_curl_simple() {
        let cfg = parse_curl(
            "curl -X POST 'https://api.example.com/users' -H 'Content-Type: application/json' -d '{\"name\":\"a\"}'",
        )
        .unwrap();
        assert_eq!(cfg.method, Method::Post);
        assert_eq!(cfg.url, "https://api.example.com/users");
        assert_eq!(cfg.body_type, BodyType::Json);
        assert!(cfg.body.contains("name"));
        assert!(cfg
            .headers
            .iter()
            .any(|h| h.key.eq_ignore_ascii_case("content-type")));
    }

    #[test]
    fn test_parse_curl_auth_and_redirect() {
        let cfg = parse_curl("curl 'https://x.com/a?b=1' \\\n  -u 'user:pass' \\\n  --max-redirs 0")
            .unwrap();
        assert_eq!(cfg.method, Method::Get);
        assert_eq!(cfg.auth.auth_type, AuthType::Basic);
        assert_eq!(cfg.auth.username, "user");
        assert!(!cfg.follow_redirects);
        assert_eq!(cfg.url, "https://x.com/a?b=1");
    }
}
