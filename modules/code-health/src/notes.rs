//! The folders that don't say what they hold, read from the files alone:
//! the quality standard's rule 1 has every folder carry a short README, or
//! for a folder of one kind of file, the opening comment of its entry file.
//! A folder says what it holds when it has a README.md or AGENTS.md, or
//! when its entry file opens with a comment. Its entry is a file a tool
//! starts from (a module's `home.nix`, a crate's `Cargo.toml`, a `main.rs`),
//! a file named for the folder, the folder's only file, or the file beside
//! it named for it (a Rust module's `serve.rs`, a `backports.nix`).
//!
//! It also passes every folder core's own rule passes
//! (tests/repository/every-folder-says-what-it-holds.sh), so an owner is
//! never sent after a folder that rule already lets through: a `tests`
//! folder says what it holds when the folder it tests does, a crate's
//! `src/bin` when each program in it opens with a comment, any other
//! folder by a line of its parent's note opening with its name, and a top
//! folder by a line of the repository's map. Where several folders of a
//! larger repository are read, the notes above them aren't, so each of
//! those says what it holds only by a note or an entry of its own.
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
    "system.nix",
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
    let tree = Tree { folders, texts };
    let above_a_root = |folder: &str| {
        roots
            .iter()
            .any(|root| folder.is_empty() || root.starts_with(&format!("{folder}/")))
    };
    tree.folders
        .keys()
        .filter(|folder| !folder.split('/').any(|part| part.starts_with('.')))
        .filter(|folder| !above_a_root(folder))
        .filter(|folder| !tree.says(folder))
        .map(|folder| {
            if folder.is_empty() {
                ".".to_owned()
            } else {
                (*folder).to_owned()
            }
        })
        .collect()
}

/// A repository's folders, each by the names of the files right in it,
/// and the texts of the files read.
struct Tree<'a> {
    folders: BTreeMap<&'a str, Vec<&'a str>>,
    texts: HashMap<&'a str, &'a str>,
}

impl Tree<'_> {
    /// Whether a folder, the whole repository's as `""`, says what it
    /// holds, trying the ways in the order core's rule does.
    fn says(&self, folder: &str) -> bool {
        let names = self.folders.get(folder).map_or(&[][..], Vec::as_slice);
        if names.iter().any(|name| is_note(name)) {
            return true;
        }
        if folder.is_empty() {
            return self.entered(folder, names);
        }
        let (parent, last) = folder.rsplit_once('/').unwrap_or(("", folder));
        let told = if parent.is_empty() {
            self.mapped(last)
        } else if last == "tests" {
            self.says(parent)
        } else if folder.ends_with("/src/bin") {
            self.programs_say(folder, names)
        } else {
            self.named_by_parent(parent, last)
        };
        told || self.entered(folder, names)
    }

    /// Whether a folder's entry, or the file beside it of its name, opens
    /// with a comment.
    fn entered(&self, folder: &str, names: &[&str]) -> bool {
        let (parent, last) = folder.rsplit_once('/').unwrap_or(("", folder));
        let inside = |name: &str| {
            if folder.is_empty() {
                name.to_owned()
            } else {
                format!("{folder}/{name}")
            }
        };
        let entered = names.iter().any(|name| {
            let stem = name.rsplit_once('.').map_or(*name, |(stem, _)| stem);
            let entry = RUN_BY_NAME.contains(name)
                || ENTRIES.contains(name)
                || stem == last
                || names.len() == 1;
            entry && self.opens(&inside(name))
        });
        let beside = |extension: &str| {
            if parent.is_empty() {
                format!("{last}.{extension}")
            } else {
                format!("{parent}/{last}.{extension}")
            }
        };
        entered || (!folder.is_empty() && ["rs", "nix"].iter().any(|x| self.opens(&beside(x))))
    }

    /// Whether a file read opens with a comment.
    /// Its first line starting with `#` or `//` is enough, as core's rule
    /// reads it, though in Rust that `#` may open an attribute.
    fn opens(&self, path: &str) -> bool {
        self.texts.get(path).is_some_and(|text| {
            opens_with_comment(path, text) || text.starts_with('#') || text.starts_with("//")
        })
    }

    /// Whether a crate's `src/bin` holds a program, and each of its
    /// programs, its `.rs` files, opens with a `//` comment on its first
    /// line.
    fn programs_say(&self, folder: &str, names: &[&str]) -> bool {
        let programs: Vec<&&str> = names
            .iter()
            .filter(|name| extension(name) == "rs")
            .collect();
        !programs.is_empty()
            && programs.iter().all(|name| {
                self.texts
                    .get(format!("{folder}/{name}").as_str())
                    .is_some_and(|text| text.starts_with("//"))
            })
    }

    /// Whether a line of the parent's README.md or AGENTS.md opens with
    /// the folder's name, as `` - `launcher/`: ``, linked or not.
    fn named_by_parent(&self, parent: &str, last: &str) -> bool {
        self.note_lines(&[
            &format!("{parent}/README.md"),
            &format!("{parent}/AGENTS.md"),
        ])
        .any(|line| opens_with_name(line, last))
    }

    /// Whether a line of the repository's map names a top folder: a
    /// table's row opening `` | `name/ ``, or a tree's line, `name/` and
    /// spaces before a `# ` comment.
    fn mapped(&self, name: &str) -> bool {
        let row = format!("| `{name}/");
        self.note_lines(&["README.md", "AGENTS.md"]).any(|line| {
            line.starts_with(&row)
                || line
                    .strip_prefix(name)
                    .and_then(|rest| rest.strip_prefix('/'))
                    .is_some_and(|rest| {
                        let comment = rest.trim_start_matches(' ');
                        comment.len() < rest.len() && comment.starts_with("# ")
                    })
        })
    }

    /// Every line of those of `notes` that were read.
    fn note_lines<'b>(&'b self, notes: &[&str]) -> impl Iterator<Item = &'b str> {
        let texts: Vec<&'b str> = notes
            .iter()
            .filter_map(|note| self.texts.get(note).copied())
            .collect();
        texts.into_iter().flat_map(str::lines)
    }
}

/// Whether a line opens with a folder's name as core's rule reads one: a
/// bullet's dash or none, the name in backticks with a slash, linked by
/// a reference or a URL or not, then a colon or a space.
fn opens_with_name(line: &str, name: &str) -> bool {
    let rest = line.strip_prefix("- ").unwrap_or(line);
    let rest = rest.strip_prefix('[').unwrap_or(rest);
    let Some(rest) = rest.strip_prefix(&format!("`{name}/`")) else {
        return false;
    };
    let rest = past_link(rest, "][", ']')
        .or_else(|| past_link(rest, "](", ')'))
        .unwrap_or(rest);
    rest.starts_with([':', ' '])
}

/// What follows a link's target, when `rest` opens one with `open` and
/// closes it with `close`.
fn past_link<'a>(rest: &'a str, open: &str, close: char) -> Option<&'a str> {
    rest.strip_prefix(open)
        .and_then(|link| link.find(close).map(|end| &link[end + 1..]))
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
            file("modules/litra/fixtures/a.py", "\"\"\"Proves a.\"\"\"\n"),
            file("modules/litra/fixtures/b.py", "\"\"\"Proves b.\"\"\"\n"),
            file("docs/guide.md", "# A guide\n"),
            file("docs/more.md", "# More\n"),
        ];
        assert_eq!(bare(&files), ["docs", "modules", "modules/litra/fixtures"]);
    }

    #[test]
    fn a_tests_folder_says_what_it_holds_when_its_folder_does() {
        let files = [
            file("README.md", "# A repository\n"),
            file("modules/litra/home.nix", "# litra, the desk light.\n{ }\n"),
            file("modules/litra/tests/a.py", "\n"),
            file("modules/litra/tests/b.py", "\n"),
            file("modules/litra/tests/tests/c.py", "\n"),
            file("modules/litra/tests/tests/d.py", "\n"),
            file("modules/hue/more.nix", "{ }\n"),
            file("modules/hue/other.nix", "{ }\n"),
            file("modules/hue/tests/a.py", "\n"),
            file("modules/hue/tests/b.py", "\n"),
            file("modules/README.md", "# Modules\n"),
        ];
        assert_eq!(
            bare(&files),
            ["modules/hue", "modules/hue/tests"],
            "a tests folder was judged apart from the folder it tests"
        );
    }

    #[test]
    fn a_line_of_its_parents_note_opening_with_its_name_says_what_a_folder_holds() {
        let files = [
            file(
                "tools/README.md",
                concat!(
                    "# Tools\n",
                    "- `bare/`: a bullet.\n",
                    "[`linked/`](linked): a link.\n",
                    "- [`referred/`][1] a reference.\n",
                    "`plain/` opens a paragraph.\n",
                    "- `ended/`\n",
                    "  - `indented/`: too deep.\n",
                    "See `inside/`: not at the start.\n",
                    "- `a.b/`: a dot, as itself.\n",
                ),
            ),
            file("README.md", "# A repository\n"),
            file("tools/bare/a.txt", "\n"),
            file("tools/bare/b.txt", "\n"),
            file("tools/linked/a.txt", "\n"),
            file("tools/linked/b.txt", "\n"),
            file("tools/referred/a.txt", "\n"),
            file("tools/referred/b.txt", "\n"),
            file("tools/plain/a.txt", "\n"),
            file("tools/plain/b.txt", "\n"),
            file("tools/ended/a.txt", "\n"),
            file("tools/ended/b.txt", "\n"),
            file("tools/indented/a.txt", "\n"),
            file("tools/indented/b.txt", "\n"),
            file("tools/inside/a.txt", "\n"),
            file("tools/inside/b.txt", "\n"),
            file("tools/a.b/a.txt", "\n"),
            file("tools/a.b/b.txt", "\n"),
            file("tools/axb/a.txt", "\n"),
            file("tools/axb/b.txt", "\n"),
            file("tools/AGENTS.md", "- `agents/`: from the other note.\n"),
            file("tools/agents/a.txt", "\n"),
            file("tools/agents/b.txt", "\n"),
        ];
        assert_eq!(
            bare(&files),
            ["tools/axb", "tools/ended", "tools/indented", "tools/inside"]
        );
    }

    #[test]
    fn a_line_of_the_map_says_what_a_top_folder_holds() {
        let files = [
            file(
                "README.md",
                concat!(
                    "# A repository\n",
                    "| `tabled/` | a table's row |\n",
                    "drawn/    # a tree's line\n",
                    "cramped/# no space\n",
                    "dashed/   - not a comment\n",
                    "├── deep/   # a branch of the tree\n",
                ),
            ),
            file("AGENTS.md", "agents/  # from the other map\n"),
            file("tabled/a.txt", "\n"),
            file("tabled/b.txt", "\n"),
            file("drawn/a.txt", "\n"),
            file("drawn/b.txt", "\n"),
            file("cramped/a.txt", "\n"),
            file("cramped/b.txt", "\n"),
            file("dashed/a.txt", "\n"),
            file("dashed/b.txt", "\n"),
            file("deep/a.txt", "\n"),
            file("deep/b.txt", "\n"),
            file("agents/a.txt", "\n"),
            file("agents/b.txt", "\n"),
            file("drawn/below/a.txt", "\n"),
            file("drawn/below/b.txt", "\n"),
        ];
        assert_eq!(bare(&files), ["cramped", "dashed", "deep", "drawn/below"]);
    }

    #[test]
    fn a_top_tests_folder_and_a_top_src_bin_are_read_as_top_and_inner_folders() {
        let files = [
            file("README.md", "# A crate\n"),
            file("tests/a.py", "\n"),
            file("tests/b.py", "\n"),
            file("src/main.rs", "//! The main program.\n"),
            file("src/bin/one.rs", "//! One program.\n"),
            file("src/bin/two.rs", "// Two programs.\n"),
        ];
        assert_eq!(
            bare(&files),
            ["src/bin", "tests"],
            "a top tests folder needs the map, and a top src/bin its parent's note"
        );
    }

    #[test]
    fn an_entry_whose_first_line_opens_with_a_hash_says_as_core_reads_it() {
        let files = [
            file("README.md", "# A repository\n"),
            file(
                "attributed/lib.rs",
                "#![forbid(unsafe_code)]\npub fn a() {}\n",
            ),
            file("attributed/more.rs", "\n"),
            file("launched/home.nix", "#!nix\n{ }\n"),
            file("launched/more.nix", "{ }\n"),
            file("spaced/home.nix", "{ }\n# too late\n"),
            file("spaced/more.nix", "{ }\n"),
        ];
        assert_eq!(bare(&files), ["spaced"]);
    }

    #[test]
    fn a_crates_src_bin_says_what_it_holds_when_each_program_opens_with_a_comment() {
        let files = [
            file("README.md", "# A repository\n"),
            file("said/Cargo.toml", "# said: a crate.\n"),
            file("said/src/main.rs", "//! The main program.\n"),
            file("said/src/bin/one.rs", "//! One program.\n"),
            file("said/src/bin/two.rs", "// Two programs.\n"),
            file("half/Cargo.toml", "# half: a crate.\n"),
            file("half/src/main.rs", "//! The main program.\n"),
            file("half/src/bin/one.rs", "//! One program.\n"),
            file("half/src/bin/two.rs", "fn main() {}\n"),
        ];
        assert_eq!(bare(&files), ["half/src/bin"]);
    }

    #[test]
    fn a_file_beside_a_folder_and_of_its_name_says_what_it_holds() {
        let files = [
            file("README.md", "# A repository\n"),
            file("modules/README.md", "# Modules\n"),
            file(
                "modules/backports.nix",
                "# What nixpkgs hasn't taken yet.\n{ }\n",
            ),
            file("modules/backports/one.patch", "\n"),
            file("modules/backports/two.patch", "\n"),
            file("modules/booted/system.nix", "# The system it boots.\n{ }\n"),
            file("modules/booted/two.nix", "{ }\n"),
        ];
        assert!(bare(&files).is_empty(), "{:?}", bare(&files));
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
            file("areas/zone/lab/fixtures/a.tf", "a = 1\n"),
            file("areas/zone/lab/fixtures/b.tf", "b = 1\n"),
            file("areas/zone/live/main.tf", "c = 1\n"),
            file("areas/zone/live/more.tf", "d = 1\n"),
        ];
        let paths: Vec<String> = files.iter().map(|file| file.path.clone()).collect();
        let roots = ["areas/zone/lab".to_owned(), "areas/zone/live".to_owned()];
        assert_eq!(
            without_notes(&paths, &files, &roots),
            ["areas/zone/lab/fixtures", "areas/zone/live"],
            "a folder above the roots was counted, or one in them was missed"
        );
    }
}
