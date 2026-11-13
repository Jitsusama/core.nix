//! `merge-net ready COMMIT`: whether a branch's head may merge, asked
//! before merging by whatever merges it. It may when it sits on origin's
//! main as it is now, and changed-checks built what it named there.
//!
//! On 8 November four checks were red on main by 11:00. changed-checks
//! named each at its merge, but one branch was built before main gained
//! what it collided with, and the others built only some of what was named,
//! or nothing. Both show in nix-offload's record of what passed at each
//! commit (`/var/lib/nix-offload-passed/COMMIT` where its keeper stands,
//! else `~/.local/state/nix-offload/passed/COMMIT`), which keeps a line
//! for each changed-checks run sent that passed, with its arguments.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::apart::{
    LISTS, apart, cases_apart, checks_apart, descriptions_apart, is_cases, is_prose, lines_apart,
    prose_apart,
};

/// Whether nix-offload's record for a commit holds a changed-checks run that
/// passed there and built what it named, against a base `on_main` says
/// origin's main holds, so it named everything a merge onto main moves.
/// A run that only named or only kept a base built nothing.
pub fn built_there(record: &str, on_main: &dyn Fn(&str) -> bool) -> bool {
    record
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|line| line["dir"].is_null())
        .filter_map(|line| line["run"].as_array().cloned())
        .any(|run| {
            let words: Vec<&str> = run.iter().filter_map(serde_json::Value::as_str).collect();
            let built = !words
                .iter()
                .any(|word| matches!(*word, "--name-only" | "--keep-base"));
            let base = words
                .iter()
                .position(|word| *word == "--base")
                .and_then(|at| words.get(at + 1));
            built && base.is_none_or(|base| on_main(base))
        })
}

/// What an earlier commit of the same change says of a head with no run of
/// its own.
#[derive(Debug, PartialEq, Eq)]
pub enum Before {
    /// The same change passed at this commit, and main has since moved
    /// none of its files, so the rebase changed nothing it was checked on.
    Clean(String),
    /// The same change passed at this commit, but main has since moved
    /// these of its files, so changed-checks runs again at the head.
    Shares(String, Vec<String>),
    /// The same change passed whole at two commits, the newest first, and
    /// main has since moved these of its files: it has been checked twice,
    /// so a clean rebase needs no third run unless main renamed or deleted
    /// a path it names, which [`names_gone`] says.
    Twice(String, String, Vec<String>),
    /// No commit of the same change passed.
    Unchecked,
}

/// Whether the head is the same change as one of `candidates`, newest
/// first, that a changed-checks run passed at (its `record`, judged as
/// `built_there` does with `on_main`). `same` says, of a candidate,
/// whether it makes the same change and, if so, which files it shares
/// with what main moved since the base it was checked on, as `shared`
/// works them out. On 8 November a
/// rule let a change merge after a rebase onto main without running its
/// checks again when main's new commits share none of its files, so the
/// gate takes what the rule takes. When they share some, only changed-checks
/// at the head counts, since which checks read a file shows only in their
/// derivations: on 9 November #2504 merged with words.json's checks rerun
/// at its head after main moved two other of its files, which no check
/// that ran read. Nix builds only what the rebase moved, and takes the
/// rest from the builders. But main moves faster than a run: on 9
/// November #2571 ran six times and #2576, #2603 and #2600 four or five
/// each, every run passing, so a change that passed at two commits is
/// `Twice` and the net tests it after it merges.
pub fn checked_before(
    candidates: &[String],
    record: &dyn Fn(&str) -> String,
    on_main: &dyn Fn(&str) -> bool,
    same: &dyn Fn(&str) -> Option<Vec<String>>,
) -> Before {
    let mut first_shared: Option<(String, Vec<String>)> = None;
    let mut passes = 0;
    for candidate in candidates {
        if !built_there(&record(candidate), on_main) {
            continue;
        }
        let Some(shared) = same(candidate) else {
            continue;
        };
        if shared.is_empty() {
            return Before::Clean(candidate.clone());
        }
        passes += 1;
        match &first_shared {
            None => first_shared = Some((candidate.clone(), shared)),
            Some((newest, shared)) if passes == 2 => {
                return Before::Twice(newest.clone(), candidate.clone(), shared.clone());
            }
            Some(_) => {}
        }
    }
    first_shared.map_or(Before::Unchecked, |(newest, shared)| {
        Before::Shares(newest, shared)
    })
}

/// What `then` makes of `candidate`'s base (`base_of`), only when the
/// change from that base to it has the patch id `change` (`id_of`).
/// Nearly every candidate is another change, and its id, from a diff of
/// that change alone, tells it apart in milliseconds, while what main
/// moved since a base hours old is a diff of hundreds of files, with
/// renames found. On 9 November the gate read that for each of a day's
/// 1,245 passed records before it compared ids, so `ready` took five
/// minutes and the review tool's merge gave up on #2687.
pub fn when_same<T>(
    candidate: &str,
    change: &str,
    base_of: impl Fn(&str) -> Option<String>,
    id_of: impl Fn(&str, &str) -> Option<String>,
    then: impl FnOnce(String) -> Option<T>,
) -> Option<T> {
    let base = base_of(candidate)?;
    if id_of(&base, candidate).as_deref() != Some(change) {
        return None;
    }
    then(base)
}

/// One side of a rebase: the files it changed, the paths it renamed or
/// deleted, and what a file holds on that side.
pub struct Side<'a> {
    pub changed: &'a [String],
    pub gone: &'a [String],
    pub text: &'a dyn Fn(&str) -> String,
}

/// The files a branch shares with what main moved since its base: those
/// both changed, and those either side changed that name a path the other
/// renamed or deleted. On 9 November #2597 renamed a script after #2613,
/// merged since its run, named the old path in a list #2597 never touched,
/// and main broke when #2597 merged with no run of its own.
pub fn shared(branch: &Side, main: &Side) -> Vec<String> {
    let mut shared: Vec<String> = main
        .changed
        .iter()
        .filter(|file| branch.changed.contains(file))
        .cloned()
        .collect();
    for file in names_gone(branch, main) {
        if !shared.contains(&file) {
            shared.push(file);
        }
    }
    shared
}

/// The files either side changed that name a path the other renamed or
/// deleted, which no number of earlier runs speaks for, since the run
/// that reads the old path never saw it gone.
pub fn names_gone(branch: &Side, main: &Side) -> Vec<String> {
    let mut naming: Vec<String> = Vec::new();
    for (side, other) in [(main, branch), (branch, main)] {
        for file in side.changed {
            let text = (side.text)(file);
            if !naming.contains(file) && other.gone.iter().any(|path| names(&text, file, path)) {
                naming.push(file.clone());
            }
        }
    }
    naming
}

/// Whether `text`, the file at `file`, names `path`: whole, from the
/// repository's root, or from the file's own folder, as a Nix path does.
fn names(text: &str, file: &str, path: &str) -> bool {
    let folder = file.rfind('/').map_or("", |at| &file[..=at]);
    text.contains(path)
        || path
            .strip_prefix(folder)
            .is_some_and(|near| !folder.is_empty() && text.contains(&format!("./{near}")))
}

/// Whether `file`, which the branch shares with main, is a JSON file,
/// check-descriptions.nix, a folder's checks.nix, a list of modules, a
/// pytest file or a note in docs/, both changed `apart`, so it stands as
/// two files would.
/// One naming a path either side renamed or deleted stays shared, as
/// `shared` holds. `base` gives a file where the branch was checked from,
/// and `checked` where it was checked at.
pub fn stands_apart(
    file: &str,
    branch: &Side,
    main: &Side,
    base: &dyn Fn(&str) -> String,
    checked: &dyn Fn(&str) -> String,
) -> bool {
    let json = std::path::Path::new(file)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
    let checks = file == "checks.nix" || file.ends_with("/checks.nix");
    let read: fn(&str, &str, &str, &str) -> bool = if file == "check-descriptions.nix" {
        descriptions_apart
    } else if file == "checks.nix" {
        // The root's checks.nix gives checks and lists every folder's.
        |base, branch, main, head| {
            checks_apart(base, branch, main, head) || lines_apart(base, branch, main, head)
        }
    } else if LISTS.contains(&file) {
        lines_apart
    } else if checks {
        checks_apart
    } else if is_cases(file) {
        cases_apart
    } else if is_prose(file) {
        prose_apart
    } else if json {
        apart
    } else {
        return false;
    };
    let both = [branch, main]
        .iter()
        .all(|side| side.changed.iter().any(|changed| changed == file));
    if !both {
        return false;
    }
    let head = (branch.text)(file);
    let names_gone = branch
        .gone
        .iter()
        .chain(main.gone)
        .any(|path| head.contains(path.as_str()));
    !names_gone && read(&base(file), &checked(file), &(main.text)(file), &head)
}

/// The commits nix-offload kept a passed record for in the last day,
/// newest first.
pub fn recent_records() -> Vec<String> {
    let Some(folder) = record_of("") else {
        return Vec::new();
    };
    let day_ago = std::time::SystemTime::now() - std::time::Duration::from_hours(24);
    let mut kept: Vec<(std::time::SystemTime, String)> = fs::read_dir(folder)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let at = entry.metadata().ok()?.modified().ok()?;
            let name = entry.file_name().into_string().ok()?;
            (at >= day_ago && name.len() == 40).then_some((at, name))
        })
        .collect();
    kept.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
    kept.into_iter().map(|(_, name)| name).collect()
}

/// Where nix-offload's keeper keeps what passed, which only it may change,
/// as nix-offload's `keep::FOLDER` names it.
const KEPT: &str = "/var/lib/nix-offload-passed";

/// The keeper's socket unit, as `keep::UNIT` names it: there while the
/// machine runs the keeper, so a folder one left behind isn't read.
const KEEPER: &str = "/etc/systemd/system/nix-offload-keep.socket";

/// nix-offload's folder under the account's state.
pub fn offload_state() -> Option<PathBuf> {
    let state = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))?;
    Some(state.join("nix-offload"))
}

/// The file nix-offload keeps what passed at `commit` in: its keeper's,
/// where the machine has one, else the account's own.
pub fn record_of(commit: &str) -> Option<PathBuf> {
    if Path::new(KEEPER).exists() {
        return Some(Path::new(KEPT).join(commit));
    }
    Some(offload_state()?.join("passed").join(commit))
}

/// What nix-offload kept as passed at `commit`, or nothing.
pub fn record(commit: &str) -> String {
    record_of(commit)
        .and_then(|file| fs::read_to_string(file).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN: &str = "aaaa";

    fn on_main(commit: &str) -> bool {
        commit == MAIN
    }

    fn nothing(_: &str) -> String {
        String::new()
    }

    /// What a branch changing `files` shares with main moving `moved`,
    /// with nothing renamed on either side.
    fn sharing(files: &[String], moved: Option<Vec<String>>) -> Option<Vec<String>> {
        moved.map(|moved| {
            let branch = Side {
                changed: files,
                gone: &[],
                text: &nothing,
            };
            let main = Side {
                changed: &moved,
                gone: &[],
                text: &nothing,
            };
            shared(&branch, &main)
        })
    }

    fn owned(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn a_file_naming_a_path_the_other_side_renamed_is_shared() {
        // #2597's case: the branch renamed a script, and main since moved a
        // list naming its old path, which the branch never touched.
        let list = "tests/rules/every-script-waiting.nix";
        let at_main = |file: &str| {
            if file == list {
                "{ \"tests/pi-rings-low-and-says-it.sh\" = 21; }".to_owned()
            } else {
                String::new()
            }
        };
        let renamed = owned(&["tests/pi-rings-low-and-says-it.sh"]);
        let branch_files = owned(&["tests/pi-rings-low.sh", "modules/pi/home.nix"]);
        let main_files = owned(&[list, "docs/glossary.md"]);
        let branch = Side {
            changed: &branch_files,
            gone: &renamed,
            text: &nothing,
        };
        let main = Side {
            changed: &main_files,
            gone: &[],
            text: &at_main,
        };
        assert_eq!(shared(&branch, &main), owned(&[list]));
        // The other way round: main renamed a module's file, and the
        // branch's file names it from its own folder, as a Nix path does.
        let at_head = |file: &str| {
            if file == "modules/pi/home.nix" {
                "{ imports = [ ./rings.nix ]; }".to_owned()
            } else {
                String::new()
            }
        };
        let gone = owned(&["modules/pi/rings.nix"]);
        let branch = Side {
            changed: &branch_files,
            gone: &[],
            text: &at_head,
        };
        let main = Side {
            changed: &main_files,
            gone: &gone,
            text: &nothing,
        };
        assert_eq!(shared(&branch, &main), owned(&["modules/pi/home.nix"]));
        // Naming another file of that folder isn't naming the gone one.
        let at_head = |_: &str| "{ imports = [ ./ringside.nix ]; }".to_owned();
        let branch = Side {
            changed: &branch_files,
            gone: &[],
            text: &at_head,
        };
        assert_eq!(shared(&branch, &main), Vec::<String>::new());
    }

    #[test]
    fn a_run_that_built_what_it_named_is_ready() {
        let record = "{\"check\":\"checks.x86_64-linux.b\",\"dir\":null}\n\
                      {\"dir\":null,\"printed\":\"changed  b\\n\",\
                      \"run\":[\"--system\",\"x86_64-linux\"]}\n";
        assert!(built_there(record, &on_main));
    }

    #[test]
    fn a_run_that_only_named_or_kept_a_base_is_not() {
        let named = "{\"dir\":null,\"printed\":\"\",\
                     \"run\":[\"--name-only\",\"--system\",\"x86_64-linux\"]}\n\
                     {\"dir\":null,\"printed\":\"\",\
                     \"run\":[\"--keep-base\",\"--system\",\"x86_64-linux\"]}\n\
                     {\"check\":\"checks.x86_64-linux.b\",\"dir\":null}\n";
        assert!(!built_there(named, &on_main));
        assert!(!built_there("", &on_main), "nothing passed there");
    }

    #[test]
    fn a_run_against_a_base_main_lacks_is_not() {
        let run = |base: &str| {
            format!(
                "{{\"dir\":null,\"printed\":\"\",\"run\":[\"--base\",\"{base}\",\
                 \"--system\",\"x86_64-linux\"]}}\n"
            )
        };
        assert!(built_there(&run(MAIN), &on_main));
        assert!(
            !built_there(&run("bbbb"), &on_main),
            "a base on the branch skips what came before it"
        );
    }

    #[test]
    fn the_same_change_checked_before_a_clean_rebase_is_ready() {
        let passed = "{\"dir\":null,\"printed\":\"\",\"run\":[\"--system\",\"x86_64-linux\"]}\n";
        let record = |commit: &str| {
            if commit == "old" || commit == "older" {
                passed.to_owned()
            } else {
                String::new()
            }
        };
        let files = vec!["a.rs".to_owned(), "b.md".to_owned()];
        let candidates = vec!["other".to_owned(), "old".to_owned(), "older".to_owned()];
        // Main moved other files since: the rebase changed nothing checked.
        let moved = |commit: &str| (commit == "old").then(|| vec!["c.nix".to_owned()]);
        assert_eq!(
            checked_before(&candidates, &record, &on_main, &|at: &str| sharing(
                &files,
                moved(at)
            )),
            Before::Clean("old".to_owned())
        );
        // Main moved one of its files: changed-checks runs again at the
        // head, whatever single checks passed there.
        let moved =
            |commit: &str| (commit == "old").then(|| vec!["b.md".to_owned(), "c.nix".to_owned()]);
        assert_eq!(
            checked_before(&candidates, &record, &on_main, &|at: &str| sharing(
                &files,
                moved(at)
            )),
            Before::Shares("old".to_owned(), vec!["b.md".to_owned()])
        );
        // Only single checks passed at a later commit of the same change:
        // they don't say what read the shared file.
        let rerun = "{\"check\":\"checks.x86_64-linux.formatting\",\"dir\":null}\n";
        let record_rechecked = |commit: &str| match commit {
            "old" => passed.to_owned(),
            "rebased" => rerun.to_owned(),
            _ => String::new(),
        };
        let candidates = vec!["rebased".to_owned(), "old".to_owned()];
        let moved = |commit: &str| match commit {
            "old" => Some(vec!["b.md".to_owned(), "c.nix".to_owned()]),
            "rebased" => Some(vec!["c.nix".to_owned()]),
            _ => None,
        };
        assert_eq!(
            checked_before(&candidates, &record_rechecked, &on_main, &|at: &str| {
                sharing(&files, moved(at))
            }),
            Before::Shares("old".to_owned(), vec!["b.md".to_owned()])
        );
        // changed-checks passed at that later commit, and main moved none of
        // the change's files since its base.
        let record_rerun = |commit: &str| match commit {
            "old" | "rebased" => passed.to_owned(),
            _ => String::new(),
        };
        assert_eq!(
            checked_before(&candidates, &record_rerun, &on_main, &|at: &str| sharing(
                &files,
                moved(at)
            )),
            Before::Clean("rebased".to_owned())
        );
        let candidates = vec!["other".to_owned(), "old".to_owned(), "older".to_owned()];
        // An earlier commit of another change, or the same change that
        // never passed, says nothing.
        let moved = |commit: &str| (commit == "other").then(Vec::new);
        assert_eq!(
            checked_before(&candidates, &record, &on_main, &|at: &str| sharing(
                &files,
                moved(at)
            )),
            Before::Unchecked
        );
    }

    #[test]
    fn another_change_is_told_apart_by_its_id_before_mains_moves_are_read() {
        let read = std::cell::Cell::new(0);
        let base_of = |candidate: &str| Some(format!("{candidate}-base"));
        let id_of = |_: &str, candidate: &str| {
            Some(if candidate == "same" { "id" } else { "other" }.to_owned())
        };
        let mains_moves = |base: String| {
            read.set(read.get() + 1);
            Some(base)
        };
        assert_eq!(
            when_same("another", "id", base_of, id_of, mains_moves),
            None,
            "a change with another id counted as the same"
        );
        assert_eq!(read.get(), 0, "it read what main moved for another change");
        assert_eq!(
            when_same("same", "id", base_of, id_of, mains_moves),
            Some("same-base".to_owned()),
            "the same change wasn't read from its own base"
        );
    }

    #[test]
    fn the_same_change_passed_at_two_commits_needs_no_third_run() {
        let passed = "{\"dir\":null,\"printed\":\"\",\"run\":[\"--system\",\"x86_64-linux\"]}\n";
        let files = vec!["a.rs".to_owned(), "b.md".to_owned()];
        let candidates = vec!["rebased".to_owned(), "old".to_owned(), "other".to_owned()];
        let moved = |commit: &str| match commit {
            "rebased" => Some(vec!["b.md".to_owned()]),
            "old" => Some(vec!["a.rs".to_owned(), "b.md".to_owned()]),
            _ => None,
        };
        let both = |commit: &str| match commit {
            "rebased" | "old" | "other" => passed.to_owned(),
            _ => String::new(),
        };
        assert_eq!(
            checked_before(&candidates, &both, &on_main, &|at: &str| sharing(
                &files,
                moved(at)
            )),
            Before::Twice(
                "rebased".to_owned(),
                "old".to_owned(),
                vec!["b.md".to_owned()]
            ),
            "passed whole at two commits of the same change, so a third rebase needs no run"
        );
        // Passed once, and another change passed: one run still asks.
        let once = |commit: &str| match commit {
            "rebased" | "other" => passed.to_owned(),
            _ => String::new(),
        };
        assert_eq!(
            checked_before(&candidates, &once, &on_main, &|at: &str| sharing(
                &files,
                moved(at)
            )),
            Before::Shares("rebased".to_owned(), vec!["b.md".to_owned()])
        );
    }

    #[test]
    fn a_file_naming_what_main_took_away_is_named_whatever_ran() {
        let files = vec!["tests/list.nix".to_owned()];
        let moved = vec!["tests/old.sh".to_owned()];
        let names_old = |_: &str| "[ ./old.sh ]".to_owned();
        let branch = Side {
            changed: &files,
            gone: &[],
            text: &names_old,
        };
        let main = Side {
            changed: &moved,
            gone: &moved,
            text: &nothing,
        };
        assert_eq!(names_gone(&branch, &main), files);
        let branch = Side {
            changed: &files,
            gone: &[],
            text: &nothing,
        };
        assert_eq!(names_gone(&branch, &main), Vec::<String>::new());
    }

    #[test]
    fn check_descriptions_changed_at_different_checks_stand_apart() {
        // #2763's case: each side described a check of its own.
        let entry = |name: &str| format!("  {name} = ''\n    proves\n  '';\n");
        let file = "check-descriptions.nix";
        let base = format!("{{\n{}}}\n", entry("b"));
        let ours = format!("{{\n{}{}}}\n", entry("b"), entry("c"));
        let theirs = format!("{{\n{}{}}}\n", entry("a"), entry("b"));
        let head = format!("{{\n{}{}{}}}\n", entry("a"), entry("b"), entry("c"));
        let files = owned(&[file]);
        let (at_head, at_main) = (|_: &str| head.clone(), |_: &str| theirs.clone());
        let branch = Side {
            changed: &files,
            gone: &[],
            text: &at_head,
        };
        let main = Side {
            changed: &files,
            gone: &[],
            text: &at_main,
        };
        let (at_base, checked) = (|_: &str| base.clone(), |_: &str| ours.clone());
        assert!(
            stands_apart(file, &branch, &main, &at_base, &checked),
            "two checks described apart were counted as one file"
        );
        assert!(
            !stands_apart("other.nix", &branch, &main, &at_base, &checked),
            "another Nix file was read as check descriptions"
        );
        let main = Side {
            changed: &files,
            gone: &[],
            text: &|_: &str| ours.replace("proves", "holds"),
        };
        assert!(
            !stands_apart(file, &branch, &main, &at_base, &checked),
            "both sides' changes to one check stood apart"
        );
    }

    #[test]
    fn a_folders_checks_changed_at_different_checks_stand_apart() {
        // #2807's case: the branch added a check while main changed another.
        let entry = |name: &str, does: &str| format!("\n  {name} = {does};\n");
        let file = "modules/shell/checks.nix";
        let around = |entries: &str| format!("{{ pkgs }}:\nlet\n  x = 1;\nin\n{{{entries}}}\n");
        let base = around(&entry("a", "x"));
        let ours = around(&(entry("a", "x") + &entry("b", "x")));
        let theirs = around(&entry("a", "x + 1"));
        let head = around(&(entry("a", "x + 1") + &entry("b", "x")));
        let files = owned(&[file]);
        let (at_head, at_main) = (|_: &str| head.clone(), |_: &str| theirs.clone());
        let branch = Side {
            changed: &files,
            gone: &[],
            text: &at_head,
        };
        let main = Side {
            changed: &files,
            gone: &[],
            text: &at_main,
        };
        let (at_base, checked) = (|_: &str| base.clone(), |_: &str| ours.clone());
        assert!(
            stands_apart(file, &branch, &main, &at_base, &checked),
            "two checks changed apart in a folder's checks.nix were counted as one file"
        );
        assert!(
            !stands_apart(
                "modules/shell/tests.nix",
                &branch,
                &main,
                &at_base,
                &checked
            ),
            "another Nix file was read as checks"
        );
    }

    #[test]
    fn lists_of_modules_given_a_line_each_stand_apart() {
        // 12 November's case: two changes each listed a module of its own.
        let base = "[\n  ./a\n  ./c\n]\n";
        let (ours, theirs) = ("[\n  ./a\n  ./b\n  ./c\n]\n", "[\n  ./0\n  ./a\n  ./c\n]\n");
        let head = "[\n  ./0\n  ./a\n  ./b\n  ./c\n]\n";
        let (at_head, at_main) = (|_: &str| head.to_owned(), |_: &str| theirs.to_owned());
        let (at_base, checked) = (|_: &str| base.to_owned(), |_: &str| ours.to_owned());
        for (file, list) in LISTS
            .iter()
            .map(|file| (*file, true))
            .chain([("a.nix", false)])
        {
            let files = owned(&[file]);
            let (branch, main) = (
                Side {
                    changed: &files,
                    gone: &[],
                    text: &at_head,
                },
                Side {
                    changed: &files,
                    gone: &[],
                    text: &at_main,
                },
            );
            assert_eq!(
                stands_apart(file, &branch, &main, &at_base, &checked),
                list,
                "{file}, given a line by each side, was judged wrongly"
            );
        }
    }

    #[test]
    fn a_run_in_another_folder_of_the_repository_is_not() {
        let record = "{\"dir\":\"sub\",\"printed\":\"\",\"run\":[\"--system\",\"x86_64-linux\"]}\n";
        assert!(!built_there(record, &on_main));
    }
}
