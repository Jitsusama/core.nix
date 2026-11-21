//! Which of the harness's files a pi host's own program loads, and when a
//! version carrying a change was first installed. A host resolves its
//! program, `scripts/pi-host.ts`, through pi-packages' `current` once, as
//! it starts, so it keeps the version it started with through an update
//! (core's `host.nix` says so) until it next starts: a harness change to
//! that program runs only once every running host started after it was
//! installed. A conversation's extensions aren't counted, since each
//! session loads its own and reloads onto a new install between runs.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use crate::command;

/// The pi host's program, from which every file it loads is imported.
const PROGRAM: &str = "scripts/pi-host.ts";

/// The files naming the packages the program imports by name, which a
/// change to loads into the next host to start as surely as its own code.
const PACKAGES: &[&str] = &["package.json", "pnpm-lock.yaml"];

/// A relative import as the harness's TypeScript writes one: `from "./x"`,
/// a re-export's `from "../x"`, a bare `import "./x"` or `import("./x")`.
const IMPORT: &str = r#"(from|import) *\(? *"\.\.?/[^"]+""#;

/// How long pi-packages takes from unpacking a version to pointing
/// `current` at it, with room to spare: 19 and 59 seconds on 20 and 21
/// November. A host started in between loaded the version before, so it's
/// read as started before the install, never after.
const INSTALLING: i64 = 5 * 60;

/// The files the host's program loads as the harness's commit `sha` left
/// them, from one grep of `path`; `None` when git can't say.
pub fn files(path: &str, sha: &str) -> Option<BTreeSet<String>> {
    let found = command(
        "git",
        &["-C", path, "grep", "-o", "-E", IMPORT, sha, "--", "*.ts"],
    )
    .ok()?;
    let mut files = imported(&found, sha);
    files.extend(PACKAGES.iter().map(|file| (*file).to_owned()));
    Some(files)
}

/// Every file `PROGRAM` imports, itself and through what it imports, from
/// `found`'s lines, each `SHA:FILE:IMPORT` as `git grep -o` writes them
/// for the commit `sha`.
fn imported(found: &str, sha: &str) -> BTreeSet<String> {
    let mut imports: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let prefix = format!("{sha}:");
    for line in found.lines() {
        let Some((file, import)) = line
            .strip_prefix(&prefix)
            .and_then(|rest| rest.split_once(':'))
        else {
            continue;
        };
        let Some(target) = import.split('"').nth(1).and_then(|to| resolve(file, to)) else {
            continue;
        };
        imports.entry(file).or_default().push(target);
    }
    let mut loaded = BTreeSet::from([PROGRAM.to_owned()]);
    let mut next = VecDeque::from([PROGRAM.to_owned()]);
    while let Some(file) = next.pop_front() {
        for target in imports.get(file.as_str()).into_iter().flatten() {
            if loaded.insert(target.clone()) {
                next.push_back(target.clone());
            }
        }
    }
    loaded
}

/// The file `to`, imported from `file`, from the repository's root; `None`
/// for one that climbs out of it.
fn resolve(file: &str, to: &str) -> Option<String> {
    let mut parts: Vec<&str> = file.split('/').collect();
    parts.pop();
    for part in to.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

/// When a version carrying a change was first installed, in seconds, from
/// the versions pi-packages keeps beside `mirror`, as `carries` says of each
/// by its name: the earliest one's folder's birth, or its last change where
/// the filesystem keeps no birth, and `INSTALLING` more. `None` when no
/// version kept carries it. A version retired since is gone, which only
/// makes the time later, so no host is read as started since when it wasn't.
pub fn first_installed(mirror: &Path, carries: impl Fn(&str) -> bool) -> Option<i64> {
    let kept = std::fs::read_dir(mirror.parent()?.join("versions")).ok()?;
    let versions: Vec<(String, i64)> = kept
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let meta = entry.metadata().ok()?;
            let at = meta.created().or_else(|_| meta.modified()).ok()?;
            let at = at.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
            Some((name, i64::try_from(at).ok()?))
        })
        .collect();
    earliest(&versions, carries)
}

/// The earliest of `versions`, each a name and when it was unpacked, that
/// `carries` says carries a change, and `INSTALLING` more.
fn earliest(versions: &[(String, i64)], carries: impl Fn(&str) -> bool) -> Option<i64> {
    versions
        .iter()
        .filter(|(name, _)| super::is_rev(name) && carries(name))
        .map(|(_, at)| at + INSTALLING)
        .min()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_program_loads_what_it_imports_and_what_that_imports() {
        let found = concat!(
            "f00d:scripts/pi-host.ts:from \"../host/service/program.ts\"\n",
            "f00d:host/service/program.ts:import(\"../screen/screens.ts\"\n",
            "f00d:host/service/program.ts:from \"./rollout.ts\"\n",
            "f00d:host/screen/screens.ts:import \"../../lib/core/x.ts\"\n",
            "f00d:host/screen/screens.ts:from \"../../../outside.ts\"\n",
            "f00d:extensions/quest/index.ts:from \"../../lib/core/y.ts\"\n",
            "beef:host/service/rollout.ts:from \"./elsewhere.ts\"\n",
        );
        assert_eq!(
            imported(found, "f00d"),
            BTreeSet::from(
                [
                    "scripts/pi-host.ts",
                    "host/service/program.ts",
                    "host/screen/screens.ts",
                    "host/service/rollout.ts",
                    "lib/core/x.ts",
                ]
                .map(str::to_owned)
            ),
            "the program's files weren't followed through each import, or an extension, \
             another commit's line or a path out of the repository was taken in"
        );
    }

    #[test]
    fn a_change_was_installed_once_the_first_version_carrying_it_was() {
        let versions = [
            ("2".repeat(40), 300),
            ("1".repeat(40), 100),
            ("3".repeat(40), 200),
            ("current".to_owned(), 50),
        ];
        let after_one = |name: &str| !name.starts_with('1');
        assert_eq!(
            earliest(&versions, after_one),
            Some(200 + INSTALLING),
            "the first version carrying it, by when it was unpacked, wasn't read"
        );
        assert_eq!(
            earliest(&versions, |_| false),
            None,
            "a change no version carries read as installed"
        );
    }
}
