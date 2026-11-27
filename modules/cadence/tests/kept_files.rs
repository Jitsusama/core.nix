//! Each file cadence keeps is named in one file of its code, so one module
//! reads and writes it. Two files naming it are two readers of one thing,
//! and two readers come to disagree: one learns the file's new shape or
//! place and the other goes on reading the old. So this fails when a kept
//! file's name, a string ending `.json` or `.jsonl`, is named in more than
//! one file of `src` outside its tests. The names more than one file named
//! when it came wait on a list, each with its files, until each is read
//! through one module; a listed name that gains a file, or keeps only one,
//! fails too, so the list stays true and each name leaves it once fixed.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// Cadence's code, read as it stands in the checkout.
const SOURCE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src");

/// Each kept file's name more than one file of `src` names today, with
/// those files, waiting to be read through the one module that keeps it.
const WAITING: &[(&str, &[&str])] = &[
    ("asks.jsonl", &["axes/asks.rs", "hour.rs"]),
    ("collection.json", &["axes/landings.rs", "hour.rs"]),
    (
        "findings.jsonl",
        &[
            "axes/counted.rs",
            "axes/open.rs",
            "axes/readings.rs",
            "daily.rs",
            "hour.rs",
            "owe.rs",
        ],
    ),
    ("landings.json", &["axes/review.rs", "main.rs"]),
    ("notices.json", &["axes/readings.rs", "axes/review.rs"]),
    (
        "repository.json",
        &["axes/readings.rs", "axes/repository.rs"],
    ),
    ("reverted.json", &["hour/net.rs", "main.rs"]),
    ("there.jsonl", &["builders.rs", "hour.rs", "main.rs"]),
];

/// Names different files share, each in a folder of its own, so the files
/// naming one aren't two readers of one thing and wait on nothing. Of
/// `hour.json`, `hour.rs` reads the hour's own, code-health's and
/// pain-points', and `hour/measures/apps.rs` apps-under-load's, which
/// `hour.rs` reads through it.
const APART: &[(&str, &[&str])] = &[("hour.json", &["hour.rs", "hour/measures/apps.rs"])];

/// The names on either list, with the files each is listed with.
fn on_the_lists() -> impl Iterator<Item = &'static (&'static str, &'static [&'static str])> {
    WAITING.iter().chain(APART)
}

#[test]
fn no_kept_file_gains_a_second_reader() {
    let listed: BTreeSet<&str> = on_the_lists().map(|(name, _)| *name).collect();
    let new: Vec<String> = readers(Path::new(SOURCE))
        .into_iter()
        .filter(|(name, files)| files.len() > 1 && !listed.contains(name.as_str()))
        .map(|(name, files)| format!("{name} in {}", joined(&files)))
        .collect();
    assert!(
        new.is_empty(),
        "a kept file is named in more than one file of cadence's src, so two readers of it can \
         come to disagree; read it through the one module that keeps it: {}",
        new.join("; ")
    );
}

#[test]
fn each_listed_name_keeps_the_files_it_is_listed_with() {
    let readers = readers(Path::new(SOURCE));
    let mut wrong = Vec::new();
    for (name, files) in on_the_lists() {
        let listed: BTreeSet<String> = files.iter().map(|file| (*file).to_owned()).collect();
        let found = readers.get(*name).cloned().unwrap_or_default();
        if found == listed {
            continue;
        }
        wrong.push(if found.is_empty() {
            format!("{name} is named in no file now: take it off the list")
        } else if found.len() == 1 {
            format!(
                "{name} is named in {} alone now, so no two files read it: take it off the list",
                joined(&found)
            )
        } else if found.is_superset(&listed) {
            format!(
                "{name} is listed in {} but named in {} too: read it through the one module \
                 that keeps it, rather than listing another reader",
                joined(&listed),
                joined(&found.difference(&listed).cloned().collect())
            )
        } else {
            format!(
                "{name} is listed in {} but named in {}: list the files that name it now",
                joined(&listed),
                joined(&found)
            )
        });
    }
    assert!(
        wrong.is_empty(),
        "the list of shared names is out of date: {}",
        wrong.join("; ")
    );
}

#[test]
fn a_name_is_read_from_literals_as_rust_reads_them() {
    let code = r##"
        // "commented.json" in a line comment
        /* "blocked.json" /* "nested.json" */ still "blocked.json" */
        let quote = '"'; let tick = '\''; fn borrow<'a>(text: &'a str) {}
        let raw = r#"folder/raw.json"#;
        let plain = "folder/plain.jsonl";
        let escaped = "a \"quoted.json\"";
        let prose = "a whole prose.json";
        let template = format!("{date}.json");
        let suffix = ".json";
    "##;
    assert_eq!(
        names(code),
        ["raw.json", "plain.jsonl"],
        "a kept file's name is read from a comment, past a quote in a character, from prose or \
         a template, or missed in a raw string"
    );
}

#[test]
fn test_code_names_no_reader() {
    let code = "fn kept() { open(\"kept.json\") }\n\
                #[cfg(test)]\n\
                mod tests {\n    fn reads() {\n        open(\"test.json\");\n    }\n}\n\
                #[cfg(test)]\n\
                pub(crate) mod stand_in;\n\
                #[cfg(test)]\n\
                #[expect(dead_code, reason = \"a test's\")]\n\
                const ALL: &str = \"all.json\";\n\
                fn after() { open(\"after.json\") }\n";
    let (kept, modules) = outside_tests(code);
    assert_eq!(
        names(&kept),
        ["kept.json", "after.json"],
        "a name in a test module or a test's item counts as a reader, or one past them doesn't"
    );
    assert_eq!(
        modules,
        ["stand_in"],
        "a test module kept in a file of its own isn't found"
    );
    let tests = BTreeSet::from([format!("{}stand_in", folder_of("hour/measures.rs"))]);
    for path in [
        "hour/tests.rs",
        "watch/down/tests/held.rs",
        "hour/measures/stand_in.rs",
        "hour/measures/stand_in/more.rs",
    ] {
        assert!(
            is_test(path, &tests),
            "{path}, a test's file, is read as cadence's code"
        );
    }
    for path in ["hour.rs", "hour/measures/slips.rs", "main.rs"] {
        assert!(
            !is_test(path, &tests),
            "{path}, cadence's code, is left out as a test's file"
        );
    }
}

/// Each kept file's name in cadence's code outside its tests, with the
/// files under `source` naming it.
fn readers(source: &Path) -> BTreeMap<String, BTreeSet<String>> {
    let mut files = Vec::new();
    walk(source, source, &mut files);
    let mut tests = BTreeSet::new();
    let mut code = Vec::new();
    for (path, text) in files {
        let (kept, modules) = outside_tests(&text);
        let folder = folder_of(&path);
        tests.extend(modules.iter().map(|module| format!("{folder}{module}")));
        code.push((path, kept));
    }
    let mut readers: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (path, kept) in code.into_iter().filter(|(path, _)| !is_test(path, &tests)) {
        for name in names(&kept) {
            readers.entry(name).or_default().insert(path.clone());
        }
    }
    readers
}

/// Every Rust file under `folder`, by its path from `root` with `/`
/// between folders, and its text.
fn walk(root: &Path, folder: &Path, files: &mut Vec<(String, String)>) {
    let entries =
        fs::read_dir(folder).unwrap_or_else(|err| panic!("{} unread: {err}", folder.display()));
    for entry in entries {
        let path = entry.expect("a folder's entry").path();
        if path.is_dir() {
            walk(root, &path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let from_root = path.strip_prefix(root).expect("a path under the root");
            let name = from_root.to_str().expect("a path in UTF-8").to_owned();
            let text = fs::read_to_string(&path).unwrap_or_else(|err| panic!("{name}: {err}"));
            files.push((name, text));
        }
    }
}

/// The folder holding the modules a file declares: its own folder for
/// `main.rs`, `lib.rs` or `mod.rs`, else the folder of its name beside it.
fn folder_of(path: &str) -> String {
    let (folder, file) = path.rsplit_once('/').unwrap_or(("", path));
    let folder = if folder.is_empty() {
        String::new()
    } else {
        format!("{folder}/")
    };
    match file {
        "main.rs" | "lib.rs" | "mod.rs" => folder,
        _ => format!("{folder}{}/", file.trim_end_matches(".rs")),
    }
}

/// Whether `path` holds only tests: a `tests.rs`, a file under a `tests`
/// folder, or a module declared only for tests, by its path less `.rs`.
fn is_test(path: &str, tests: &BTreeSet<String>) -> bool {
    let mut parts: Vec<&str> = path.split('/').collect();
    let file = parts.pop();
    file == Some("tests.rs")
        || parts.contains(&"tests")
        || tests.iter().any(|module| {
            path.strip_prefix(module.as_str())
                .is_some_and(|rest| rest == ".rs" || rest.starts_with('/'))
        })
}

/// A file's text less each item marked `#[cfg(test)]`, from the mark to
/// the line closing the item at its own indent, or to the item's own line
/// when it ends there; and the names of the modules so marked that are
/// kept in files of their own.
fn outside_tests(text: &str) -> (String, Vec<String>) {
    let indent = |line: &str| line.len() - line.trim_start().len();
    let mut kept = String::new();
    let mut modules = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if line.trim() != "#[cfg(test)]" {
            kept.push_str(line);
            kept.push('\n');
            continue;
        }
        let Some(item) = lines
            .by_ref()
            .find(|line| !line.trim_start().starts_with("#["))
        else {
            break;
        };
        if item.trim_end().ends_with(';') {
            let mut module = item.split_whitespace().skip_while(|word| *word != "mod");
            if let Some(name) = module.nth(1) {
                modules.push(name.trim_end_matches(';').to_owned());
            }
            continue;
        }
        let at = indent(item);
        for line in lines.by_ref() {
            if indent(line) == at && line.trim_start().starts_with('}') {
                break;
            }
        }
    }
    (kept, modules)
}

/// The text of each string literal in `code`, plain or raw, past its
/// comments and character literals, escapes left as written.
fn literals(code: &str) -> Vec<String> {
    let chars: Vec<char> = code.chars().collect();
    let mut found = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        at = if starts(&chars, at, "//") {
            find(&chars, at, "\n")
        } else if starts(&chars, at, "/*") {
            block_end(&chars, at)
        } else if chars[at] == '\'' {
            character_end(&chars, at)
        } else if chars[at] == '"' {
            let (text, end) = string(&chars, at);
            found.push(text);
            end
        } else {
            at + 1
        };
    }
    found
}

/// Whether `text` is in `chars` at `at`.
fn starts(chars: &[char], at: usize, text: &str) -> bool {
    text.chars()
        .enumerate()
        .all(|(n, c)| chars.get(at + n) == Some(&c))
}

/// Where `text` is first in `chars` from `from` on, or their end.
fn find(chars: &[char], from: usize, text: &str) -> usize {
    (from..chars.len())
        .find(|at| starts(chars, *at, text))
        .unwrap_or(chars.len())
}

/// Just past the block comment opening at `at`, whose comments nest.
fn block_end(chars: &[char], at: usize) -> usize {
    let mut depth = 0_usize;
    let mut end = at;
    while end < chars.len() {
        if starts(chars, end, "/*") {
            depth += 1;
            end += 2;
        } else if starts(chars, end, "*/") {
            depth -= 1;
            end += 2;
            if depth == 0 {
                break;
            }
        } else {
            end += 1;
        }
    }
    end
}

/// Just past the character literal opening at `at`, or past the quote
/// alone when it opens a lifetime or a label.
fn character_end(chars: &[char], at: usize) -> usize {
    if chars.get(at + 1) == Some(&'\\') {
        find(chars, at + 3, "'") + 1
    } else if chars.get(at + 2) == Some(&'\'') {
        at + 3
    } else {
        at + 1
    }
}

/// The text of the string literal whose quote is at `at`, raw when an `r`
/// and its hashes come before it, and where its literal ends.
fn string(chars: &[char], at: usize) -> (String, usize) {
    let hashes = chars[..at].iter().rev().take_while(|c| **c == '#').count();
    let raw = at > hashes && chars[at - 1 - hashes] == 'r';
    let (end, closing) = if raw {
        (
            find(chars, at + 1, &format!("\"{}", "#".repeat(hashes))),
            hashes + 1,
        )
    } else {
        let mut end = at + 1;
        while end < chars.len() && chars[end] != '"' {
            end += if chars[end] == '\\' { 2 } else { 1 };
        }
        (end.min(chars.len()), 1)
    };
    (chars[at + 1..end].iter().collect(), end + closing)
}

/// The kept files' names in `code`'s string literals, in their order.
fn names(code: &str) -> Vec<String> {
    literals(code)
        .iter()
        .filter_map(|literal| kept_name(literal))
        .collect()
}

/// The kept file's name a literal holds: its last part, when that ends in
/// `.json` or `.jsonl` after a stem and is a plain name, not prose, a
/// template or a suffix alone.
fn kept_name(literal: &str) -> Option<String> {
    let name = literal.rsplit('/').next()?;
    let stem = name
        .strip_suffix(".jsonl")
        .or_else(|| name.strip_suffix(".json"))?;
    let plain = |c: char| c.is_ascii_alphanumeric() || "-_.".contains(c);
    (!stem.is_empty() && !stem.starts_with('.') && name.chars().all(plain)).then(|| name.to_owned())
}

/// The files, between commas, as an assert names them.
fn joined(files: &BTreeSet<String>) -> String {
    files
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}
