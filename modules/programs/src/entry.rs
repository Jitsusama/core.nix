//! A desktop entry, read as the Desktop Entry Specification (1.5) lays
//! one out, made into a `program` record: every key it names that says
//! what the program is, its file kinds' globs and descriptions from
//! shared-mime-info, and the package its command comes from.

use serde_json::{Value, json};

use crate::mime::Mime;
use crate::package;

/// A file's groups in order, each its name and its keys in order, every
/// value unescaped but lists left whole. A key with a locale, as
/// `Name[fr]`, is left out: Joel reads English.
pub fn groups(text: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut groups: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            groups.push((name.to_owned(), Vec::new()));
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.contains('[') {
            continue;
        }
        if let Some((_, keys)) = groups.last_mut() {
            keys.push((key.to_owned(), value.trim_start().to_owned()));
        }
    }
    groups
}

/// A string value with its escapes read: `\s`, `\n`, `\t`, `\r` and `\\`.
/// A `\;` is kept as it is, for [`list`] to read.
pub fn unescaped(value: &str) -> String {
    let mut said = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            said.push(c);
            continue;
        }
        match chars.next() {
            Some('s') => said.push(' '),
            Some('n') => said.push('\n'),
            Some('t') => said.push('\t'),
            Some('r') => said.push('\r'),
            // A backslash escaped, or one ending the value, is itself.
            Some('\\') | None => said.push('\\'),
            Some(other) => {
                said.push('\\');
                said.push(other);
            }
        }
    }
    said
}

/// A list value's items: split at each `;` not escaped as `\;`, each item
/// unescaped, empty ones left out.
pub fn list(value: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut item = String::new();
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some(';') => item.push(';'),
                Some(other) => {
                    item.push('\\');
                    item.push(other);
                }
                None => item.push('\\'),
            },
            ';' => items.push(std::mem::take(&mut item)),
            _ => item.push(c),
        }
    }
    items.push(item);
    items
        .iter()
        .map(|item| unescaped(item))
        .filter(|item| !item.is_empty())
        .collect()
}

/// An `Exec` value as the arguments it runs: split at spaces outside
/// double quotes, where `\"`, `` \` ``, `\$` and `\\` stand for their
/// letter. Field codes such as `%U` are kept as given.
pub fn argv(exec: &str) -> Vec<String> {
    let exec = unescaped(exec);
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    let mut begun = false;
    let mut chars = exec.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                quoted = !quoted;
                begun = true;
            }
            '\\' if quoted => {
                if let Some(next) = chars.next() {
                    word.push(next);
                }
            }
            ' ' | '\t' if !quoted => {
                if begun {
                    words.push(std::mem::take(&mut word));
                    begun = false;
                }
            }
            _ => {
                word.push(c);
                begun = true;
            }
        }
    }
    if begun {
        words.push(word);
    }
    words
}

/// A key's value in a group, unescaped, or none where it isn't there.
fn said(keys: &[(String, String)], key: &str) -> Option<String> {
    keys.iter()
        .find(|(k, _)| k == key)
        .map(|(_, value)| unescaped(value))
        .filter(|value| !value.is_empty())
}

/// A key's list in a group.
fn listed(keys: &[(String, String)], key: &str) -> Vec<String> {
    keys.iter()
        .find(|(k, _)| k == key)
        .map(|(_, value)| list(value))
        .unwrap_or_default()
}

/// The `program` record of the entry `id`, read from `text` at `path`, or
/// none where it has no `[Desktop Entry]` group or says it's `Hidden`,
/// which the specification counts as deleted.
pub fn record(id: &str, path: &str, text: &str, mime: &mut Mime, search: &str) -> Option<Value> {
    let groups = groups(text);
    let (_, entry) = groups.iter().find(|(name, _)| name == "Desktop Entry")?;
    if said(entry, "Hidden").as_deref() == Some("true") {
        return None;
    }
    let exec = said(entry, "Exec").map(|exec| argv(&exec));
    let actions: Vec<Value> = listed(entry, "Actions")
        .iter()
        .filter_map(|action| {
            let (_, keys) = groups
                .iter()
                .find(|(name, _)| *name == format!("Desktop Action {action}"))?;
            Some(json!({
                "id": action,
                "name": said(keys, "Name"),
                "exec": said(keys, "Exec").map(|exec| argv(&exec)),
            }))
        })
        .collect();
    let types = listed(entry, "MimeType");
    let handles: Vec<&str> = types
        .iter()
        .filter_map(|kind| kind.strip_prefix("x-scheme-handler/"))
        .collect();
    let opens: Vec<Value> = types
        .iter()
        .filter(|kind| !kind.starts_with("x-scheme-handler/"))
        .map(|kind| {
            json!({
                "type": kind,
                "globs": mime.globs(kind),
                "comment": mime.comment(kind),
            })
        })
        .collect();
    let package = exec
        .as_ref()
        .and_then(|argv| argv.first())
        .and_then(|program| package::of(program, search));
    Some(json!({
        "key": format!("program:{id}"),
        "kind": "program",
        "id": id,
        "name": said(entry, "Name"),
        "generic_name": said(entry, "GenericName"),
        "comment": said(entry, "Comment"),
        "categories": listed(entry, "Categories"),
        "keywords": listed(entry, "Keywords"),
        "exec": exec,
        "terminal": said(entry, "Terminal").as_deref() == Some("true"),
        "application": said(entry, "Type").as_deref() == Some("Application"),
        // An entry with no window to open, listed only where another
        // provider gives it acts (program_rows.rs).
        "no_display": said(entry, "NoDisplay").as_deref() == Some("true"),
        "starts_in": said(entry, "Path"),
        "window_class": said(entry, "StartupWMClass"),
        "actions": actions,
        "opens": opens,
        "handles": handles,
        "entry_path": path,
        "package": package,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_splits_at_its_semicolons_but_an_escaped_one() {
        assert_eq!(list("Office;Viewer;"), ["Office", "Viewer"]);
        assert_eq!(list(r"a\;b;c\sd"), ["a;b", "c d"]);
    }

    #[test]
    fn an_exec_splits_at_spaces_outside_its_quotes() {
        assert_eq!(
            argv(r#"kitty --single-instance "--title=A \"B\"" %U"#),
            ["kitty", "--single-instance", r#"--title=A "B""#, "%U"]
        );
        assert_eq!(argv(r"sh -c echo\s1"), ["sh", "-c", "echo", "1"]);
        assert_eq!(argv(r#"prog """#), ["prog", ""]);
    }

    #[test]
    fn a_localized_key_and_a_comment_are_left_out() {
        let groups = groups("# made by hand\n[Desktop Entry]\nName=Files\nName[fr]=Fichiers\n");
        assert_eq!(groups[0].1, [("Name".to_owned(), "Files".to_owned())]);
    }

    #[test]
    fn a_hidden_entry_is_none() {
        let mut mime = Mime::default();
        assert!(
            record(
                "gone",
                "/gone.desktop",
                "[Desktop Entry]\nName=Gone\nHidden=true\n",
                &mut mime,
                ""
            )
            .is_none(),
            "a hidden entry is no program"
        );
    }
}
