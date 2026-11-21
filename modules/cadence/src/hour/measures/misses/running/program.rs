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
/// `import("./x")`, in either quote.
const IMPORT: &str = r#"(from|import) *\(? *["']\.\.?/[^"']+["']"#;

/// The files that import, as the harness names its scripts.
const SCRIPTS: &[&str] = &["*.ts", "*.mts", "*.js", "*.mjs"];

/// The files the host's program loads as the harness's commit `sha` left
/// them, from one grep of `path`; `None` when git can't say, or the
/// program imports nothing, as when it moved, since a program that can't
/// be followed would read every change to it as loading nowhere.
pub fn files(path: &str, sha: &str) -> Option<BTreeSet<String>> {
    let mut args = vec!["-C", path, "grep", "--null", "-o", "-E", IMPORT, sha, "--"];
    args.extend(SCRIPTS);
    let found = command("git", &args).ok()?;
    let mut files = imported(&found, sha)?;
    files.extend(PACKAGES.iter().map(|file| (*file).to_owned()));
    Some(files)
}

/// Every file `PROGRAM` imports, itself and through what it imports, from
/// `found`'s lines, each `SHA:FILE`, a NUL, then the import, as
/// `git grep --null -o` writes them for the commit `sha`; `None` when no
/// line comes from `PROGRAM`.
fn imported(found: &str, sha: &str) -> Option<BTreeSet<String>> {
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
/// the change, so any host started since loaded it; one started after an
/// earlier version carrying it reads as before, which only waits on that
/// host's next start. `None` when the link can't be read.
pub fn swapped(mirror: &Path) -> Option<i64> {
    let link = std::fs::symlink_metadata(mirror.parent()?.join("current")).ok()?;
    let at = link.modified().ok()?;
    let at = at.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    Some(i64::try_from(at).ok()? + 1)
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
            "f00d:host/screen/screens.ts\0from \"../../../outside.ts\"\n",
            "f00d:extensions/quest/index.ts\0from \"../../lib/core/y.ts\"\n",
            "beef:host/service/rollout.ts\0from \"./elsewhere.ts\"\n",
        );
        assert_eq!(
            imported(found, "f00d"),
            Some(BTreeSet::from(
                [
                    "scripts/pi-host.ts",
                    "host/service/program.ts",
                    "host/screen/screens.ts",
                    "host/service/rollout.ts",
                    "lib/core/x:y.mjs",
                    "lib/core/z.mjs",
                ]
                .map(str::to_owned)
            )),
            "the program's files weren't followed through each import, or an extension, \
             another commit's line or a path out of the repository was taken in"
        );
        assert_eq!(
            imported("f00d:host/a.ts\0from \"./b.ts\"\n", "f00d"),
            None,
            "a program importing nothing, as one moved away, read as loading only itself"
        );
    }

    #[test]
    fn a_change_was_installed_when_current_last_moved() {
        let place = std::env::temp_dir().join(format!("cadence-swapped-{}", std::process::id()));
        std::fs::remove_dir_all(&place).ok();
        std::fs::create_dir_all(place.join("versions/abc")).expect("a version");
        let mirror = place.join("mirror.git");
        assert_eq!(swapped(&mirror), None, "a missing link read as a move");
        let before = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("after 1970")
            .as_secs();
        std::os::unix::fs::symlink("versions/abc", place.join("current")).expect("the link");
        let at = swapped(&mirror).expect("the link's time");
        assert!(
            u64::try_from(at).is_ok_and(|at| at > before),
            "the link's own time, a second on, wasn't read: {at} against {before}"
        );
        std::fs::remove_dir_all(&place).ok();
    }
}
