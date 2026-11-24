//! The comments naming a path that's gone, read from the files alone: a
//! comment or docstring saying where something lives is a guide, and one
//! left behind when that thing moved sends its reader looking for nothing.
//! A path is named when it starts at one of the repository's top folders,
//! and it's there when it's found from the top or from any folder above
//! the file naming it, since a module's comments name its own `tests/`.
//!
//! core's comments-name-what-exists holds the same rule at each change,
//! through `code-health stale` (tree.rs).
//!
//! It reads generously, so a path it names is gone but a gone path may
//! pass. Only comments are read: a path in a string is often a test's
//! fixture, meant not to be there, and markdown is the docs check's to
//! hold. And a path is gone only below a folder the repository holds
//! under its top one, such as `modules/a/`: `tools/token.c` or `tests/x.nix`
//! is as likely another project's, or a test's stand-in tree, and git's
//! `refs/heads` is no path at all. A path cut just before a hyphen, as
//! `tests/vm/screens/screens-` is, names where names begin rather than one
//! file, so it's there while any path begins with it, hyphen and all:
//! `screens.nix` beside it begins only with its letters. It's there too
//! when found whole, the dash then punctuation, as in `home.nix- which`.

use std::collections::HashSet;

use crate::measure::{File, extension};

/// Every comment of `files` naming a path `paths` doesn't hold, as the
/// file, its line and the path named.
pub fn stale(paths: &[String], files: &[File]) -> Vec<String> {
    let mut there: HashSet<&str> = HashSet::new();
    let mut tops: HashSet<&str> = HashSet::new();
    for path in paths {
        there.insert(path);
        let mut folder = path.as_str();
        while let Some((parent, _)) = folder.rsplit_once('/') {
            there.insert(parent);
            folder = parent;
        }
        if path.contains('/') && !folder.starts_with('.') {
            tops.insert(folder);
        }
    }
    let found = |file: &str, named: &str| {
        let mut folder = file;
        there.contains(named)
            || std::iter::from_fn(|| {
                let (parent, _) = folder.rsplit_once('/')?;
                folder = parent;
                Some(parent)
            })
            .any(|folder| there.contains(format!("{folder}/{named}").as_str()))
    };
    // Rare enough that reading every path for each is cheap. Each start
    // keeps the hyphen `named` was cut before.
    let begun = |file: &str, named: &str| {
        let mut folder = file;
        let above = std::iter::from_fn(|| {
            let (parent, _) = folder.rsplit_once('/')?;
            folder = parent;
            Some(format!("{parent}/{named}-"))
        });
        let starts: Vec<String> = std::iter::once(format!("{named}-")).chain(above).collect();
        paths
            .iter()
            .any(|path| starts.iter().any(|start| path.starts_with(start.as_str())))
    };
    let mut gone = Vec::new();
    for file in files {
        for (number, comment) in comments(file) {
            for (named, prefix) in named(comment, &tops) {
                // The folder under the top one the path names.
                let below = named
                    .match_indices('/')
                    .nth(1)
                    .map_or(named, |(at, _)| &named[..at]);
                // A path there whole passes even when a dash follows it
                // as punctuation.
                let here = found(&file.path, named) || (prefix && begun(&file.path, named));
                if there.contains(below) && !here {
                    gone.push(format!("{}:{number}: {named}", file.path));
                }
            }
        }
    }
    gone
}

/// A file's comment lines, numbered from one, block comments and
/// docstrings included: none for a language this doesn't know the
/// comments of.
fn comments(file: &File) -> Vec<(usize, &str)> {
    let extension = extension(&file.path);
    let (mark, blocks) = match extension {
        "rs" | "js" | "ts" | "qml" | "c" | "h" => ("//", true),
        "nix" => ("#", true),
        "sh" | "bash" | "py" | "toml" | "yaml" | "yml" => ("#", false),
        _ => return Vec::new(),
    };
    // The quotes a docstring opened with, which alone close it, while
    // one is open, and the same for a string in code, whose lines aren't
    // comment.
    let mut docstring: Option<&str> = None;
    let mut string: Option<&str> = None;
    let mut block = false;
    // The last line of code, past which a docstring stands only where a
    // statement starts: at a module's top, or after a line ending with a
    // colon, as a def or a class does.
    let mut last: Option<&str> = None;
    let mut lines = Vec::new();
    for (number, line) in file.text.lines().enumerate() {
        let trimmed = line.trim_start();
        let starts = extension == "py"
            && last.is_none_or(|code| scan(code, None).0.trim_end().ends_with(':'));
        let comment = if let Some(quotes) = docstring {
            docstring = scan(line, Some(quotes)).1;
            true
        } else if let Some(quotes) = string {
            string = scan(line, Some(quotes)).1;
            false
        } else if block {
            block = !line.contains("*/");
            true
        } else if starts && opens(trimmed).is_some() {
            // Opened and closed on one line, or open until its quotes
            // come again, unescaped.
            docstring = scan(line, None).1;
            true
        } else if blocks && trimmed.starts_with("/*") {
            block = !trimmed[2..].contains("*/");
            true
        } else {
            let comment = (trimmed.starts_with(mark) && !trimmed.starts_with("#!"))
                || (extension == "nix" && trimmed.starts_with('*'));
            if extension == "py" && !comment {
                string = scan(line, None).1;
            }
            comment
        };
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            last = Some(trimmed);
        }
        if comment {
            lines.push((number + 1, line));
        }
    }
    lines
}

/// A line of Python code read past its strings and their escapes: the
/// code before a `#` outside them, and the triple quotes that close a
/// string it leaves open. `open` holds the quotes of a string the line
/// starts inside, as one the line before left open.
fn scan<'a>(line: &'a str, mut open: Option<&'static str>) -> (&'a str, Option<&'static str>) {
    let mut at = 0;
    while let Some(c) = line[at..].chars().next() {
        let rest = &line[at..];
        if let Some(quotes) = open {
            if c == '\\' {
                at += 1 + rest[1..].chars().next().map_or(0, char::len_utf8);
                continue;
            }
            if rest.starts_with(quotes) {
                at += quotes.len();
                open = None;
                continue;
            }
        } else if c == '#' {
            return (&line[..at], None);
        } else if let Some(quotes) = ["\"\"\"", "'''", "\"", "'"]
            .into_iter()
            .find(|quotes| rest.starts_with(quotes))
        {
            at += quotes.len();
            open = Some(quotes);
            continue;
        }
        at += c.len_utf8();
    }
    // A one-quote string can't run on past its line.
    (line, open.filter(|quotes| quotes.len() == 3))
}

/// The quotes a Python docstring opens with at the start of `trimmed`,
/// past any of the letters that make a string raw, bytes or formatted.
fn opens(trimmed: &str) -> Option<&'static str> {
    let unprefixed = trimmed.trim_start_matches(|c: char| "rRbBfFuU".contains(c));
    if trimmed.len() - unprefixed.len() > 2 {
        return None;
    }
    ["\"\"\"", "'''"]
        .into_iter()
        .find(|quotes| unprefixed.starts_with(quotes))
}

/// The paths `text` names that start at one of `tops`, each from the start
/// of a word and cut at its last letter, digit or underscore, and whether
/// it was cut just before a hyphen, naming where names begin.
fn named<'a>(text: &'a str, tops: &HashSet<&str>) -> Vec<(&'a str, bool)> {
    let inside = |c: char| c.is_ascii_alphanumeric() || "_.@+/-".contains(c);
    let before = |c: char| c.is_alphanumeric() || "_./${}-".contains(c);
    let mut paths = Vec::new();
    let mut rest = 0;
    for (start, _) in text.char_indices() {
        if start < rest || text[..start].chars().next_back().is_some_and(before) {
            continue;
        }
        let length = text[start..]
            .find(|c: char| !inside(c))
            .unwrap_or(text.len() - start);
        let word = &text[start..start + length];
        let path = word.trim_end_matches(|c: char| !(c.is_ascii_alphanumeric() || c == '_'));
        let starts = path
            .split_once('/')
            .is_some_and(|(top, below)| !below.is_empty() && tops.contains(top));
        if starts {
            paths.push((path, word[path.len()..].starts_with('-')));
            rest = start + length;
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::file;

    fn paths(files: &[File]) -> Vec<String> {
        files.iter().map(|file| file.path.clone()).collect()
    }

    #[test]
    fn a_comment_naming_a_path_gone_is_stale() {
        let files = [
            file(
                "modules/a/home.nix",
                "# Read by modules/b/gone.py and modules/b.\n",
            ),
            file("modules/b/home.nix", "{ }\n"),
            file(
                "tools/c.rs",
                "// Moved from modules/b/c.rs, beside modules/a/home.nix.\n",
            ),
        ];
        assert_eq!(
            stale(&paths(&files), &files),
            [
                "modules/a/home.nix:1: modules/b/gone.py",
                "tools/c.rs:1: modules/b/c.rs"
            ],
            "a path gone was missed, or one there was taken for gone"
        );
    }

    #[test]
    fn a_path_is_found_from_any_folder_above_its_file() {
        let files = [
            file(
                "modules/a/src/main.rs",
                "//! Its words are tests/kept/words.json.\n",
            ),
            file("modules/a/tests/kept/words.json", "{}\n"),
            file("tests/kept/a.nix", "{ }\n"),
        ];
        assert!(
            stale(&paths(&files), &files).is_empty(),
            "a path named from the file's own module was taken for gone"
        );
    }

    #[test]
    fn only_comments_and_docstrings_are_read() {
        let files = [
            file(
                "tests/a.sh",
                "#!/usr/bin/env tests/kept/gone\nx=\"tests/kept/none.sh\"\n",
            ),
            file(
                "tests/b.py",
                "\"\"\"Run by\ntests/kept/checks.nix.\n\"\"\"\nx = \"tests/kept/none.py\"\n",
            ),
            file("tests/c.md", "See tests/kept/none.\n"),
            file("tests/d.nix", "/* Beside\n * tests/kept/none.nix */\n{ }\n"),
            file("tests/kept/x.nix", "{ }\n"),
        ];
        assert_eq!(
            stale(&paths(&files), &files),
            [
                "tests/b.py:2: tests/kept/checks.nix",
                "tests/d.nix:2: tests/kept/none.nix"
            ],
            "a string or a shebang was read, or a docstring missed"
        );
    }

    #[test]
    fn block_comments_and_every_docstring_are_read_whole() {
        let files = [
            file(
                "tests/e.py",
                "r\"\"\"Raw, naming\ntests/kept/raw.nix.\n\"\"\"\ndef f():\n    \
                 '''Says \"\"\" once\n    tests/kept/single.nix\n    '''\n    \
                 return \"tests/kept/none.py\"\n",
            ),
            file(
                "modules/w/a.qml",
                "/**\n Beside\n tests/kept/jsdoc.qml\n */\n\
                 Item { source: \"tests/kept/none.qml\" }\n",
            ),
            file(
                "tests/f.nix",
                "/*\n  Lines with no star,\n  tests/kept/bare.nix\n*/\n\
                 { x = \"tests/kept/none.nix\"; }\n",
            ),
            file(
                "tests/h.py",
                "'''Opens, quoting \"\"\"\nand tests/kept/inside.nix\n'''\n\
                 x = \"tests/kept/none.py\"\n",
            ),
            file(
                "tests/i.nix",
                "/* One line, naming tests/kept/one.nix */\n{ x = \"tests/kept/none.nix\"; }\n",
            ),
            file("tests/kept/x.nix", "{ }\n"),
        ];
        assert_eq!(
            stale(&paths(&files), &files),
            [
                "tests/e.py:2: tests/kept/raw.nix",
                "tests/e.py:6: tests/kept/single.nix",
                "modules/w/a.qml:3: tests/kept/jsdoc.qml",
                "tests/f.nix:3: tests/kept/bare.nix",
                "tests/h.py:2: tests/kept/inside.nix",
                "tests/i.nix:1: tests/kept/one.nix"
            ],
            "a raw docstring, one holding the other quotes, or a block's starless line was \
             missed, code past a docstring was read as one, or a one-line block stayed open"
        );
    }

    #[test]
    fn a_docstring_stands_only_where_a_statement_starts() {
        let files = [
            file(
                "tests/g.py",
                "x = \"\"\"\nUsage, as tests/kept/none.py says:\n\"\"\"\ny = 1\ncall(\n    \
                 f\"\"\"tests/kept/none.py\"\"\"\n)\n\n\ndef f():\n    \
                 u\"\"\"One line, naming tests/kept/u.nix.\"\"\"\n    \
                 return \"tests/kept/none.py\"\n\n\nclass K:\n    \
                 Rb\"\"\"Opens here\n    and names tests/kept/rb.nix\n    \"\"\"\n    z = 2\n",
            ),
            file("tests/kept/x.nix", "{ }\n"),
        ];
        assert_eq!(
            stale(&paths(&files), &files),
            [
                "tests/g.py:11: tests/kept/u.nix",
                "tests/g.py:17: tests/kept/rb.nix"
            ],
            "a string's closing quotes or an argument were read as a docstring, a docstring \
             after a def or a class was missed, or a one-line docstring stayed open"
        );
    }

    #[test]
    fn triple_quotes_in_a_string_or_a_comment_open_nothing() {
        let files = [
            file(
                "tests/j.py",
                "QUOTES = ('\"\"\"', \"'''\")\n\
                 # tests/kept/string.nix, past a string's quotes\n\
                 MARK = 1  # three quotes, ''', in a comment open nothing\n\
                 # tests/kept/comment.nix, past a comment's quotes\n\
                 def f(x=\"#\"):\n    \"\"\"Naming tests/kept/hash.nix.\"\"\"\n\
                 s = 'it\\'s \"\"\" here'\n\
                 # tests/kept/escape.nix, past an escaped quote\n",
            ),
            file("tests/kept/x.nix", "{ }\n"),
        ];
        assert_eq!(
            stale(&paths(&files), &files),
            [
                "tests/j.py:2: tests/kept/string.nix",
                "tests/j.py:4: tests/kept/comment.nix",
                "tests/j.py:6: tests/kept/hash.nix",
                "tests/j.py:8: tests/kept/escape.nix"
            ],
            "triple quotes inside a string or a comment, or past an escaped quote, were taken \
             for a string left open, or a def's `#` default hid its docstring"
        );
    }

    #[test]
    fn quotes_are_read_past_a_comment_an_escape_and_a_strings_close() {
        let files = [
            file(
                "tests/k.py",
                "def g():  # a colon, then a comment\n\
                 \x20   \"\"\"Naming tests/kept/colon.nix.\"\"\"\n\
                 def h():\n    \"\"\"An escaped \\\"\"\" stays in, as tests/kept/quoted.nix says,\n\
                 \x20   and tests/kept/after.nix too.\"\"\"\n\
                 x = \"\"\"open\n\
                 closed\"\"\" + \"\"\"again\n\
                 # tests/kept/inside.nix, inside a string\n\
                 \"\"\"\n\
                 # tests/kept/out.nix, past it\n",
            ),
            file("tests/kept/x.nix", "{ }\n"),
        ];
        assert_eq!(
            stale(&paths(&files), &files),
            [
                "tests/k.py:2: tests/kept/colon.nix",
                "tests/k.py:4: tests/kept/quoted.nix",
                "tests/k.py:5: tests/kept/after.nix",
                "tests/k.py:10: tests/kept/out.nix"
            ],
            "a comment past a def's colon hid its docstring, an escaped quote closed one, or \
             a string opened past another's close was missed"
        );
    }

    #[test]
    fn a_path_below_no_folder_held_is_another_trees() {
        let files = [
            file(
                "tools/x/a.rs",
                "// As upstream's tools/token.c, and a stand-in's modules/one/a.\n",
            ),
            file("modules/b/home.nix", "{ }\n"),
            file(
                "refs/x/b.ts",
                "// Not refs/heads/main nor refs/remotes/origin.\n",
            ),
        ];
        assert!(
            stale(&paths(&files), &files).is_empty(),
            "another project's path, a stand-in's or a git ref was taken for gone"
        );
    }

    #[test]
    fn a_path_cut_before_a_hyphen_names_where_names_begin() {
        let files = [
            file(
                "tools/a.rs",
                "// It owned only `tests/vm/screens-`, not tests/vm/gone-.\n\
                 // Nor tests/vm/screens, which no file is.\n",
            ),
            file("tests/vm/gone.nix", "{ }\n"),
            file(
                "modules/b/src/c.rs",
                "// Beside its tests/kept/one- cases, not its tests/kept/two- ones.\n\
                 // Read by tests/kept/x.nix- which is there.\n",
            ),
            file("modules/b/tests/kept/two.nix", "{ }\n"),
            file("modules/b/tests/kept/one-way.nix", "{ }\n"),
            file("tests/kept/x.nix", "{ }\n"),
            file("tests/vm/screens-come-vm-test.py", "\n"),
        ];
        assert_eq!(
            stale(&paths(&files), &files),
            [
                "tools/a.rs:1: tests/vm/gone",
                "tools/a.rs:2: tests/vm/screens",
                "modules/b/src/c.rs:1: tests/kept/two"
            ],
            "a beginning some path has was taken for gone, one only its letters begin, \
             or a whole name some path merely begins with, was missed"
        );
    }

    #[test]
    fn a_path_is_cut_where_its_characters_end() {
        let files = [
            file(
                "tools/a.sh",
                "# See (tools/a.sh), tools/x/c.sh. and `tools/x/b`.\n",
            ),
            file(
                "tools/x/c.sh",
                "# Not my-tools/x/y, ./tools/x/y or $tools/x/y.\n",
            ),
        ];
        assert_eq!(
            stale(&paths(&files), &files),
            ["tools/a.sh:1: tools/x/b"],
            "a path was cut in the wrong place or named mid-word"
        );
    }
}
