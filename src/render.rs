use std::collections::HashMap;

use crate::parse::{Segment, VarRef};

/// Renders `segments` against `env`. On success, returns the fully
/// substituted text. On failure, returns **every** missing/required
/// variable's error at once — not just the first one hit — so a
/// template with three missing variables reports all three in one
/// run instead of a frustrating fix-one-rerun-find-the-next loop.
/// A variable that's *set but empty* is treated the same as unset,
/// matching shell `:-`/`:?` parameter-expansion semantics.
pub fn render(segments: &[Segment], env: &HashMap<String, String>) -> Result<String, Vec<String>> {
    let mut out = String::new();
    let mut errors = Vec::new();

    for segment in segments {
        match segment {
            Segment::Literal(text) => out.push_str(text),
            Segment::Ref(VarRef::Required(name)) => match non_empty(env, name) {
                Some(v) => out.push_str(v),
                None => errors.push(format!("{name}: required but not set")),
            },
            Segment::Ref(VarRef::Default(name, default)) => match non_empty(env, name) {
                Some(v) => out.push_str(v),
                None => out.push_str(default),
            },
            Segment::Ref(VarRef::Message(name, message)) => match non_empty(env, name) {
                Some(v) => out.push_str(v),
                None => errors.push(format!("{name}: {message}")),
            },
        }
    }

    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
    }
}

fn non_empty<'a>(env: &'a HashMap<String, String>, name: &str) -> Option<&'a str> {
    env.get(name).map(String::as_str).filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_template;

    fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn substitutes_a_set_required_variable() {
        let segs = parse_template("host=${HOST}");
        let out = render(&segs, &env(&[("HOST", "db.internal")])).unwrap();
        assert_eq!(out, "host=db.internal");
    }

    #[test]
    fn an_unset_required_variable_fails_with_a_clear_message() {
        let segs = parse_template("host=${HOST}");
        let err = render(&segs, &env(&[])).unwrap_err();
        assert_eq!(err, vec!["HOST: required but not set"]);
    }

    #[test]
    fn all_missing_variables_are_reported_together_not_just_the_first() {
        let segs = parse_template("${A} ${B} ${C}");
        let err = render(&segs, &env(&[("B", "set")])).unwrap_err();
        assert_eq!(err.len(), 2);
        assert!(err.iter().any(|e| e.starts_with("A:")));
        assert!(err.iter().any(|e| e.starts_with("C:")));
    }

    #[test]
    fn default_is_used_when_variable_is_unset() {
        let segs = parse_template("port=${PORT:-8080}");
        let out = render(&segs, &env(&[])).unwrap();
        assert_eq!(out, "port=8080");
    }

    #[test]
    fn default_is_used_when_variable_is_set_but_empty() {
        let segs = parse_template("port=${PORT:-8080}");
        let out = render(&segs, &env(&[("PORT", "")])).unwrap();
        assert_eq!(out, "port=8080");
    }

    #[test]
    fn a_set_variable_overrides_its_default() {
        let segs = parse_template("port=${PORT:-8080}");
        let out = render(&segs, &env(&[("PORT", "9090")])).unwrap();
        assert_eq!(out, "port=9090");
    }

    #[test]
    fn message_form_fails_with_the_custom_message_not_a_generic_one() {
        let segs = parse_template("${API_KEY:?must be set in production}");
        let err = render(&segs, &env(&[])).unwrap_err();
        assert_eq!(err, vec!["API_KEY: must be set in production"]);
    }

    #[test]
    fn a_fully_satisfied_template_with_a_mix_of_forms_renders_cleanly() {
        let segs = parse_template("${HOST}:${PORT:-8080}/${PATH_PREFIX:?path prefix is required}");
        let out = render(
            &segs,
            &env(&[("HOST", "api.internal"), ("PATH_PREFIX", "v1")]),
        )
        .unwrap();
        assert_eq!(out, "api.internal:8080/v1");
    }
}
