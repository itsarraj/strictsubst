#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarRef {
    /// `$VAR` or `${VAR}` — the strict default: fails if unset or empty.
    Required(String),
    /// `${VAR:-default}` — substitutes `default` if `VAR` is unset or empty.
    Default(String, String),
    /// `${VAR:?message}` — fails with `message` if `VAR` is unset or empty.
    Message(String, String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Literal(String),
    Ref(VarRef),
}

/// Splits `${VAR:-default}`'s or `${VAR:?message}`'s inner text at
/// whichever of `:-`/`:?` appears **first** — matching real shell
/// parameter expansion, where the operator is whatever comes right
/// after the variable name, so a default value that itself happens to
/// contain `:-` or `:?` is taken literally rather than re-parsed.
fn parse_brace_expr(inner: &str) -> VarRef {
    let dash = inner.find(":-");
    let question = inner.find(":?");
    let earliest = match (dash, question) {
        (Some(d), Some(q)) if q < d => Some((q, false)),
        (Some(d), _) => Some((d, true)),
        (None, Some(q)) => Some((q, false)),
        (None, None) => None,
    };
    match earliest {
        Some((idx, is_default)) => {
            let name = inner[..idx].to_string();
            let rest = inner[idx + 2..].to_string();
            if is_default {
                VarRef::Default(name, rest)
            } else {
                VarRef::Message(name, rest)
            }
        }
        None => VarRef::Required(inner.trim().to_string()),
    }
}

/// Parses a template into literal text and variable-reference
/// segments. `$$` is a literal `$` (an escape hatch for content that
/// legitimately contains a dollar sign, like a password). A `$` not
/// followed by a valid reference start (`{`, a letter, or `_`) is left
/// as a literal character, matching real `envsubst`'s behavior rather
/// than treating every `$` as an error.
pub fn parse_template(text: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut literal = String::new();
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '$' {
            literal.push(c);
            continue;
        }
        match chars.peek().copied() {
            Some('$') => {
                chars.next();
                literal.push('$');
            }
            Some('{') => {
                chars.next();
                let mut inner = String::new();
                let mut closed = false;
                for c2 in chars.by_ref() {
                    if c2 == '}' {
                        closed = true;
                        break;
                    }
                    inner.push(c2);
                }
                if !closed {
                    literal.push_str("${");
                    literal.push_str(&inner);
                    continue;
                }
                if !literal.is_empty() {
                    segments.push(Segment::Literal(std::mem::take(&mut literal)));
                }
                segments.push(Segment::Ref(parse_brace_expr(&inner)));
            }
            Some(c2) if c2.is_alphabetic() || c2 == '_' => {
                let mut name = String::new();
                while let Some(&c2) = chars.peek() {
                    if c2.is_alphanumeric() || c2 == '_' {
                        name.push(c2);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if !literal.is_empty() {
                    segments.push(Segment::Literal(std::mem::take(&mut literal)));
                }
                segments.push(Segment::Ref(VarRef::Required(name)));
            }
            _ => literal.push('$'),
        }
    }
    if !literal.is_empty() {
        segments.push(Segment::Literal(literal));
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bare_and_braced_required_refs() {
        let segs = parse_template("host=$HOST port=${PORT}");
        assert_eq!(
            segs,
            vec![
                Segment::Literal("host=".to_string()),
                Segment::Ref(VarRef::Required("HOST".to_string())),
                Segment::Literal(" port=".to_string()),
                Segment::Ref(VarRef::Required("PORT".to_string())),
            ]
        );
    }

    #[test]
    fn parses_default_and_message_forms() {
        let segs = parse_template("${PORT:-8080} ${API_KEY:?must be set in production}");
        assert_eq!(
            segs,
            vec![
                Segment::Ref(VarRef::Default("PORT".to_string(), "8080".to_string())),
                Segment::Literal(" ".to_string()),
                Segment::Ref(VarRef::Message(
                    "API_KEY".to_string(),
                    "must be set in production".to_string()
                )),
            ]
        );
    }

    #[test]
    fn double_dollar_is_a_literal_dollar_sign() {
        let segs = parse_template("price=$$5 var=$FOO");
        assert_eq!(
            segs,
            vec![
                Segment::Literal("price=$5 var=".to_string()),
                Segment::Ref(VarRef::Required("FOO".to_string())),
            ]
        );
    }

    #[test]
    fn a_dollar_not_starting_a_valid_reference_is_literal() {
        let segs = parse_template("cost is $5.00 exactly");
        assert_eq!(
            segs,
            vec![Segment::Literal("cost is $5.00 exactly".to_string())]
        );
    }

    #[test]
    fn an_unterminated_brace_is_kept_literal_rather_than_panicking() {
        let segs = parse_template("oops ${UNCLOSED");
        assert_eq!(segs, vec![Segment::Literal("oops ${UNCLOSED".to_string())]);
    }

    #[test]
    fn a_default_value_containing_the_operator_text_is_taken_literally_after_the_first_split() {
        let segs = parse_template("${VAR:-a:-b}");
        assert_eq!(
            segs,
            vec![Segment::Ref(VarRef::Default(
                "VAR".to_string(),
                "a:-b".to_string()
            ))]
        );
    }

    #[test]
    fn bare_dollar_form_does_not_support_modifiers() {
        // $VAR:-default outside braces is just $VAR followed by
        // literal ":-default" text — real shells behave the same way.
        let segs = parse_template("$VAR:-default");
        assert_eq!(
            segs,
            vec![
                Segment::Ref(VarRef::Required("VAR".to_string())),
                Segment::Literal(":-default".to_string()),
            ]
        );
    }
}
