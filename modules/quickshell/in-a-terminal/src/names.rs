//! The rows a place holds as zsh's completion offers them: each as a step
//! is typed, its name in lower case with a hyphen for each space, which
//! launcher/Showing.js reads as the name itself, so a step needs no
//! quoting; and what it says, its side or else its count, beside it.

use serde_json::Value;

/// A row's name as a step is typed: `Sessions to Reopen` is
/// `sessions-to-reopen`.
pub fn typed(name: &str) -> String {
    name.split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join("-")
}

/// The rows the shell showed, a line each as zsh's `_describe` reads it:
/// the step, its colons escaped, and what the row says after a colon.
pub fn for_zsh(shown: &Value) -> String {
    let mut out = String::new();
    for row in shown
        .get("rows")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let said = |key: &str| row.get(key).and_then(Value::as_str).unwrap_or_default();
        let step = typed(said("name"));
        if step.is_empty() {
            continue;
        }
        out.push_str(&step.replace(':', "\\:"));
        let what = match said("side") {
            "" => said("count"),
            side => side,
        };
        if !what.is_empty() {
            out.push(':');
            out.push_str(what);
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_typed_in_lower_case_a_hyphen_for_a_space() {
        assert_eq!(typed("Sessions to Reopen"), "sessions-to-reopen");
        assert_eq!(typed("Disk"), "disk");
    }

    #[test]
    fn each_row_is_offered_with_its_side_or_its_count() {
        let shown = serde_json::json!({"rows": [
            {"name": "Disk", "side": "412 GB free"},
            {"name": "Machine", "count": "Disk \u{b7} Power"},
            {"name": "Ratio 16:9"},
            {"name": ""}
        ]});
        assert_eq!(
            for_zsh(&shown),
            "disk:412 GB free\nmachine:Disk \u{b7} Power\nratio-16\\:9\n"
        );
    }
}
