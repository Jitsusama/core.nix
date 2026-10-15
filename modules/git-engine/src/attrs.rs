//! The attributes that decide how git turns a file into a blob: text, eol,
//! crlf, filter, ident and working-tree-encoding. Read only for a file whose
//! stat says it may have changed, so a check of its contents means what
//! git's would.

use crate::pattern::Pattern;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    Set,
    Unset,
    Value(String),
}

/// One attributes file's lines: each pattern with what it assigns, and the
/// macros it defines.
#[derive(Default, Clone)]
pub struct File {
    lines: Vec<(Pattern, Assignments)>,
    macros: Vec<(String, Assignments)>,
}

/// What a line sets each attribute to; `None` leaves it unspecified.
type Assignments = Vec<(String, Option<State>)>;

fn assignments<'a>(words: impl Iterator<Item = &'a str>) -> Assignments {
    words
        .map(|word| {
            if let Some(name) = word.strip_prefix('-') {
                (name.to_string(), Some(State::Unset))
            } else if let Some(name) = word.strip_prefix('!') {
                (name.to_string(), None)
            } else if let Some((name, value)) = word.split_once('=') {
                (name.to_string(), Some(State::Value(value.to_string())))
            } else {
                (word.to_string(), Some(State::Set))
            }
        })
        .collect()
}

impl File {
    pub fn parse(text: &[u8], macros_allowed: bool) -> File {
        let mut file = File::default();
        for line in String::from_utf8_lossy(text).lines() {
            let line = line.trim_start();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (pattern, rest) = if let Some(quoted) = line.strip_prefix('"') {
                // A quoted pattern, with C-style escapes for odd names.
                let Some(close) = quoted.find('"') else {
                    continue;
                };
                (unquote(&quoted[..close]), &quoted[close + 1..])
            } else {
                let end = line.find([' ', '\t']).unwrap_or(line.len());
                (line[..end].to_string(), &line[end..])
            };
            let words = rest.split_whitespace();
            if let Some(name) = pattern.strip_prefix("[attr]") {
                if macros_allowed {
                    file.macros.push((name.to_string(), assignments(words)));
                }
                continue;
            }
            // Negated patterns mean nothing in attributes files.
            if pattern.starts_with('!') {
                continue;
            }
            if let Some(parsed) = Pattern::parse(pattern.as_bytes(), false) {
                file.lines.push((parsed, assignments(words)));
            }
        }
        file
    }
}

fn unquote(text: &str) -> String {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'\\' && at + 1 < bytes.len() {
            at += 1;
            match bytes[at] {
                b'n' => out.push(b'\n'),
                b't' => out.push(b'\t'),
                b'0'..=b'7' if at + 2 < bytes.len() => {
                    let octal = std::str::from_utf8(&bytes[at..at + 3])
                        .ok()
                        .and_then(|s| u8::from_str_radix(s, 8).ok());
                    out.push(octal.unwrap_or(bytes[at]));
                    at += 2;
                }
                c => out.push(c),
            }
        } else {
            out.push(bytes[at]);
        }
        at += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The files that apply to one path, lowest precedence first, each with the
/// folder it's read from.
pub fn resolve(files: &[(&[u8], &File)], path: &[u8]) -> HashMap<String, State> {
    let mut macros: HashMap<String, Vec<(String, Option<State>)>> = HashMap::new();
    macros.insert(
        "binary".to_string(),
        vec![
            ("diff".to_string(), Some(State::Unset)),
            ("merge".to_string(), Some(State::Unset)),
            ("text".to_string(), Some(State::Unset)),
        ],
    );
    for (_, file) in files {
        for (name, values) in &file.macros {
            macros.insert(name.clone(), values.clone());
        }
    }
    let mut out: HashMap<String, State> = HashMap::new();
    let apply = |out: &mut HashMap<String, State>, name: &str, state: &Option<State>| match state {
        Some(state) => {
            out.insert(name.to_string(), state.clone());
        }
        None => {
            out.remove(name);
        }
    };
    for (base, file) in files {
        for (pattern, values) in &file.lines {
            if !pattern.matches(path, false, base) {
                continue;
            }
            for (name, state) in values {
                if state == &Some(State::Set)
                    && let Some(expansion) = macros.get(name)
                {
                    for (inner, inner_state) in expansion {
                        apply(&mut out, inner, inner_state);
                    }
                }
                apply(&mut out, name, state);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn later_and_deeper_lines_win() {
        let root = File::parse(b"*.txt text\n*.bin binary\n[attr]mine -text eol=lf\n", true);
        let deep = File::parse(b"*.txt -text\n\"odd name\" filter=lfs\n", false);
        let files: Vec<(&[u8], &File)> = vec![(b"", &root), (b"sub", &deep)];
        let attrs = resolve(&files, b"sub/a.txt");
        assert_eq!(attrs.get("text"), Some(&State::Unset));
        let attrs = resolve(&files, b"a.txt");
        assert_eq!(attrs.get("text"), Some(&State::Set));
        let attrs = resolve(&files, b"x.bin");
        assert_eq!(attrs.get("text"), Some(&State::Unset));
        assert_eq!(attrs.get("diff"), Some(&State::Unset));
        assert_eq!(
            resolve(&files, b"sub/odd name").get("filter"),
            Some(&State::Value("lfs".into()))
        );
    }

    #[test]
    fn macros_expand() {
        let root = File::parse(b"[attr]mine -text eol=lf\n*.c mine\n", true);
        let files: Vec<(&[u8], &File)> = vec![(b"", &root)];
        let attrs = resolve(&files, b"x.c");
        assert_eq!(attrs.get("text"), Some(&State::Unset));
        assert_eq!(attrs.get("eol"), Some(&State::Value("lf".into())));
        assert_eq!(attrs.get("mine"), Some(&State::Set));
    }
}
