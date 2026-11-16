//! The folders that don't say what they hold, read from the files alone:
//! the quality standard's rule 1 has every folder carry a short README, or
//! for a folder of one kind of file, the opening comment of its entry file.
//! A folder says what it holds when it has a README.md or AGENTS.md, or
//! when its entry file opens with a comment. Its entry is a file a tool
//! starts from (a module's `home.nix`, a crate's `Cargo.toml`, a `main.rs`),
//! a file named for the folder, the folder's only file, or, for a Rust
//! module's folder, the file beside it named for it.
//!
//! Hidden folders, such as `.github`, are passed over.

use std::collections::{BTreeMap, HashMap};

use crate::measure::{File, RUN_BY_NAME, extension, is_note};

/// How a comment opens, in the languages a folder's entry is written in.
const COMMENTS: &[&str] = &["#", "//", "/*", "\"\"\"", "--", "<!--"];

/// The files a folder is entered from besides those a tool runs by name,
/// whose opening comment can be its note.
const ENTRIES: &[&str] = &[
    "home.nix",
    "nixos.nix",
    "darwin.nix",
    "package.nix",
    "checks.nix",
    "Cargo.toml",
    "pyproject.toml",
];

/// Every folder of `paths` saying nothing of what it holds, the whole
/// repository's as `.`. When `roots` names the folders the paths were read
/// from, as `git::roots` gives them, the folders above those weren't read,
/// so they aren't counted.
pub fn without_notes(paths: &[String], files: &[File], roots: &[String]) -> Vec<String> {
    let texts: HashMap<&str, &str> = files
        .iter()
        .map(|file| (file.path.as_str(), file.text.as_str()))
        .collect();
    // Every folder, by the files right in it.
    let mut folders: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for path in paths {
        let (folder, name) = path.rsplit_once('/').unwrap_or(("", path));
        folders.entry(folder).or_default().push(name);
        let mut above = folder;
        while let Some((parent, _)) = above.rsplit_once('/') {
            folders.entry(parent).or_default();
            above = parent;
        }
        folders.entry("").or_default();
    }
    let opens = |path: &str| {
        texts
            .get(path)
            .is_some_and(|text| opens_with_comment(path, text))
    };
    let above_a_root = |folder: &str| {
        roots
            .iter()
            .any(|root| folder.is_empty() || root.starts_with(&format!("{folder}/")))
    };
    folders
        .into_iter()
        .filter(|(folder, _)| !folder.split('/').any(|part| part.starts_with('.')))
        .filter(|(folder, _)| !above_a_root(folder))
        .filter(|(folder, names)| {
            let inside = |name: &str| {
                if folder.is_empty() {
                    name.to_owned()
                } else {
                    format!("{folder}/{name}")
                }
            };
            let (parent, last) = folder.rsplit_once('/').unwrap_or(("", folder));
            let noted = names.iter().any(|name| is_note(name));
            let entered = names.iter().any(|name| {
                let stem = name.rsplit_once('.').map_or(*name, |(stem, _)| stem);
                let entry = RUN_BY_NAME.contains(name)
                    || ENTRIES.contains(name)
                    || stem == last
                    || names.len() == 1;
                entry && opens(&inside(name))
            });
            let beside = if parent.is_empty() {
                format!("{last}.rs")
            } else {
                format!("{parent}/{last}.rs")
            };
            !(noted || entered || (!folder.is_empty() && opens(&beside)))
        })
        .map(|(folder, _)| {
            if folder.is_empty() {
                ".".to_owned()
            } else {
                folder.to_owned()
            }
        })
        .collect()
}

/// Whether a file's first line, past a `#!` line and blank ones, is a
/// comment; in Rust, where `#` opens an attribute, only `//` or `/*`.
fn opens_with_comment(path: &str, text: &str) -> bool {
    let marks = if extension(path) == "rs" {
        &COMMENTS[1..3]
    } else {
        COMMENTS
    };
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("#!"))
        .is_some_and(|line| marks.iter().any(|mark| line.starts_with(mark)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::file;

    fn bare(files: &[File]) -> Vec<String> {
        let paths: Vec<String> = files.iter().map(|file| file.path.clone()).collect();
        without_notes(&paths, files, &[])
    }

    #[test]
    fn a_folder_with_no_note_and_no_entry_comment_is_bare() {
        let files = [
            file("README.md", "# A repository\n"),
            file("modules/litra/home.nix", "# litra, the desk light.\n{ }\n"),
            file("modules/litra/tests/a.py", "\"\"\"Proves a.\"\"\"\n"),
            file("modules/litra/tests/b.py", "\"\"\"Proves b.\"\"\"\n"),
            file("docs/guide.md", "# A guide\n"),
            file("docs/more.md", "# More\n"),
        ];
        assert_eq!(bare(&files), ["docs", "modules", "modules/litra/tests"]);
    }

    #[test]
    fn an_entry_says_what_its_folder_holds_only_by_an_opening_comment() {
        let files = [
            file("AGENTS.md", "# Map\n"),
            file(
                "said/Cargo.toml",
                "#!x\n\n# said: reads a thing.\n[package]\n",
            ),
            file("unsaid/home.nix", "{ }\n# a comment too late\n"),
            file("named/named.rs", "//! named: what it does.\n"),
            file("named/other.rs", "\n"),
            file("alone/only.sh", "# The one script.\n"),
        ];
        assert_eq!(bare(&files), ["unsaid"]);
    }

    #[test]
    fn a_rust_modules_folder_is_said_by_the_file_beside_it() {
        let files = [
            file("README.md", "# A crate\n"),
            file("src/main.rs", "//! The program.\nmod serve;\n"),
            file("src/serve.rs", "//! Serving, by its parts.\n"),
            file("src/serve/read.rs", "\n"),
            file("src/serve/write.rs", "\n"),
        ];
        assert!(bare(&files).is_empty(), "{:?}", bare(&files));
    }

    #[test]
    fn hidden_folders_and_a_bare_root_are_told_apart() {
        let files = [
            file(".github/one.yml", "a: 1\n"),
            file(".github/two.yml", "a: 2\n"),
            file("a.rs", "\n"),
            file("b.rs", "\n"),
        ];
        assert_eq!(bare(&files), ["."]);
    }

    #[test]
    fn the_folders_above_the_roots_read_are_not_counted() {
        let files = [
            file("areas/zone/lab/README.md", "# The lab\n"),
            file("areas/zone/lab/tests/a.tf", "a = 1\n"),
            file("areas/zone/lab/tests/b.tf", "b = 1\n"),
            file("areas/zone/live/main.tf", "c = 1\n"),
            file("areas/zone/live/more.tf", "d = 1\n"),
        ];
        let paths: Vec<String> = files.iter().map(|file| file.path.clone()).collect();
        let roots = ["areas/zone/lab".to_owned(), "areas/zone/live".to_owned()];
        assert_eq!(
            without_notes(&paths, &files, &roots),
            ["areas/zone/lab/tests", "areas/zone/live"],
            "a folder above the roots was counted, or one in them was missed"
        );
    }
}
