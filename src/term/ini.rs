#[derive(Debug)]
pub struct Ini {
    entries: Vec<(String, String, String)>,
}

impl Ini {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut entries = Vec::new();
        let mut section = String::new();
        for (i, raw) in text.lines().enumerate() {
            let n = i + 1;
            let line = raw.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            if let Some(rest) = line.strip_prefix('[') {
                let Some(name) = rest.strip_suffix(']') else {
                    return Err(format!("line {n}: {raw:?} is not a section header"));
                };
                section = name.trim().to_string();
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(format!("line {n}: {raw:?} is neither a section nor a key"));
            };
            entries.push((
                section.clone(),
                key.trim().to_string(),
                unquote(value.trim()),
            ));
        }
        Ok(Self { entries })
    }

    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .rev()
            .find(|(s, k, _)| s == section && k == key)
            .map(|(_, _, v)| v.as_str())
    }

    pub fn extend(&mut self, other: Ini) {
        self.entries.extend(other.entries);
    }
}

/// Quoted values end at the closing quote; bare ones keep every character, `#` included.
fn unquote(value: &str) -> String {
    let mut chars = value.chars();
    let quote = match chars.next() {
        Some(c) if c == '\'' || c == '"' => c,
        _ => return value.to_string(),
    };
    let rest = &value[quote.len_utf8()..];
    match rest.find(quote) {
        Some(end) => rest[..end].to_string(),
        None => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_scope_their_keys() {
        let ini = Ini::parse("[a]\nx = 1\n[b]\nx = 2\n").unwrap();
        assert_eq!(ini.get("a", "x"), Some("1"));
        assert_eq!(ini.get("b", "x"), Some("2"));
    }

    #[test]
    fn quotes_and_comments_come_off() {
        let ini = Ini::parse("; a comment\n# another\n[color]\nname = 'sand'\n").unwrap();
        assert_eq!(ini.get("color", "name"), Some("sand"));
    }

    #[test]
    fn a_line_that_is_neither_fails_with_its_number() {
        let err = Ini::parse("[a]\nx = 1\nnot a key or section\n").unwrap_err();
        assert!(err.starts_with("line 3:"), "{err}");
    }

    #[test]
    fn a_comment_may_follow_a_quoted_value() {
        let ini = Ini::parse("[color]\ngradient_color_1 = '#9a744a'   ; the body\n").unwrap();
        assert_eq!(ini.get("color", "gradient_color_1"), Some("#9a744a"));
    }

    #[test]
    fn a_bare_value_keeps_its_hash() {
        let ini = Ini::parse("[color]\ngradient_color_1 = #9a744a\n").unwrap();
        assert_eq!(ini.get("color", "gradient_color_1"), Some("#9a744a"));
    }

    #[test]
    fn the_last_value_for_a_key_wins() {
        let ini = Ini::parse("[a]\nx = 1\nx = 2\n").unwrap();
        assert_eq!(ini.get("a", "x"), Some("2"));
    }

    #[test]
    fn a_bad_section_header_fails_with_its_number() {
        let err = Ini::parse("[a\nx = 1\n").unwrap_err();
        assert!(err.starts_with("line 1:"), "{err}");
    }

    #[test]
    fn extend_lets_a_later_ini_win() {
        let mut base = Ini::parse("[a]\nx = 1\n").unwrap();
        let over = Ini::parse("[a]\nx = 2\n").unwrap();
        base.extend(over);
        assert_eq!(base.get("a", "x"), Some("2"));
    }
}
