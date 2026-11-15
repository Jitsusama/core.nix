//! The lints a repository's configs turn off, and the strictest settings
//! they leave unset, read from the configs alone: each is a rule its code
//! isn't held to, as the quality standard's rule 4 has every file linted at
//! its strictest. Each is named `config: rule`, whether or not a reason
//! stands beside it, so the count says how far from strictest a repository
//! is, as `suppressions` does for the lines that turn one off.
//!
//! It reads biome's rules and preset, TypeScript's strict flags, ruff's
//! rules, mypy's strict mode, Cargo's `[lints]`, shellcheck's disables, and
//! the clippy lints a Nix file allows with `-A`.

use crate::measure::{File, extension};

/// TypeScript's strictest flags, each of which a tsconfig not extending
/// another must set.
const STRICT_TYPESCRIPT: [&str; 9] = [
    "strict",
    "noUncheckedIndexedAccess",
    "exactOptionalPropertyTypes",
    "noImplicitOverride",
    "noPropertyAccessFromIndexSignature",
    "noFallthroughCasesInSwitch",
    "noImplicitReturns",
    "noUnusedLocals",
    "noUnusedParameters",
];

/// Whether a path is a lint config this reads (a Nix file is read as code).
pub fn is_config(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    matches!(
        name,
        "biome.json"
            | "biome.jsonc"
            | "ruff.toml"
            | ".ruff.toml"
            | "pyproject.toml"
            | "mypy.ini"
            | ".mypy.ini"
            | "Cargo.toml"
            | ".shellcheckrc"
    ) || (name.starts_with("tsconfig") && extension(name) == "json")
}

/// Every lint `files` turn off, and every strictest setting they leave
/// unset, as `config: rule`.
pub fn turned_off(files: &[File]) -> Vec<String> {
    let mut off = Vec::new();
    for file in files {
        let name = file.path.rsplit('/').next().unwrap_or(&file.path);
        let rules = match name {
            "biome.json" | "biome.jsonc" => biome(&file.text),
            "ruff.toml" | ".ruff.toml" => ruff(&entries(&file.text), ""),
            "pyproject.toml" => {
                let entries = entries(&file.text);
                let mut rules = ruff(&entries, "tool.ruff");
                rules.extend(mypy(&entries, "tool.mypy"));
                rules
            }
            "mypy.ini" | ".mypy.ini" => mypy(&entries(&file.text), "mypy"),
            "Cargo.toml" => cargo(&entries(&file.text)),
            ".shellcheckrc" => shellcheck(&file.text),
            _ if name.starts_with("tsconfig") && extension(name) == "json" => {
                typescript(&file.text)
            }
            _ if extension(name) == "nix" => clippy_allowed(&file.text),
            _ => Vec::new(),
        };
        off.extend(
            rules
                .into_iter()
                .map(|rule| format!("{}: {rule}", file.path)),
        );
    }
    off
}

/// A JSON scalar with its keys from the outermost in, as a JSON-with-
/// comments walk finds it: a string's without its quotes, any other's as
/// written.
struct Value {
    keys: Vec<String>,
    value: String,
}

/// Every value of a JSON-with-comments text, comments and trailing commas
/// allowed, as biome's and TypeScript's configs are; an array's items take
/// the array's key.
fn values(text: &str) -> Vec<Value> {
    let mut values = Vec::new();
    let mut keys: Vec<String> = Vec::new();
    let mut open: Vec<Open> = Vec::new();
    let mut key: Option<String> = None;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '/' if chars.peek() == Some(&'/') => {
                chars.by_ref().take_while(|c| *c != '\n').for_each(drop);
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut last = ' ';
                for c in chars.by_ref() {
                    if last == '*' && c == '/' {
                        break;
                    }
                    last = c;
                }
            }
            '"' => {
                let mut string = String::new();
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => string.extend(chars.next()),
                        '"' => break,
                        c => string.push(c),
                    }
                }
                while chars.peek().is_some_and(|c| c.is_whitespace()) {
                    chars.next();
                }
                if chars.peek() == Some(&':') && key.is_none() {
                    chars.next();
                    key = Some(string);
                } else {
                    values.push(value(&keys, key.take().or_else(|| open_key(&open)), string));
                }
            }
            '{' => {
                let under = key.take().or_else(|| open_key(&open));
                open.push(Open::Object(under.is_some()));
                keys.extend(under);
            }
            '[' => {
                let under = key.take().or_else(|| open_key(&open));
                open.push(Open::Array(under));
            }
            '}' | ']' => {
                if let Some(Open::Object(true)) = open.pop() {
                    keys.pop();
                }
            }
            c if c.is_alphanumeric() || c == '-' => {
                let mut word = c.to_string();
                while chars
                    .peek()
                    .is_some_and(|c| c.is_alphanumeric() || "-+.".contains(*c))
                {
                    word.extend(chars.next());
                }
                values.push(value(&keys, key.take().or_else(|| open_key(&open)), word));
            }
            _ => {}
        }
    }
    values
}

/// An object or array a JSON walk is in: an object by whether it pushed
/// its key, an array by its key, which its items take.
enum Open {
    Object(bool),
    Array(Option<String>),
}

/// The key of the array a value sits right in, if it sits in one.
fn open_key(open: &[Open]) -> Option<String> {
    match open.last() {
        Some(Open::Array(key)) => key.clone(),
        _ => None,
    }
}

/// A value found under `keys`, at `key`.
fn value(keys: &[String], key: Option<String>, value: String) -> Value {
    let mut keys = keys.to_vec();
    keys.extend(key);
    Value { keys, value }
}

/// biome's rules set off, by name, and its preset where it isn't all.
fn biome(text: &str) -> Vec<String> {
    let values = values(text);
    let mut off = Vec::new();
    let preset = values
        .iter()
        .any(|value| value.keys.last().is_some_and(|key| key == "preset") && value.value == "all");
    if !preset {
        off.push("preset not all".to_owned());
    }
    for value in &values {
        if value.value != "off" {
            continue;
        }
        // A rule set off with options is off at its `level`, under its name.
        let name = match value.keys.as_slice() {
            [.., rule, level] if level == "level" => rule,
            [.., rule] => rule,
            [] => continue,
        };
        off.push(name.clone());
    }
    off
}

/// TypeScript's strict flags a tsconfig leaves unset or false; none for
/// one extending another, which the one it extends holds.
fn typescript(text: &str) -> Vec<String> {
    let values = values(text);
    let extends = values.iter().any(|value| value.keys == ["extends"]);
    if extends {
        return Vec::new();
    }
    STRICT_TYPESCRIPT
        .iter()
        .filter(|flag| {
            !values.iter().any(|value| {
                value.keys.last().is_some_and(|key| key == *flag) && value.value == "true"
            })
        })
        .map(|flag| (*flag).to_owned())
        .collect()
}

/// A TOML or INI entry: its section, its key unquoted, its whole value as
/// written (an array across lines joined), and the strings quoted in it.
struct Entry {
    section: String,
    key: String,
    value: String,
    quoted: Vec<String>,
}

/// Every entry of a TOML or INI text, comments left out.
fn entries(text: &str) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    let mut section = String::new();
    // How many brackets the last entry's value has left open.
    let mut depth = 0_i32;
    for line in text.lines() {
        let line = uncommented(line).trim();
        if depth > 0 {
            if let Some(entry) = entries.last_mut() {
                entry.value.push(' ');
                entry.value.push_str(line);
                entry.quoted.extend(quoted(line));
            }
            depth += brackets(line);
            continue;
        }
        if line.starts_with('[') && !line.contains('=') {
            line.trim_matches(['[', ']', ' ']).clone_into(&mut section);
        } else if let Some((key, value)) = line.split_once('=') {
            let value = value.trim();
            entries.push(Entry {
                section: section.clone(),
                key: key.trim().trim_matches('"').to_owned(),
                value: value.to_owned(),
                quoted: quoted(value),
            });
            depth = brackets(value);
        }
    }
    entries
}

/// A line before its comment, where no quote before the `#` is open.
fn uncommented(line: &str) -> &str {
    let mut quote: Option<char> = None;
    for (at, c) in line.char_indices() {
        match (c, quote) {
            ('"' | '\'', None) => quote = Some(c),
            (c, Some(open)) if c == open => quote = None,
            ('#' | ';', None) if c == '#' || at == 0 => return &line[..at],
            _ => {}
        }
    }
    line
}

/// The strings quoted in a value.
fn quoted(value: &str) -> Vec<String> {
    value
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// How many brackets a value opens past those it closes.
fn brackets(value: &str) -> i32 {
    let count = |bracket| i32::try_from(value.matches(bracket).count()).unwrap_or(i32::MAX);
    count('[') + count('{') - count(']') - count('}')
}

/// ruff's rules ignored, for every file or some, and its selection where
/// it isn't all; `root` is where ruff's settings sit (`tool.ruff` in a
/// pyproject.toml), and nothing is counted where it has none.
fn ruff(entries: &[Entry], root: &str) -> Vec<String> {
    let lint = if root.is_empty() {
        "lint".to_owned()
    } else {
        format!("{root}.lint")
    };
    let ours = |section: &str| section == root || section == lint;
    let files = |section: &str| {
        section == format!("{lint}.per-file-ignores")
            || section == format!("{root}.per-file-ignores").trim_start_matches('.')
    };
    if !entries
        .iter()
        .any(|entry| ours(&entry.section) || files(&entry.section))
    {
        return Vec::new();
    }
    let mut off = Vec::new();
    let all = entries.iter().any(|entry| {
        ours(&entry.section)
            && matches!(entry.key.as_str(), "select" | "extend-select")
            && entry.quoted.iter().any(|rule| rule == "ALL")
    });
    if !all {
        off.push("select not ALL".to_owned());
    }
    for entry in entries {
        if ours(&entry.section) && matches!(entry.key.as_str(), "ignore" | "extend-ignore") {
            off.extend(entry.quoted.iter().cloned());
        } else if files(&entry.section) {
            off.extend(
                entry
                    .quoted
                    .iter()
                    .map(|rule| format!("{rule} in {}", entry.key)),
            );
        }
    }
    off
}

/// mypy's strict mode where its section (`mypy`, or `tool.mypy` in a
/// pyproject.toml) leaves it off; nothing where it has no section.
fn mypy(entries: &[Entry], section: &str) -> Vec<String> {
    let mut ours = entries
        .iter()
        .filter(|entry| entry.section == section)
        .peekable();
    if ours.peek().is_none() {
        return Vec::new();
    }
    let strict =
        ours.any(|entry| entry.key == "strict" && entry.value.eq_ignore_ascii_case("true"));
    if strict {
        Vec::new()
    } else {
        vec!["strict".to_owned()]
    }
}

/// Each lint a Cargo.toml's `[lints]` or `[workspace.lints]` allows, as
/// `tool::lint` (rustc's as `rust::lint`).
fn cargo(entries: &[Entry]) -> Vec<String> {
    entries
        .iter()
        .filter_map(|entry| {
            let tool = entry
                .section
                .strip_prefix("workspace.")
                .unwrap_or(&entry.section)
                .strip_prefix("lints.")?;
            let allowed = entry.quoted.first().is_some_and(|level| level == "allow");
            allowed.then(|| format!("{tool}::{}", entry.key))
        })
        .collect()
}

/// Each check a .shellcheckrc disables.
fn shellcheck(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| uncommented(line).trim())
        .filter_map(|line| line.strip_prefix("disable="))
        .flat_map(|checks| checks.split(',').map(|check| check.trim().to_owned()))
        .filter(|check| !check.is_empty())
        .collect()
}

/// Each clippy lint a Nix file allows, as a `"-A"` followed by the lint's
/// own string, the way core's clippy.nix lists its flags.
fn clippy_allowed(text: &str) -> Vec<String> {
    let code: String = text.lines().map(uncommented).collect::<Vec<_>>().join("\n");
    let strings = quoted(&code);
    strings
        .windows(2)
        .filter(|pair| pair[0] == "-A" && pair[1].starts_with("clippy::"))
        .map(|pair| pair[1].clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, text: &str) -> File {
        File {
            path: path.to_owned(),
            text: text.to_owned(),
        }
    }

    fn off(path: &str, text: &str) -> Vec<String> {
        turned_off(&[file(path, text)])
    }

    #[test]
    fn biome_counts_each_rule_off_and_a_preset_short_of_all() {
        let text = r#"{
            // a comment naming "noVoid": "off" isn't a rule
            "linter": { "rules": { "preset": "all",
                "style": { "noVoid": "off", "useForOf": "error",
                    "noShadow": { "level": "off", "options": {} } } } }
        }"#;
        assert_eq!(
            off("biome.jsonc", text),
            ["biome.jsonc: noVoid", "biome.jsonc: noShadow"]
        );
        assert_eq!(
            off("a/biome.json", r#"{ "linter": { "enabled": true } }"#),
            ["a/biome.json: preset not all"]
        );
    }

    #[test]
    fn typescript_counts_each_strict_flag_left_unset() {
        let set: Vec<String> = STRICT_TYPESCRIPT
            .iter()
            .filter(|flag| **flag != "noUncheckedIndexedAccess")
            .map(|flag| format!("\"{flag}\": true,"))
            .collect();
        let text = format!(
            "{{ \"compilerOptions\": {{ {} \"noUncheckedIndexedAccess\": false }} }}",
            set.join(" ")
        );
        assert_eq!(
            off("tsconfig.json", &text),
            ["tsconfig.json: noUncheckedIndexedAccess"]
        );
        assert!(
            off("tsconfig.build.json", r#"{ "extends": "./tsconfig.json" }"#).is_empty(),
            "a tsconfig extending another is held by the one it extends"
        );
    }

    #[test]
    fn ruff_counts_each_rule_ignored_and_a_selection_short_of_all() {
        let text = r#"
[lint]
select = ["ALL"]
ignore = [
  # "E501" in a comment ignores nothing
  "D203",
  "T201",
]

[lint.per-file-ignores]
"*-test.py" = ["S101"]
"#;
        assert_eq!(
            off("ruff.toml", text),
            [
                "ruff.toml: D203",
                "ruff.toml: T201",
                "ruff.toml: S101 in *-test.py"
            ]
        );
        let pyproject = "[tool.ruff.lint]\nselect = [\"E\", \"F\"]\n";
        assert_eq!(
            off("pyproject.toml", pyproject),
            ["pyproject.toml: select not ALL"]
        );
    }

    #[test]
    fn mypy_counts_strict_mode_left_off() {
        assert_eq!(
            off("mypy.ini", "[mypy]\npython_version = 3.14\n"),
            ["mypy.ini: strict"]
        );
        assert_eq!(
            off("mypy.ini", "[mypy]\nstrict = True\n"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn cargo_counts_each_lint_allowed() {
        let text = r#"
[package]
name = "allow"

[lints.clippy]
pedantic = { level = "warn", priority = -1 }
must_use_candidate = "allow"
missing_errors_doc = { level = "allow" }
"#;
        assert_eq!(
            off("Cargo.toml", text),
            [
                "Cargo.toml: clippy::must_use_candidate",
                "Cargo.toml: clippy::missing_errors_doc"
            ]
        );
    }

    #[test]
    fn shellcheck_counts_each_check_disabled() {
        assert_eq!(
            off(".shellcheckrc", "# why\ndisable=SC2034,SC1090\n"),
            [".shellcheckrc: SC2034", ".shellcheckrc: SC1090"]
        );
    }

    #[test]
    fn a_nix_file_counts_each_clippy_lint_it_allows() {
        let text = concat!(
            "[\n  \"-W\"\n  \"clippy::pedantic\"\n",
            "  \"-A\"\n  \"clippy::must_use_candidate\"\n]\n# nix-build -A foo\n",
        );
        assert_eq!(
            off("clippy.nix", text),
            ["clippy.nix: clippy::must_use_candidate"]
        );
    }
}
