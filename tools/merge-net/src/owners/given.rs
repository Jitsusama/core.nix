//! How a `checks.nix` calls each VM test: the `import` line a test is
//! given by, and the plain strings the call gives it by name.

use std::collections::BTreeMap;
use std::iter::Peekable;

/// The `import` line a test bound on a line of its own is given by, after
/// the `let` it may open with. The search ends where its value does: a
/// line closing it with `;`, or the next binding of the set, left for the
/// caller to read. Until 20 November it ran on unbounded, so a test built
/// without an import took the next test's and dropped it. Each `let` is
/// closed by its own `in`, so one nested in another's bindings leaves the
/// outer one open.
pub fn called<'a>(lines: &mut Peekable<impl Iterator<Item = &'a str>>) -> Option<&'a str> {
    let mut depth = 0;
    let mut lets = 0_usize;
    while let Some(&line) = lines.peek() {
        let at_rest = depth == 0 && lets == 0;
        if at_rest && line.starts_with("import ") {
            return lines.next();
        }
        if at_rest && line.contains(" =") && !line.starts_with("let") {
            return None;
        }
        lines.next();
        lets += words(line, "let");
        lets = lets.saturating_sub(words(line, "in"));
        depth += nesting(line);
        if depth <= 0 && lets == 0 && code(line).trim_end().ends_with(';') {
            return None;
        }
    }
    None
}

/// Each `vm-` name `checks` binds to a value holding an `import` of a
/// path, read by its layout alone, which `nix fmt` keeps: the value is the
/// binding's own line and each line below it indented deeper. It is what
/// the reading should find, so one it drops is seen.
pub fn importing(checks: &str) -> Vec<String> {
    let lines: Vec<&str> = checks.lines().collect();
    let indent = |line: &str| line.len() - line.trim_start().len();
    let mut found = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        let Some((name, rest)) = line.trim().split_once(" =") else {
            continue;
        };
        if !name.starts_with("vm-") || name.contains(char::is_whitespace) {
            continue;
        }
        let value = std::iter::once(rest).chain(
            lines[at + 1..]
                .iter()
                .take_while(|below| below.trim().is_empty() || indent(below) > indent(line))
                .copied(),
        );
        if value
            .map(code)
            .any(|text| text.contains("import ./") || text.contains("import ../"))
        {
            found.push(name.to_owned());
        }
    }
    found
}

/// How often `word` stands alone in `line`'s code, as a keyword does.
fn words(line: &str, word: &str) -> usize {
    code(line)
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '\''))
        .filter(|part| *part == word)
        .count()
}

/// `line` without its strings' contents or its comment, so a bracket or a
/// word written in either is not read as code. Until 20 November an
/// unclosed bracket in a comment inside a call hid every test after it.
fn code(line: &str) -> String {
    let mut kept = String::new();
    let mut quoted = false;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' if quoted => {
                chars.next();
            }
            '"' => {
                quoted = !quoted;
                kept.push(c);
            }
            '#' if !quoted => break,
            _ if quoted => {}
            _ => kept.push(c),
        }
    }
    kept
}

/// How much deeper `text` leaves a value's sets, lists and brackets. A
/// `${` opens an interpolation, which its own `}` closes.
pub fn nesting(text: &str) -> isize {
    let code = code(text);
    let count = |what: &str| isize::try_from(code.matches(what).count()).unwrap_or(0);
    count("{") + count("[") + count("(") - count("}") - count("]") - count(")")
}

/// The strings a call, opening on `call` and running on through `lines`,
/// gives by name, read only at its own depth: a nested set's `area` is not
/// the call's, nor one inside a list. A line opening a set or a list
/// besides the call's own gives nothing, so a value built from it owns
/// each file it could name. A call holding a `let` or an indented string,
/// whose lines may read as bindings, or one whose set another is merged
/// onto with `//`, gives nothing at all.
pub fn given_by<'a>(
    call: &str,
    lines: &mut Peekable<impl Iterator<Item = &'a str>>,
) -> BTreeMap<String, String> {
    // A `${` opens a value, not a set, and its own `}` closes it.
    let opens = |text: &str| {
        let code = code(text);
        let count = |what: &str| isize::try_from(code.matches(what).count()).unwrap_or(0);
        count("{") + count("[") + count("(") - count("${")
    };
    let unsure = |text: &str| words(text, "let") > 0 || text.contains("''");
    let mut given = BTreeMap::new();
    let mut whole = !unsure(call);
    if opens(call) <= 1 {
        strings_given(call, &mut given);
    }
    let mut at = nesting(call);
    let mut last = call;
    while at > 0 {
        let Some(line) = lines.next() else { break };
        whole &= !unsure(line);
        if at == 1 && opens(line) == 0 {
            strings_given(line, &mut given);
        }
        at += nesting(line);
        last = line;
    }
    let merged = code(last).contains("//")
        || lines
            .peek()
            .is_some_and(|next| code(next).trim_start().starts_with("//"));
    if whole && !merged {
        given
    } else {
        BTreeMap::new()
    }
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
