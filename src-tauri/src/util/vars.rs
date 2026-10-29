//! 环境变量 `{{var}}` 占位符解析与替换

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// splitmix64：用时间戳 + 计数器混出一个伪随机数（避免为一个随机数引入依赖）
fn random_u64() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut x = nanos ^ COUNTER.fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// 生成 8-4-4-4-12 形式的 UUID（版本位标记为 4）
fn random_uuid() -> String {
    let a = random_u64();
    let b = random_u64();
    let hex = format!("{a:016x}{b:016x}");
    let mut bytes: Vec<u8> = hex.bytes().collect();
    // xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
    bytes[12] = b'4';
    bytes[16] = match bytes[16] {
        b'0' | b'1' | b'2' | b'3' => b'8',
        b'4' | b'5' | b'6' | b'7' => b'9',
        b'8' | b'9' | b'a' | b'b' => b'a',
        _ => b'b',
    };
    let hex: String = bytes.into_iter().map(char::from).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// 内置动态变量：以 `$` 开头，不需要在环境里定义
fn builtin_value(name: &str) -> Option<String> {
    match name {
        "$timestamp" => Some(chrono::Local::now().timestamp().to_string()),
        "$timestampMs" => Some(chrono::Local::now().timestamp_millis().to_string()),
        "$isoTimestamp" => Some(chrono::Local::now().to_rfc3339()),
        "$guid" | "$uuid" => Some(random_uuid()),
        "$randomInt" => Some((random_u64() % 1000).to_string()),
        _ => None,
    }
}

/// 将文本中的 `{{key}}` 替换为环境变量值（`{{$xxx}}` 为内置动态变量）。
/// 未找到的变量原样保留（便于用户感知遗漏）。
pub fn replace_vars(text: &str, vars: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < text.len() {
        // 查找 "{{"
        if bytes[i] == b'{' && text[i..].starts_with("{{") {
            if let Some(end) = text[i + 2..].find("}}") {
                let key_start = i + 2;
                let key_end = i + 2 + end;
                let key = text[key_start..key_end].trim();
                match vars.get(key) {
                    Some(v) => out.push_str(v),
                    None => match builtin_value(key) {
                        Some(v) => out.push_str(&v),
                        None => out.push_str(&text[i..=key_end + 1]),
                    },
                }
                i = key_end + 2;
                continue;
            }
        }
        let ch = text[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// 收集文本中出现的所有变量名（内置 `$` 变量不计入，避免误报未定义）
pub fn collect_var_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < text.len() {
        if bytes[i] == b'{' && text[i..].starts_with("{{") {
            if let Some(end) = text[i + 2..].find("}}") {
                let key = text[i + 2..i + 2 + end].trim();
                if !key.starts_with('$') {
                    names.push(key.to_string());
                }
                i = i + 2 + end + 2;
                continue;
            }
        }
        let ch = text[i..].chars().next().unwrap();
        i += ch.len_utf8();
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("host".to_string(), "example.com".to_string());
        m.insert("token".to_string(), "abc123".to_string());
        m
    }

    #[test]
    fn test_basic_replace() {
        let v = vars();
        assert_eq!(
            replace_vars("https://{{host}}/api?key={{token}}", &v),
            "https://example.com/api?key=abc123"
        );
    }

    #[test]
    fn test_not_found_kept_unchanged() {
        let v = vars();
        assert_eq!(replace_vars("{{missing}}/x", &v), "{{missing}}/x");
    }

    #[test]
    fn test_no_vars() {
        let v = vars();
        assert_eq!(replace_vars("plain text", &v), "plain text");
    }

    #[test]
    fn test_whitespace_inside() {
        let v = HashMap::from([("base url".to_string(), "http://local".to_string())]);
        assert_eq!(replace_vars("{{ base url }}/p", &v), "http://local/p");
    }

    #[test]
    fn test_collect_names() {
        let names = collect_var_names("a={{x}} b={{ y }} c={{x}}");
        assert_eq!(names, vec!["x", "y", "x"]);
    }

    #[test]
    fn test_builtin_vars() {
        let v = HashMap::new();
        let ts = replace_vars("{{$timestamp}}", &v);
        assert!(ts.parse::<i64>().is_ok(), "时间戳应为整数: {ts}");

        let guid = replace_vars("{{$guid}}", &v);
        assert_eq!(guid.len(), 36, "uuid 长度: {guid}");
        assert_eq!(guid.chars().filter(|c| *c == '-').count(), 4);
        assert_ne!(guid, replace_vars("{{$guid}}", &v), "每次应不同");

        let n: u64 = replace_vars("{{$randomInt}}", &v).parse().unwrap();
        assert!(n < 1000);
    }

    #[test]
    fn test_builtin_not_reported_as_missing() {
        assert_eq!(collect_var_names("{{$guid}}"), Vec::<String>::new());
        assert_eq!(collect_var_names("{{$guid}}/{{x}}"), vec!["x"]);
    }
}
