//! 环境变量 `{{var}}` 占位符解析与替换

use std::collections::HashMap;

/// 将文本中的 `{{key}}` 替换为环境变量值。
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
                let key = &text[key_start..key_end];
                match vars.get(key.trim()) {
                    Some(v) => out.push_str(v),
                    None => out.push_str(&text[i..=key_end + 1]),
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
}
