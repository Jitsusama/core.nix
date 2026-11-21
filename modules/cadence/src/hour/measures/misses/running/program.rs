//! Which of the harness's files a pi host's own program loads, and when
//! pi-packages last pointed `current` at the version installed. A host,
//! and each conversation's own process beside it, resolves its program,
//! `scripts/pi-host.ts`, through `current` once, as it starts, so it keeps
//! the version it started with through an update (core's `host.nix` says
//! so) until it next starts: a harness change to that program runs only
//! once every running host and conversation started after it was
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

/// A relative import as the harness writes one: `from "./x"`, a
/// re-export's or a split import's `from "../x"`, a bare `import "./x"` or
/// `import("./x")`, in either quote; or any relative path to a file named
/// in quotes, as a script it runs on a thread or in a process of its own,
/// or a schema or query it reads, by `new URL("./schema.sql",
/// import.meta.url)`, on one line or split over two. A path written only
/// in a comment counts too, which errs toward a change waiting on the
/// hosts.
const IMPORT: &str =
    r#"(from|import) *\(? *["']\.\.?/[^"']+["']|["']\.\.?/[^"'[:space:]]+\.[A-Za-z0-9]+["']"#;

/// The files that import, as the harness names its scripts.
const SCRIPTS: &[&str] = &["*.ts", "*.mts", "*.cts", "*.tsx", "*.js", "*.mjs", "*.cjs"];

/// The files the host's program loads as the harness's commit `sha` left
/// them, from one grep of `path`; `None` when git can't say, the program
/// imports nothing, as when it moved, or it names a file the commit
/// doesn't hold, as a `./x.js` written for `x.ts` or a folder would: a
/// walk that can't follow each step would read a change past it as
/// loading nowhere. A load by a path made at run time isn't followed:
/// the harness makes one only for a session's extensions, which aren't
/// counted, and a worker's own code, written inline.
pub fn files(path: &str, sha: &str) -> Option<BTreeSet<String>> {
    let mut args = vec!["-C", path, "grep", "--null", "-o", "-E", IMPORT, sha, "--"];
    args.extend(SCRIPTS);
    let found = command("git", &args).ok()?;
    // Split on NUL, as the grep writes names, since git quotes a line's odd one.
    let held = command(
        "git",
        &["-C", path, "ls-tree", "-r", "-z", "--name-only", sha],
    )
    .ok()?;
    let mut files = imported(&found, sha, &held.split('\0').collect())?;
    files.extend(PACKAGES.iter().map(|file| (*file).to_owned()));
    Some(files)
}

/// Every file `PROGRAM` imports, itself and through what it imports, from
/// `found`'s lines, each `SHA:FILE`, a NUL, then the import, as
/// `git grep --null -o` writes them for the commit `sha`; `None` when no
/// line comes from `PROGRAM`, or it reaches a file not `held`.
fn imported(found: &str, sha: &str, held: &BTreeSet<&str>) -> Option<BTreeSet<String>> {
    let mut imports: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let prefix = format!("{sha}:");
    for line in found.lines() {
        let Some((file, import)) = line
            .strip_prefix(&prefix)
            .and_then(|rest| rest.split_once('\0'))
        else {
            continue;
        };
        let Some(target) = import
            .split(['"', '\''])
            .nth(1)
            .and_then(|to| resolve(file, to))
        else {
            continue;
        };
        imports.entry(file).or_default().push(target);
    }
    imports.contains_key(PROGRAM).then_some(())?;
    let mut loaded = BTreeSet::from([PROGRAM.to_owned()]);
    let mut next = VecDeque::from([PROGRAM.to_owned()]);
    while let Some(file) = next.pop_front() {
        for target in imports.get(file.as_str()).into_iter().flatten() {
            if !held.contains(target.as_str()) {
                return None;
            }
            if loaded.insert(target.clone()) {
                next.push_back(target.clone());
            }
        }
    }
    Some(loaded)
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

/// When pi-packages last pointed `current`, beside `mirror`, at a version,
/// in seconds and a second more, so a host started in the same second
/// reads as started before it. pi-packages makes the link afresh each time
/// it moves, so its own time is the move, however long the version waited
/// built under a hold, and a step back and forward again counts from the
/// step forward. The version `current` names is the one checked to carry
/// the change, so any host started since loaded it; the hour keeps the
/// first such time it saw, so a later install doesn't hold a conversation
/// started on an earlier one carrying it. `None` when the link can't be
/// read.
pub fn swapped(mirror: &Path) -> Option<i64> {
    let link = std::fs::symlink_metadata(mirror.parent()?.join("current")).ok()?;
    let at = link.modified().ok()?;
    let at = at.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    Some(i64::try_from(at).ok()? + 1)
}

/// Each version `current` left at or after `at`, by its commit, as
/// pi-packages marks one in `retired`, beside `mirror`, with the time it
/// left: `current` named each such version after `at`, so a host or
/// conversation started since may run it. pi-packages drops a version's
/// mark as it drops the version, once nothing runs it. `None` when the
/// marks can't be read; none when there are none yet.
pub fn left_since(mirror: &Path, at: i64) -> Option<Vec<String>> {
    let marks = match std::fs::read_dir(mirror.parent()?.join("retired")) {
        Ok(marks) => marks,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Some(Vec::new()),
        Err(_) => return None,
    };
    let mut left = Vec::new();
    for mark in marks {
        let mark = mark.ok()?;
        let when = mark.metadata().ok()?.modified().ok()?;
        let when = when.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
        if i64::try_from(when).ok()? >= at {
            left.push(mark.file_name().into_string().ok()?);
        }
    }
    Some(left)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_program_loads_what_it_imports_and_what_that_imports() {
        let found = concat!(
            "f00d:scripts/pi-host.ts\0from \"../host/service/program.ts\"\n",
            "f00d:host/service/program.ts\0import(\"../screen/screens.ts\"\n",
            "f00d:host/service/program.ts\0from './rollout.ts'\n",
            "f00d:host/screen/screens.ts\0import \"../../lib/core/x:y.mjs\"\n",
            "f00d:lib/core/x:y.mjs\0from \"./z.mjs\"\n",
            "f00d:lib/core/z.mjs\0new URL(\"./read.cjs\"\n",
            "f00d:host/screen/screens.ts\0from \"../../../outside.ts\"\n",
            "f00d:extensions/quest/index.ts\0from \"../../lib/core/y.ts\"\n",
            "beef:host/service/rollout.ts\0from \"./elsewhere.ts\"\n",
        );
        let loaded = [
            "scripts/pi-host.ts",
            "host/service/program.ts",
            "host/screen/screens.ts",
            "host/service/rollout.ts",
            "lib/core/x:y.mjs",
            "lib/core/z.mjs",
            "lib/core/read.cjs",
        ];
        let held: BTreeSet<&str> = loaded
            .into_iter()
            .chain(["extensions/quest/index.ts"])
            .collect();
        assert_eq!(
            imported(found, "f00d", &held),
            Some(BTreeSet::from(loaded.map(str::to_owned))),
            "the program's files weren't followed through each import, or an extension, \
             another commit's line or a path out of the repository was taken in"
        );
        assert_eq!(
            imported("f00d:host/a.ts\0from \"./b.ts\"\n", "f00d", &held),
            None,
            "a program importing nothing, as one moved away, read as loading only itself"
        );
        let unheld: BTreeSet<&str> = held
            .iter()
            .copied()
            .filter(|f| *f != "lib/core/read.cjs")
            .collect();
        assert_eq!(
            imported(found, "f00d", &unheld),
            None,
            "a program naming a file its commit doesn't hold was followed as if it loaded nothing \
             past it"
        );
    }

    #[test]
    fn a_change_was_installed_when_current_last_moved() {
        let place = std::env::temp_dir().join(format!("cadence-swapped-{}", std::process::id()));
        std::fs::remove_dir_all(&place).ok();
        std::fs::create_dir_all(place.join("versions/abc")).expect("a version");
        let mirror = place.join("mirror.git");
        assert_eq!(swapped(&mirror), None, "a missing link read as a move");
        let current = place.join("current");
        std::os::unix::fs::symlink("versions/abc", &current).expect("the link");
        // The version and the link at two known times, read off no clock.
        let touch = |args: &[&str]| {
            let done = std::process::Command::new("touch").args(args).status();
            assert!(
                done.is_ok_and(|done| done.success()),
                "touch {args:?} failed"
            );
        };
        let version = place.join("versions/abc");
        touch(&["-d", "@2000000000", &version.to_string_lossy()]);
        touch(&["-h", "-d", "@1000000000", &current.to_string_lossy()]);
        assert_eq!(
            swapped(&mirror),
            Some(1_000_000_001),
            "the link's own time, a second on, wasn't read, or the link was followed to its \
             version's"
        );
        assert_eq!(
            left_since(&mirror, 0),
            Some(Vec::new()),
            "no version left yet read as unreadable"
        );
        std::fs::create_dir_all(place.join("retired")).expect("the marks' folder");
        for (commit, at) in [("old", "@999999999"), ("stepped", "@1000000500")] {
            let mark = place.join("retired").join(commit);
            std::fs::write(&mark, "").expect("a mark");
            touch(&["-d", at, &mark.to_string_lossy()]);
        }
        assert_eq!(
            left_since(&mirror, 1_000_000_001),
            Some(vec!["stepped".to_owned()]),
            "a version current left after the time kept wasn't found, or one it left before was"
        );
        std::fs::remove_dir_all(&place).ok();
    }
}
