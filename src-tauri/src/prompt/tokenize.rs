use std::sync::OnceLock;

use jieba_rs::Jieba;

static JIEBA: OnceLock<Jieba> = OnceLock::new();

fn jieba() -> &'static Jieba {
    JIEBA.get_or_init(Jieba::new)
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
}
