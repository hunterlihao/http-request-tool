//! JSON 语法高亮（基于 egui LayoutJob）

use eframe::egui::{Color32, FontId, TextFormat};
use eframe::egui::text::LayoutJob;

const KEY: Color32 = Color32::from_rgb(0x25, 0x63, 0xEB);
const STR: Color32 = Color32::from_rgb(0x05, 0x96, 0x69);
const NUM: Color32 = Color32::from_rgb(0xB4, 0x53, 0x09);
const LIT: Color32 = Color32::from_rgb(0x7C, 0x3A, 0xED);
const PUNC: Color32 = Color32::from_rgb(0x98, 0xA2, 0xB3);
const PLAIN: Color32 = Color32::from_rgb(0x11, 0x18, 0x27);

fn fmt(color: Color32) -> TextFormat {
    TextFormat {
        font_id: FontId::monospace(13.0),
        color,
        ..Default::default()
    }
}

/// 把 JSON 文本转成带颜色的 LayoutJob
pub fn json_job(text: &str) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;

    let bytes = text.as_bytes();
    let mut i = 0;
    while i < text.len() {
        let ch = text[i..].chars().next().unwrap();
        match ch {
            '"' => {
                let start = i;
                i += 1;
                while i < text.len() {
                    let b = bytes[i];
                    if b == b'\\' {
                        // 跳过反斜杠及其后的一个完整字符，避免落在多字节字符中间
                        i += 1;
                        if i < text.len() {
                            let c = text[i..].chars().next().unwrap();
                            i += c.len_utf8();
                        }
                        continue;
                    }
                    if b == b'"' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                let end = i.min(text.len());
                let s = &text[start..end];
                // 向后跳过空白，若紧跟 ':' 则视为 key
                let mut j = end;
                while j < text.len() && (bytes[j] as char).is_ascii_whitespace() {
                    j += 1;
                }
                let is_key = j < text.len() && bytes[j] == b':';
                job.append(s, 0.0, fmt(if is_key { KEY } else { STR }));
                i = end;
            }
            '{' | '}' | '[' | ']' | ',' | ':' => {
                job.append(&ch.to_string(), 0.0, fmt(PUNC));
                i += ch.len_utf8();
            }
            c if c.is_ascii_digit() || c == '-' => {
                let start = i;
                i += 1;
                while i < text.len() {
                    let b = bytes[i];
                    if b.is_ascii_digit()
                        || b == b'.'
                        || b == b'e'
                        || b == b'E'
                        || b == b'+'
                        || b == b'-'
                    {
                        i += 1;
                    } else {
                        break;
                    }
                }
                job.append(&text[start..i], 0.0, fmt(NUM));
            }
            c if c.is_ascii_alphabetic() => {
                let start = i;
                while i < text.len() && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let word = &text[start..i];
                let color = if word == "true" || word == "false" || word == "null" {
                    LIT
                } else {
                    PLAIN
                };
                job.append(word, 0.0, fmt(color));
            }
            _ => {
                job.append(&ch.to_string(), 0.0, fmt(PLAIN));
                i += ch.len_utf8();
            }
        }
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_job_no_panic() {
        let job = json_job("{\n  \"名\": \"值\",\n  \"n\": -1.5e3,\n  \"b\": true,\n  \"x\": null\n}");
        assert!(job.sections.len() > 5);
    }

    #[test]
    fn test_json_job_escaped() {
        // 含转义与中文，确保不 panic
        let _ = json_job("{\"a\":\"x\\\"y\",\"中文\":\"测试\"}");
    }
}
