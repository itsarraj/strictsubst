use std::collections::HashMap;

/// A deliberately minimal `.env` parser: `KEY=value` per line, `#`
/// comments, blank lines skipped, one layer of surrounding quotes
/// stripped. See `procrun`'s identical rationale for this crate's own
/// copy — a template-substitution tool doesn't need to be a second
/// full dotenv-format implementation to kept in sync with every edge
/// case, just enough to layer a few extra vars over the real
/// environment for local testing.
pub fn parse_dotenv(text: &str) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let mut value = value.trim();
        if (value.starts_with('"') && value.ends_with('"') && value.len() >= 2)
            || (value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2)
        {
            value = &value[1..value.len() - 1];
        }
        if !key.is_empty() {
            vars.insert(key.to_string(), value.to_string());
        }
    }
    vars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_key_value_pairs() {
        let vars = parse_dotenv("PORT=3000\n");
        assert_eq!(vars.get("PORT"), Some(&"3000".to_string()));
    }

    #[test]
    fn strips_surrounding_quotes() {
        let vars = parse_dotenv("NAME=\"hello world\"\n");
        assert_eq!(vars.get("NAME"), Some(&"hello world".to_string()));
    }

    #[test]
    fn skips_comments_and_blank_lines() {
        let vars = parse_dotenv("\n# comment\nKEY=value\n");
        assert_eq!(vars.len(), 1);
    }
}
