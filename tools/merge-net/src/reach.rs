//! Which Rust crates read which, by their manifests' path dependencies,
//! so the gate counts what main moved in a crate a branch's crate reads,
//! or one that reads it, as shared, though neither side changed the
//! other's files. A crate builds as one, so two files of the same crate
//! are shared too.
//!
//! On 12 November #3572 changed daemon-serve after its gate ran, while
//! main gained secret-asks's use of it, and #3613 changed slack's daemon
//! after its gate ran, while main moved daemon-serve, which slack reads.
//! Both merged with no run of their own, as main had moved none of their
//! files, and main stopped compiling.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

/// Each crate, by its folder, and the crates it reads.
#[derive(Debug, Default)]
pub struct Crates {
    reads: BTreeMap<String, BTreeSet<String>>,
}

/// What `git grep` is asked for in every manifest: the line every crate
/// has, and each path it names.
const ASKED: [&str; 4] = ["-e", r"^\[package\]", "-e", r#"path *= *""#];

impl Crates {
    /// The crates at `commit`, or none when git can't say.
    #[must_use]
    pub fn at(commit: &str) -> Self {
        let grep = Command::new("git")
            .args(["grep", "-n"])
            .args(ASKED)
            .args([commit, "--", "*Cargo.toml"])
            .output()
            .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
            .unwrap_or_default();
        Self::of(&grep)
    }

    /// The crates `grep` says, its lines as `git grep -n` gives them,
    /// `COMMIT:FILE:LINE:TEXT`. A path naming no crate's folder, such as
    /// a binary's source, is no crate read.
    #[must_use]
    pub fn of(grep: &str) -> Self {
        let mut named = Vec::new();
        let mut reads: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for line in grep.lines() {
            let Some(at) = line.find("Cargo.toml:") else {
                continue;
            };
            let manifest = &line[..at];
            let folder = manifest
                .rsplit_once(':')
                .map_or(manifest, |(_, file)| file)
                .trim_end_matches('/');
            if folder.is_empty() {
                continue;
            }
            reads.entry(folder.to_owned()).or_default();
            if let Some(path) = path_in(&line[at..]) {
                named.push((folder.to_owned(), joined(folder, path)));
            }
        }
        for (folder, read) in named {
            if read != folder && reads.contains_key(&read) {
                reads.entry(folder).or_default().insert(read);
            }
        }
        Self { reads }
    }

    /// The files of `main`, what main moved, in a crate one of the
    /// branch's files is in, reads, or is read by, however far.
    /// Markdown is no crate's code, so it's left out on both sides.
    #[must_use]
    pub fn beside(&self, branch: &[String], main: &[String]) -> Vec<String> {
        let changed: BTreeSet<&str> = branch
            .iter()
            .filter_map(|file| self.crate_of(file))
            .collect();
        main.iter()
            .filter(|file| {
                self.crate_of(file).is_some_and(|moved| {
                    changed.iter().any(|own| {
                        *own == moved || self.reaches(own, moved) || self.reaches(moved, own)
                    })
                })
            })
            .cloned()
            .collect()
    }

    /// The crate `file` is in, its folder the longest that holds it.
    fn crate_of(&self, file: &str) -> Option<&str> {
        let markdown = Path::new(file)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"));
        if markdown {
            return None;
        }
        self.reads
            .keys()
            .filter(|folder| {
                file.strip_prefix(folder.as_str())
                    .is_some_and(|rest| rest.starts_with('/'))
            })
            .max_by_key(|folder| folder.len())
            .map(String::as_str)
    }

    /// Whether `from` reads `to`, itself or through crates it reads.
    fn reaches(&self, from: &str, to: &str) -> bool {
        let mut seen = BTreeSet::new();
        let mut next = vec![from];
        while let Some(at) = next.pop() {
            for read in self.reads.get(at).into_iter().flatten() {
                if read == to {
                    return true;
                }
                if seen.insert(read.as_str()) {
                    next.push(read);
                }
            }
        }
        false
    }
}

/// The path a manifest's line names, `path = "..."`, if it names one.
fn path_in(line: &str) -> Option<&str> {
    let after = &line[line.find("path")? + "path".len()..];
    let quoted = after.trim_start().strip_prefix('=')?.trim_start();
    let inner = quoted.strip_prefix('"')?;
    inner.split_once('"').map(|(path, _)| path)
}

/// `path`, relative to `folder`, from the repository's root.
fn joined(folder: &str, path: &str) -> String {
    let mut parts: Vec<&str> = folder.split('/').collect();
    for part in path.split('/') {
        match part {
            ".." => {
                parts.pop();
            }
            "." | "" => {}
            _ => parts.push(part),
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Manifests as `git grep` says them at a commit: slack reads
    /// daemon-serve, secret-asks reads it too, and sun reads nothing.
    const GREP: &str = "abc:modules/daemon-serve/Cargo.toml:1:[package]\n\
        abc:modules/slack/Cargo.toml:1:[package]\n\
        abc:modules/slack/Cargo.toml:12:daemon-serve = { path = \"../daemon-serve\" }\n\
        abc:modules/slack/Cargo.toml:20:path = \"src/main.rs\"\n\
        abc:modules/quickshell/asks/Cargo.toml:1:[package]\n\
        abc:modules/quickshell/asks/Cargo.toml:9:daemon-serve = { path = \"../../daemon-serve\" }\n\
        abc:modules/sun/Cargo.toml:1:[package]\n";

    fn files(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn what_main_moved_in_a_crate_the_branchs_reads_is_shared() {
        let crates = Crates::of(GREP);
        assert_eq!(
            crates.beside(
                &files(&["modules/slack/src/daemon.rs"]),
                &files(&[
                    "modules/daemon-serve/src/serve.rs",
                    "modules/sun/src/lib.rs"
                ])
            ),
            files(&["modules/daemon-serve/src/serve.rs"]),
            "a crate slack reads moved on main, or one it doesn't read was counted"
        );
    }

    #[test]
    fn what_main_moved_in_a_crate_reading_the_branchs_is_shared() {
        let crates = Crates::of(GREP);
        assert_eq!(
            crates.beside(
                &files(&["modules/daemon-serve/src/serve.rs"]),
                &files(&[
                    "modules/quickshell/asks/src/daemon.rs",
                    "modules/daemon-serve/src/asks.rs",
                    "modules/daemon-serve/AGENTS.md",
                ])
            ),
            files(&[
                "modules/quickshell/asks/src/daemon.rs",
                "modules/daemon-serve/src/asks.rs",
            ]),
            "a crate reading daemon-serve, or another file of daemon-serve, wasn't counted"
        );
    }

    #[test]
    fn crates_that_never_read_each_other_stay_apart() {
        let crates = Crates::of(GREP);
        assert!(
            crates
                .beside(
                    &files(&["modules/slack/src/daemon.rs", "docs/testing.md"]),
                    &files(&["modules/quickshell/asks/src/daemon.rs", "flake.nix"])
                )
                .is_empty(),
            "two crates that only read the same third one were counted as shared"
        );
    }

    #[test]
    fn a_crate_read_through_another_is_reached() {
        let grep = "c:a/Cargo.toml:1:[package]\nc:a/Cargo.toml:5:b = { path = \"../b\" }\n\
            c:b/Cargo.toml:1:[package]\nc:b/Cargo.toml:5:c = { path = \"../c\" }\n\
            c:c/Cargo.toml:1:[package]\n";
        assert_eq!(
            Crates::of(grep).beside(&files(&["a/src/main.rs"]), &files(&["c/src/lib.rs"])),
            files(&["c/src/lib.rs"]),
            "a crate read through another wasn't reached"
        );
    }
}
