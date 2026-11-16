use std::sync::OnceLock;

use jieba_rs::Jieba;
use serde::Serialize;
use unicode_normalization::UnicodeNormalization;

static JIEBA: OnceLock<Jieba> = OnceLock::new();

fn jieba() -> &'static Jieba {
    JIEBA.get_or_init(Jieba::new)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptToken {
    /// UTF-16 offset into the original prompt, matching JavaScript string indices.
    pub start: usize,
    /// UTF-16 end offset (exclusive).
    pub end: usize,
    pub text: String,
    pub term: String,
}

pub fn tokenize_spans(text: &str) -> Vec<PromptToken> {
    let chars: Vec<char> = text.chars().collect();
    let mut offsets = Vec::with_capacity(chars.len() + 1);
    let mut offset = 0;
    for ch in &chars {
        offsets.push(offset);
        offset += ch.len_utf16();
    }
    offsets.push(offset);

    let mut tokens = Vec::new();
    let mut index = 0;
    let mut depth = 0usize;
    let mut latin_start: Option<usize> = None;
    let mut cjk_start: Option<usize> = None;

    while index < chars.len() {
        let ch = chars[index];
        if is_cjk(ch) {
            flush_latin(&chars, &offsets, &mut latin_start, index, &mut tokens);
            if cjk_start.is_none() {
                cjk_start = Some(index);
            }
            index += 1;
            continue;
        }

        flush_cjk(&chars, &offsets, &mut cjk_start, index, &mut tokens);

        if ch.is_ascii_alphanumeric() || ch == '_' || (ch.is_alphanumeric() && !is_cjk(ch)) {
            if latin_start.is_none() {
                latin_start = Some(index);
            }
            index += 1;
            continue;
        }

        flush_latin(&chars, &offsets, &mut latin_start, index, &mut tokens);

        if ch == '\\'
            && index + 1 < chars.len()
            && (chars[index + 1] == '(' || chars[index + 1] == ')')
        {
            if chars[index + 1] == '(' {
                depth += 1;
            } else {
                depth = depth.saturating_sub(1);
            }
            index += 2;
            continue;
        }
        if ch == '(' {
            depth += 1;
            index += 1;
            continue;
        }
        if ch == ')' {
            depth = depth.saturating_sub(1);
            index += 1;
            continue;
        }
        if ch == ':' && depth > 0 {
            let mut cursor = index + 1;
            let mut digits = false;
            let mut dot = false;
            while cursor < chars.len() && (chars[cursor].is_ascii_digit() || chars[cursor] == '.') {
                if chars[cursor] == '.' {
                    if dot {
                        break;
                    }
                    dot = true;
                } else {
                    digits = true;
                }
                cursor += 1;
            }
            if digits && cursor < chars.len() && chars[cursor] == ')' {
                index = cursor;
                continue;
            }
        }
        if ch == '<' {
            if let Some(end) = chars[index..].iter().position(|&item| item == '>') {
                let inner_start = index + 1;
                let inner_end = index + end;
                let inner = &chars[inner_start..inner_end];
                if let Some(first_colon) = inner.iter().position(|&item| item == ':') {
                    let tag: String = inner[..first_colon].iter().collect();
                    if tag.eq_ignore_ascii_case("lora") {
                        let name_start_rel = first_colon + 1;
                        let name_end_rel = inner[name_start_rel..]
                            .iter()
                            .position(|&item| item == ':')
                            .map(|position| name_start_rel + position)
                            .unwrap_or(inner.len());
                        let name_start = inner_start + name_start_rel;
                        let name_end = inner_start + name_end_rel;
                        if name_start < name_end {
                            tokenize_plain(&chars, &offsets, name_start, name_end, &mut tokens);
                        }
                        index = index + end + 1;
                        continue;
                    }
                }
            }
        }
        index += 1;
    }

    flush_latin(&chars, &offsets, &mut latin_start, chars.len(), &mut tokens);
    flush_cjk(&chars, &offsets, &mut cjk_start, chars.len(), &mut tokens);
    tokens.sort_by_key(|token| token.start);
    tokens
}

fn tokenize_plain(
    chars: &[char],
    offsets: &[usize],
    start: usize,
    end: usize,
    out: &mut Vec<PromptToken>,
) {
    let mut index = start;
    let mut latin_start: Option<usize> = None;
    let mut cjk_start: Option<usize> = None;
    while index < end {
        let ch = chars[index];
        if is_cjk(ch) {
            flush_latin(chars, offsets, &mut latin_start, index, out);
            if cjk_start.is_none() {
                cjk_start = Some(index);
            }
        } else if ch.is_ascii_alphanumeric() || ch == '_' || (ch.is_alphanumeric() && !is_cjk(ch)) {
            flush_cjk(chars, offsets, &mut cjk_start, index, out);
            if latin_start.is_none() {
                latin_start = Some(index);
            }
        } else {
            flush_latin(chars, offsets, &mut latin_start, index, out);
            flush_cjk(chars, offsets, &mut cjk_start, index, out);
        }
        index += 1;
    }
    flush_latin(chars, offsets, &mut latin_start, end, out);
    flush_cjk(chars, offsets, &mut cjk_start, end, out);
}

fn flush_latin(
    chars: &[char],
    offsets: &[usize],
    start: &mut Option<usize>,
    end: usize,
    out: &mut Vec<PromptToken>,
) {
    if let Some(start) = start.take() {
        if start < end {
            push_span(chars, offsets, start, end, out);
        }
    }
}

fn flush_cjk(
    chars: &[char],
    offsets: &[usize],
    start: &mut Option<usize>,
    end: usize,
    out: &mut Vec<PromptToken>,
) {
    let Some(start) = start.take() else {
        return;
    };
    if start >= end {
        return;
    }
    let segment: String = chars[start..end].iter().collect();
    for token in jieba().cut(&segment, true) {
        let word = token.word.trim();
        if word.is_empty() || !word.chars().any(|ch| ch.is_alphanumeric()) {
            continue;
        }
        let token_start = start + token.start;
        let token_end = start + token.end;
        if token_start < token_end {
            push_span(chars, offsets, token_start, token_end, out);
        }
    }
}

fn push_span(
    chars: &[char],
    offsets: &[usize],
    start: usize,
    end: usize,
    out: &mut Vec<PromptToken>,
) {
    let text: String = chars[start..end].iter().collect();
    let term = text.nfc().collect::<String>().to_lowercase();
    if term.is_empty() {
        return;
    }
    out.push(PromptToken {
        start: offsets[start],
        end: offsets[end],
        text,
        term,
    });
}

pub fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut latin = String::new();
    let mut cjk = String::new();

    fn push(out: &mut Vec<String>, token: String) {
        if !token.is_empty() {
            out.push(token);
        }
    }

    fn flush_latin(latin: &mut String, out: &mut Vec<String>) {
        if latin.is_empty() {
            return;
        }
        push(out, latin.to_lowercase());
        latin.clear();
    }

    fn flush_cjk(cjk: &mut String, out: &mut Vec<String>) {
        if cjk.is_empty() {
            return;
        }
        let segment = std::mem::take(cjk);
        for token in jieba().cut(&segment, true) {
            let word = token.word.trim();
            if word.is_empty() {
                continue;
            }
            let word = word.to_lowercase();
            if word.chars().any(|ch| ch.is_alphanumeric()) {
                push(out, word);
            }
        }
    }

    for ch in text.chars() {
        if is_cjk(ch) {
            flush_latin(&mut latin, &mut tokens);
            cjk.push(ch);
        } else if ch.is_ascii_alphanumeric() || ch == '_' || (ch.is_alphanumeric() && !is_cjk(ch)) {
            flush_cjk(&mut cjk, &mut tokens);
            latin.push(ch);
        } else {
            flush_latin(&mut latin, &mut tokens);
            flush_cjk(&mut cjk, &mut tokens);
        }
    }
    flush_latin(&mut latin, &mut tokens);
    flush_cjk(&mut cjk, &mut tokens);
    tokens
}

pub fn tokenize_text(text: &str) -> String {
    tokenize(text).join(" ")
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3040..=0x30FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xAC00..=0xD7AF
            | 0xF900..=0xFAFF
            | 0x20000..=0x2FA1F
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_latin_and_keeps_underscores() {
        assert_eq!(
            tokenize("Masterpiece, 1GIRL, red_dress"),
            vec!["masterpiece", "1girl", "red_dress"]
        );
    }

    #[test]
    fn segments_chinese() {
        let tokens = tokenize("一个女孩站在花丛中");
        assert!(tokens.contains(&"女孩".to_string()));
        assert!(tokens.contains(&"花丛".to_string()));
    }

    #[test]
    fn preserves_repeated_tokens() {
        assert_eq!(tokenize("very very red"), vec!["very", "very", "red"]);
    }

    #[test]
    fn spans_latin_tokens_in_original_text() {
        let tokens = tokenize_spans("Masterpiece, red_dress");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].term, "masterpiece");
        assert_eq!(tokens[0].text, "Masterpiece");
        assert_eq!((tokens[0].start, tokens[0].end), (0, 11));
        assert_eq!(tokens[1].term, "red_dress");
        assert_eq!(tokens[1].text, "red_dress");
        assert_eq!((tokens[1].start, tokens[1].end), (13, 22));
    }

    #[test]
    fn spans_chinese_tokens() {
        let tokens = tokenize_spans("一个女孩站在花丛中");
        let girl = tokens
            .iter()
            .find(|token| token.term == "女孩")
            .expect("女孩 token");
        assert_eq!(girl.text, "女孩");
        assert_eq!((girl.start, girl.end), (2, 4));
        assert!(tokens.iter().any(|token| token.term == "花丛"));
    }

    #[test]
    fn skips_weights_and_keeps_lora_name_span() {
        let tokens = tokenize_spans("(masterpiece:1.2), <lora:my_lora:0.8>");
        let terms: Vec<&str> = tokens.iter().map(|token| token.term.as_str()).collect();
        assert_eq!(terms, vec!["masterpiece", "my_lora"]);
        assert_eq!(tokens[0].text, "masterpiece");
        assert_eq!(tokens[1].text, "my_lora");
        assert!(!terms.contains(&"1.2"));
        assert!(!terms.contains(&"0.8"));
        assert!(!terms.contains(&"lora"));
    }

    #[test]
    fn handles_escaped_parentheses() {
        let tokens = tokenize_spans(r"\(masterpiece\)");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "masterpiece");
        assert_eq!(tokens[0].term, "masterpiece");
    }

    #[test]
    fn reports_utf16_offsets() {
        let tokens = tokenize_spans("😀 girl");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "girl");
        assert_eq!((tokens[0].start, tokens[0].end), (3, 7));
    }

    #[test]
    fn keeps_repeated_token_spans() {
        let tokens = tokenize_spans("very very red");
        let terms: Vec<&str> = tokens.iter().map(|token| token.term.as_str()).collect();
        assert_eq!(terms, vec!["very", "very", "red"]);
        assert_eq!((tokens[1].start, tokens[1].end), (5, 9));
    }
}
