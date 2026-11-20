//! How a `checks.nix` calls each VM test: the `import` line a test is
//! given by, and the plain strings the call gives it by name.

use std::collections::BTreeMap;
use std::iter::Peekable;

/// The `import` line a test bound on a line of its own is given by, after
/// the `let` it may open with. The search ends where its value does: a
/// line closing it with `;`, or the next binding of the set, left for the
/// caller to read. Until 20 November it ran on unbounded, so a test built
/// without an import took the next test's and dropped it.
pub fn called<'a>(lines: &mut Peekable<impl Iterator<Item = &'a str>>) -> Option<&'a str> {
    let mut depth = 0;
    let mut bound = false;
    while let Some(&line) = lines.peek() {
        let at_rest = depth == 0 && !bound;
        if at_rest && line.starts_with("import ") {
            return lines.next();
        }
        if at_rest && line.contains(" =") && !line.starts_with("let") {
            return None;
        }
        lines.next();
        if line == "let" || line.starts_with("let ") {
            bound = true;
        }
        if line == "in" || line.starts_with("in ") {
            bound = false;
        }
        depth += nesting(line);
        if depth <= 0 && !bound && line.ends_with(';') {
            return None;
        }
    }
    None
}

/// How much deeper `text` leaves a value's sets, lists and brackets. A
/// `${` opens an interpolation, which its own `}` closes.
fn nesting(text: &str) -> isize {
    let count = |what: &str| isize::try_from(text.matches(what).count()).unwrap_or(0);
    count("{") + count("[") + count("(") - count("}") - count("]") - count(")")
}

/// The strings a call, opening on `call` and running on through `lines`,
/// gives by name, read only at its own depth: a nested set's `area` is not
/// the call's, nor one inside a list. A line opening a set or a list
/// besides the call's own gives nothing, so a value built from it owns
/// each file it could name.
pub fn given_by<'a>(
    call: &str,
    lines: &mut impl Iterator<Item = &'a str>,
) -> BTreeMap<String, String> {
    let count = |text: &str, what: &str| isize::try_from(text.matches(what).count()).unwrap_or(0);
    // A `${` opens a value, not a set, and its own `}` closes it.
    let opens =
        |text: &str| count(text, "{") + count(text, "[") + count(text, "(") - count(text, "${");
    let mut given = BTreeMap::new();
    if opens(call) <= 1 {
        strings_given(call, &mut given);
    }
    let mut at = nesting(call);
    while at > 0 {
        let Some(line) = lines.next() else { break };
        if at == 1 && opens(line) == 0 {
            strings_given(line, &mut given);
        }
        at += nesting(line);
    }
    given
}

/// Into `given`, each `name = "value";` in `text` whose value is a plain
/// string, built from no other value.
fn strings_given(text: &str, given: &mut BTreeMap<String, String>) {
    let is_name = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    for part in text.split(';') {
        let Some((name, value)) = part.split_once('=') else {
            continue;
        };
        let name = name.rsplit('{').next().unwrap_or_default().trim();
        let value = value.trim();
        let plain = value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .filter(|value| !value.contains(['"', '$', '\\']));
        if let Some(value) = plain
            && !name.is_empty()
            && name.chars().all(is_name)
        {
            given.insert(name.to_owned(), value.to_owned());
        }
    }
}
