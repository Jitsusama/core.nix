//! The comments naming a path that's gone, read from the files alone: a
//! comment or docstring saying where something lives is a guide, and one
//! left behind when that thing moved sends its reader looking for nothing.
//! A path is named when it starts at one of the repository's top folders,
//! and it's there when it's found from the top or from any folder above
//! the file naming it, since a module's comments name its own `tests/`.
//!
//! It reads generously, so a path it names is gone but a gone path may
//! pass. Only comments are read: a path in a string is often a test's
//! fixture, meant not to be there, and markdown is the docs check's to
//! hold. And a path is gone only below a folder the repository holds
//! under its top one, such as `modules/a/`: `tools/token.c` or `tests/x.nix`
//! is as likely another project's, or a test's stand-in tree, and git's
//! `refs/heads` is no path at all.

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
    let mut gone = Vec::new();
    for file in files {
        for (number, comment) in comments(file) {
            for named in named(comment, &tops) {
                // The folder under the top one the path names.
                let below = named
                    .match_indices('/')
                    .nth(1)
                    .map_or(named, |(at, _)| &named[..at]);
                if there.contains(below) && !found(&file.path, named) {
                    gone.push(format!("{}:{number}: {named}", file.path));
                }
            }
        }
    }
    gone
}

/// A file's comment lines, numbered from one, docstrings included: none
/// for a language this doesn't know the comments of.
fn comments(file: &File) -> Vec<(usize, &str)> {
    let extension = extension(&file.path);
    let mark = match extension {
        "rs" | "js" | "ts" | "qml" | "c" | "h" => "//",
        "nix" | "sh" | "bash" | "py" | "toml" | "yaml" | "yml" => "#",
        _ => return Vec::new(),
    };
    let mut docstring = false;
    let mut lines = Vec::new();
    for (number, line) in file.text.lines().enumerate() {
        let trimmed = line.trim_start();
        let quotes = line.matches("\"\"\"").count() + line.matches("'''").count();
        let comment = if extension == "py"
            && (docstring || trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''"))
        {
            // A docstring closes on its next quotes, or opens and
            // closes on one line.
            docstring = if docstring {
                quotes % 2 == 0
            } else {
                quotes == 1
            };
            true
        } else {
            (trimmed.starts_with(mark) && !trimmed.starts_with("#!"))
                || (extension == "nix" && trimmed.starts_with('*'))
                || (extension == "nix" && trimmed.starts_with("/*"))
        };
        if comment {
            lines.push((number + 1, line));
        }
    }
    lines
}

/// The paths `text` names that start at one of `tops`, each from the start
/// of a word and cut at its last letter, digit or underscore.
fn named<'a>(text: &'a str, tops: &HashSet<&str>) -> Vec<&'a str> {
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
            paths.push(path);
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
