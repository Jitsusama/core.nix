//! The machine repository's changes that reached him, counted the way
//! core's and the harness's are: a merge to its main that touches the
//! folder the machine is built from reached him at the first update that
//! built a commit holding it, and stayed unless a later merge reverted it.
//!
//! Where that repository lives is update-to-main's to know, so it's read
//! from update-to-main's own config, which names the checkout, the folder
//! and the command that says what main was last fetched as; core.nix
//! names no machine repository. A machine whose update-to-main follows no
//! hosted checkout reads nothing here.
//!
//! A merge that only moves the folder's lock is left out: those take a
//! new core, whose changes core's count already holds. So is a revert,
//! which brings nothing new and counts against the merge it undoes.
//!
//! Long because its cases are over a third of it: each way the kept walk
//! is read, joined, aged out or refused, with a git that answers or fails.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;
use std::{fs, io};

use jiff::tz::TimeZone;
use serde_json::Value;

use super::Reading;
use super::landings::{self, Sources};
use crate::records::Look;
use crate::{DAY, command, write_whole};

/// update-to-main's config, which every account may read.
const CONFIG: &str = "/etc/update-to-main/config.json";

/// How many days back its changes are read, as landings reads core's.
const DAYS: i64 = 10;

/// Where the machine repository is, from update-to-main's config.
#[derive(Debug, PartialEq, Eq)]
struct Hosted {
    checkout: String,
    folder: String,
    /// The command, run in the checkout, that prints main as last fetched.
    fetched: Vec<String>,
}

fn hosted(config: &str) -> Option<Hosted> {
    let all: Value = serde_json::from_str(config).ok()?;
    let hosted = &all["hosted"];
    let words = |key: &str| hosted[key].as_str().map(str::to_owned);
    Some(Hosted {
        checkout: words("checkout")?,
        folder: words("directory")?,
        fetched: hosted["fetched"]
            .as_array()?
            .iter()
            .map(|w| w.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
            .filter(|c| !c.is_empty())?,
    })
}

/// The first-parent logs of core's, the harness's and the machine
/// repository's mains, newest first as `%H %ct %s` lines, for `cadence
/// hour`, which counts what each landed; read side by side, as the axes
/// read them, from the same mirrors and the same kept log.
pub fn mains(folder: &Path, now: i64) -> [String; 3] {
    let runtime = super::readings::runtime_dir();
    let kept = super::readings::cache_home().join("cadence/repository.json");
    std::thread::scope(|s| {
        let core = s.spawn(|| landings::gather(folder, &runtime, now).main);
        let harness = s.spawn(|| landings::gather_harness(now).main);
        let machine = gather(now, &kept).main;
        [
            core.join().unwrap_or_default(),
            harness.join().unwrap_or_default(),
            machine,
        ]
    })
}

/// Reads the machine repository's main as last fetched, never fetching,
/// and update-to-main's looks at it, from the journal.
#[must_use]
pub fn gather(now: i64, kept: &Path) -> Sources {
    let Some(at) = fs::read_to_string(CONFIG).ok().as_deref().and_then(hosted) else {
        return Sources::default();
    };
    let Some(main) = fetched(&at) else {
        return Sources::default();
    };
    let since = format!("--since=@{}", now - DAYS * DAY);
    // Said, so a walk that fails every hour, and the kept walk shown in
    // its place, can be seen in the journal rather than only as an old
    // landing count.
    let tried = |args: &[&str]| {
        let mut all = vec!["-C", &at.checkout];
        all.extend_from_slice(args);
        command("git", &all).inspect_err(said).ok()
    };
    let git = |args: &[&str]| tried(args).unwrap_or_default();
    let (log, journal) = std::thread::scope(|s| {
        let journal = s.spawn(|| {
            command(
                "journalctl",
                &[
                    "-u",
                    "update-to-main.service",
                    &since,
                    "-o",
                    "short-unix",
                    "--no-pager",
                    "--grep=\"step\":\"(done|unchanged)\"",
                ],
            )
            .unwrap_or_default()
        });
        let log = log_of(&walked(&tried, kept, &main, &at.folder, now), &at.folder);
        let journal = journal
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
        (log, journal)
    });
    let built = built(&journal);
    let commits: BTreeSet<&str> = built.iter().map(|(_, c)| c.as_str()).collect();
    let times_at = kept.with_file_name("built-times.txt");
    let (mut times, unknown) =
        known_times(&commits, &fs::read_to_string(&times_at).unwrap_or_default());
    if !unknown.is_empty() {
        let mut args = vec!["log", "--no-walk=unsorted", "--format=%H %ct"];
        args.extend(unknown);
        times.push_str(&git(&args));
    }
    // Kept for the next read; a failure only costs it git again.
    let _ = write_whole(&times_at, &times);
    Sources {
        looks: looks(&log.main, &built, &times),
        main: log.main,
        reverts: log.reverts,
        bills: String::new(),
    }
}

/// The folder's merges on main, lock moves and reverts left out, and
/// the merges reverted, as landings reads them.
#[derive(Debug, PartialEq, Eq)]
struct Log {
    main: String,
    reverts: String,
}

/// What a walk of the folder's history found, before lock moves and
/// reverts are left out.
#[derive(Debug, Default, PartialEq, Eq)]
struct Walk {
    /// The folder's merges on main, newest first, as `%H %ct %s` lines.
    merges: String,
    /// What each merge that moved the lock changed in the folder, as
    /// `git log --name-only --format=@%H` gives it.
    locks: String,
}

/// The folder's walk at `main`, kept by main's commit for the next read.
/// Main moves every hour, and walking the whole window each time took
/// 43 s and 615 MB for one of its two logs on 25 November, holding the
/// hour at its memory limit for five minutes. So when main has only
/// moved on from the commit kept, only what it gained is walked; when it
/// moved anywhere else, or nothing is kept, the whole window is. What
/// falls out of the window is let go either way. A walk git fails in is
/// never kept, so the next read walks again rather than missing merges
/// for ten days: until then the walk kept shows, true as far as it goes,
/// where main only gained, and nothing where it moved elsewhere, as a
/// failed walk showed before walks were kept.
fn walked(
    git: &(impl Fn(&[&str]) -> Option<String> + Sync),
    kept: &Path,
    main: &str,
    folder: &str,
    now: i64,
) -> Walk {
    let since = now - DAYS * DAY;
    let (at, before) = kept_walk(kept, folder).unwrap_or_default();
    if at == main {
        return within(&before, since);
    }
    let gained = !at.is_empty()
        && git(&["merge-base", at.as_str(), main]).is_some_and(|base| base.trim() == at);
    let walk = if gained {
        let Some(new) = walk_of(git, &format!("{at}..{main}"), since, folder) else {
            return within(&before, since);
        };
        Walk {
            merges: joined(&new.merges, &before.merges),
            locks: joined(&new.locks, &before.locks),
        }
    } else {
        let Some(whole) = walk_of(git, main, since, folder) else {
            return Walk::default();
        };
        whole
    };
    let walk = within(&walk, since);
    keep_walk(kept, main, folder, &walk);
    walk
}

/// The newer text's lines, then the older's.
fn joined(newer: &str, older: &str) -> String {
    newer
        .lines()
        .chain(older.lines())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The walk without the merges made before `since`, nor what the lock's
/// moves among them changed.
fn within(walk: &Walk, since: i64) -> Walk {
    let merges: Vec<&str> = walk
        .merges
        .lines()
        .filter(|line| {
            line.split(' ')
                .nth(1)
                .and_then(|at| at.parse::<i64>().ok())
                .is_some_and(|at| at >= since)
        })
        .collect();
    let shas: BTreeSet<&str> = merges.iter().filter_map(|l| l.split(' ').next()).collect();
    let mut keeping = false;
    let locks: Vec<&str> = walk
        .locks
        .lines()
        .filter(|line| {
            if let Some(sha) = line.strip_prefix('@') {
                keeping = shas.contains(sha);
            }
            keeping
        })
        .collect();
    Walk {
        merges: merges.join("\n"),
        locks: locks.join("\n"),
    }
}

/// The folder's merges and the lock's moves in `range` made since `from`,
/// from git, or none when git fails at any of it.
fn walk_of(
    git: &(impl Fn(&[&str]) -> Option<String> + Sync),
    range: &str,
    from: i64,
    folder: &str,
) -> Option<Walk> {
    let since = format!("--since=@{from}");
    let since = since.as_str();
    let lock = format!("{folder}/flake.lock");
    let format = "--format=%H %ct %s";
    let (merges, locks, ()) = landings::side_by_side(
        || git(&["log", "--first-parent", since, format, range, "--", folder]),
        || {
            // A pathspec that leaves the lock out reads every commit's tree
            // and took 17 s on a busy machine; the lock's own log, then the
            // files each of its few merges moved, take a fraction of one.
            let moved = git(&[
                "log",
                "--first-parent",
                since,
                "--format=%H",
                range,
                "--",
                &lock,
            ])?;
            let shas: Vec<&str> = moved.split_whitespace().collect();
            if shas.is_empty() {
                Some(String::new())
            } else {
                let mut args = vec![
                    "log",
                    "--no-walk=unsorted",
                    "--diff-merges=first-parent",
                    "--name-only",
                    "--format=@%H",
                ];
                args.extend(&shas);
                args.extend(["--", folder]);
                git(&args)
            }
        },
        || (),
    );
    Some(Walk {
        merges: merges?,
        locks: locks?,
    })
}

/// The folder's log from its walk: the lock's lone moves and the reverts
/// left out, and the merges reverted named.
fn log_of(walk: &Walk, folder: &str) -> Log {
    let (reverts, undoes) = reverted(&walk.merges);
    let mut left_out = lock_only(&walk.locks, &format!("{folder}/flake.lock"));
    left_out.extend(undoes);
    Log {
        main: without(&walk.merges, &left_out),
        reverts,
    }
}

/// The walk kept for `folder`, and main's commit it was kept for. A walk
/// kept before walks were kept whole, or for another folder, reads as
/// none, so the window is walked once.
fn kept_walk(kept: &Path, folder: &str) -> Option<(String, Walk)> {
    let all: Value = serde_json::from_str(&fs::read_to_string(kept).ok()?).ok()?;
    let words = |key: &str| all[key].as_str().map(str::to_owned);
    if words("folder")? != folder {
        return None;
    }
    Some((
        words("at")?,
        Walk {
            merges: words("merges")?,
            locks: words("locks")?,
        },
    ))
}

/// Keeps `folder`'s walk for main's commit `at`; a failure only costs the
/// next read its git.
fn keep_walk(kept: &Path, at: &str, folder: &str, walk: &Walk) {
    let all = serde_json::json!({
        "at": at,
        "folder": folder,
        "merges": walk.merges,
        "locks": walk.locks,
    });
    if let Some(folder) = kept.parent() {
        let _ = fs::create_dir_all(folder);
    }
    if let Err(err) = write_whole(kept, &all.to_string()) {
        said(&err);
    }
}

/// Says on the journal why the walk read or kept less than it might. Git's
/// answer is another program's text, so a newline in it can't write a line
/// that reads as cadence's own.
fn said(err: &io::Error) {
    let err = err.to_string().replace(char::is_control, "?");
    eprintln!("cadence: repository: {err}");
}

/// The kept times, `<sha> <seconds>` a line, of the built commits still
/// wanted, and the built commits git has yet to be asked about. A commit's
/// time never changes, and asking git for each one's walked the machine
/// repository's packs every run: about 90 MB read each hour and each time
/// Spec looked, on 14 November.
fn known_times<'a>(commits: &BTreeSet<&'a str>, kept: &str) -> (String, Vec<&'a str>) {
    let mut known = String::new();
    let mut seen = BTreeSet::new();
    for line in kept.lines() {
        if let Some((sha, _)) = line.split_once(' ')
            && commits.contains(sha)
            && seen.insert(sha)
        {
            known.push_str(line);
            known.push('\n');
        }
    }
    let unknown = commits
        .iter()
        .copied()
        .filter(|c| !seen.contains(c))
        .collect();
    (known, unknown)
}

/// Main's commit as last fetched, from the command update-to-main runs.
fn fetched(at: &Hosted) -> Option<String> {
    let (program, args) = at.fetched.split_first()?;
    let out = Command::new(program)
        .args(args)
        .current_dir(Path::new(&at.checkout))
        .output()
        .ok()?;
    let sha = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    (out.status.success() && !sha.is_empty()).then_some(sha)
}

/// The merges that moved nothing in the folder but its lock, from
/// `git log --name-only --format=@%H`.
fn lock_only(names: &str, lock: &str) -> BTreeSet<String> {
    let mut moved: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut sha = None;
    for line in names.lines().filter(|l| !l.is_empty()) {
        if let Some(s) = line.strip_prefix('@') {
            sha = Some(s);
            moved.entry(s).or_default();
        } else if let Some(s) = sha {
            moved.entry(s).or_default().push(line);
        }
    }
    moved
        .into_iter()
        .filter(|(_, files)| files.as_slice() == [lock])
        .map(|(sha, _)| sha.to_owned())
        .collect()
}

/// The merges a later one reverted, as landings reads them ("This reverts
/// commit <sha>"), and the reverts themselves. A revert's subject quotes
/// the one it undoes and names both requests, `Revert "<title>" (#<undone>)
/// (#<revert>)`; a walk of every branch's commits for their bodies, as
/// core's are read, took 27 s here.
fn reverted(main: &str) -> (String, BTreeSet<String>) {
    let merges: Vec<(&str, &str)> = main
        .lines()
        .filter_map(|line| {
            let mut words = line.splitn(3, ' ');
            let sha = words.next()?;
            words.next()?;
            Some((sha, words.next().unwrap_or_default()))
        })
        .collect();
    let mut named = String::new();
    let mut undoes = BTreeSet::new();
    for (sha, subject) in &merges {
        let Some(rest) = subject.strip_prefix("Revert \"") else {
            continue;
        };
        undoes.insert((*sha).to_owned());
        let Some(number) = rest.rsplit(" (#").nth(1).and_then(|n| n.strip_suffix(')')) else {
            continue;
        };
        let undone = format!("(#{number})");
        if let Some((sha, _)) = merges.iter().find(|(_, s)| s.ends_with(&undone)) {
            named.push_str("This reverts commit ");
            named.push_str(sha);
            named.push('\n');
        }
    }
    (named, undoes)
}

/// Main's log without the merges named.
fn without(main: &str, left_out: &BTreeSet<String>) -> String {
    main.lines()
        .filter(|line| {
            line.split(' ')
                .next()
                .is_none_or(|sha| !left_out.contains(sha))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Each update's look, when it ended and the machine repository's commit
/// it built, from update-to-main's journal in `short-unix`.
fn built(journal: &str) -> Vec<(i64, String)> {
    journal
        .lines()
        .filter_map(|line| {
            let (stamp, rest) = line.split_once(' ')?;
            let at: f64 = stamp.parse().ok()?;
            let said: Value = serde_json::from_str(&rest[rest.find('{')?..]).ok()?;
            matches!(said["step"].as_str(), Some("done" | "unchanged")).then_some(())?;
            let commit = said["commit"].as_str()?.to_owned();
            #[expect(clippy::cast_possible_truncation, reason = "a time in seconds")]
            let at = at as i64;
            Some((at, commit))
        })
        .collect()
}

/// The looks as landings reads them: each names the newest of the
/// folder's merges made at or before the commit it built, since most
/// commits main gains touch nothing the machine is built from. A commit
/// not yet in the checkout, or older than the log, is left out.
fn looks(main: &str, built: &[(i64, String)], times: &str) -> Vec<Look> {
    let made: BTreeMap<&str, i64> = times
        .lines()
        .filter_map(|line| {
            let (sha, at) = line.split_once(' ')?;
            Some((sha, at.trim().parse().ok()?))
        })
        .collect();
    let merges: Vec<(&str, i64)> = main
        .lines()
        .filter_map(|line| {
            let mut words = line.splitn(3, ' ');
            Some((words.next()?, words.next()?.parse().ok()?))
        })
        .collect();
    built
        .iter()
        .filter_map(|(at, commit)| {
            let when = *made.get(commit.as_str())?;
            let (sha, _) = merges.iter().find(|(_, merged)| *merged <= when)?;
            Some(Look {
                at: *at,
                core: (*sha).to_owned(),
                switched: true,
            })
        })
        .collect()
}

/// The machine repository's changes that reached him and stayed
/// yesterday, against the seven days before.
#[must_use]
pub fn reading(s: &Sources, now: i64, tz: &TimeZone) -> Reading {
    landings::kept_yesterday(
        s,
        now,
        tz,
        (
            "{} changes to your machine's repository reached you and stayed yesterday",
            "16V6F4",
        ),
        "the machine repository's main as last fetched, and update-to-main's journal",
    )
    .detail(Some(
        "a move of its lock alone is left out, since core's count holds what it brings".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn where_it_lives_is_read_from_update_to_mains_config() {
        let config = r#"{"hosted": {"checkout": "/src", "directory": "home/a/dots",
            "fetched": ["git", "rev-parse", "refs/prefetch/main"]}}"#;
        assert_eq!(
            hosted(config),
            Some(Hosted {
                checkout: "/src".into(),
                folder: "home/a/dots".into(),
                fetched: vec![
                    "git".into(),
                    "rev-parse".into(),
                    "refs/prefetch/main".into()
                ],
            })
        );
        assert_eq!(
            hosted(r#"{"repository": "https://example.org/a.git"}"#),
            None
        );
    }

    #[test]
    fn a_merge_that_moves_only_the_lock_is_left_out() {
        let names =
            "@aaa\nhome/a/dots/flake.lock\n\n@bbb\nhome/a/dots/flake.lock\nhome/a/dots/flake.nix\n";
        let left_out = lock_only(names, "home/a/dots/flake.lock");
        assert_eq!(left_out, BTreeSet::from(["aaa".to_owned()]));
        assert_eq!(
            without("ccc 3 Third\nbbb 2 Second\naaa 1 Take a core", &left_out),
            "ccc 3 Third\nbbb 2 Second"
        );
    }

    /// Ten days and a little after the walks' times below.
    const NOW: i64 = 1_000_000;

    const DOTS: &str = "home/a/dots";

    /// A kept walk's file in a folder of its own, let go with it.
    struct Kept(std::path::PathBuf);

    impl std::ops::Deref for Kept {
        type Target = Path;

        fn deref(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Kept {
        fn drop(&mut self) {
            if let Some(folder) = self.0.parent().and_then(Path::parent) {
                let _ = fs::remove_dir_all(folder);
            }
        }
    }

    /// A kept walk's file, in a folder of its own for `name`.
    fn kept_at(name: &str) -> Kept {
        let folder =
            std::env::temp_dir().join(format!("cadence-repository-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        Kept(folder.join("cadence/repository.json"))
    }

    /// The answer that makes the git below fail.
    const FAILS: &str = "<git fails>";

    /// A git that gives the first answer whose words its ask holds, or
    /// nothing, failing where that answer is `FAILS`, and keeps each ask.
    fn answering<'a>(
        asked: &'a std::sync::Mutex<Vec<String>>,
        answers: &'a [(&'a str, &'a str)],
    ) -> impl Fn(&[&str]) -> Option<String> + Sync + 'a {
        move |args: &[&str]| {
            let ask = args.join(" ");
            let answer = answers.iter().find(|(words, _)| ask.contains(*words));
            asked.lock().expect("no ask panicked").push(ask);
            match answer {
                Some((_, said)) if *said == FAILS => None,
                Some((_, said)) => Some((*said).to_owned()),
                None => Some(String::new()),
            }
        }
    }

    #[test]
    fn a_walk_kept_for_main_as_it_stands_asks_git_nothing() {
        let kept = kept_at("kept");
        let walk = Walk {
            merges: "m1 200000 First".into(),
            locks: String::new(),
        };
        keep_walk(&kept, "new", DOTS, &walk);
        let asked = std::sync::Mutex::new(Vec::new());
        let git = answering(&asked, &[]);
        assert_eq!(walked(&git, &kept, "new", DOTS, NOW), walk);
        assert_eq!(
            asked.lock().expect("no ask panicked").clone(),
            Vec::<String>::new(),
            "main hasn't moved, so git has nothing to walk"
        );
    }

    #[test]
    fn main_moved_on_from_the_kept_walk_walks_only_what_it_gained() {
        let kept = kept_at("gained");
        let before = Walk {
            merges: "m1 200000 First".into(),
            locks: String::new(),
        };
        keep_walk(&kept, "old", DOTS, &before);
        let asked = std::sync::Mutex::new(Vec::new());
        let answers = [
            ("merge-base old new", "old\n"),
            ("--name-only", "@m3\nhome/a/dots/flake.lock"),
            ("old..new -- home/a/dots/flake.lock", "m3\n"),
            (
                "old..new -- home/a/dots",
                "m3 400000 Take a core\nm2 300000 Second",
            ),
        ];
        let git = answering(&asked, &answers);
        let walk = walked(&git, &kept, "new", DOTS, NOW);
        assert_eq!(
            walk,
            Walk {
                merges: "m3 400000 Take a core\nm2 300000 Second\nm1 200000 First".into(),
                locks: "@m3\nhome/a/dots/flake.lock".into(),
            },
            "what main gained comes first, over what was kept"
        );
        let asked = asked.lock().expect("no ask panicked").clone();
        assert!(
            asked
                .iter()
                .filter(|ask| ask.starts_with("log --first-parent"))
                .all(|ask| ask.contains(" old..new -- ")),
            "each walk starts where the kept one ended: {asked:?}"
        );
        assert_eq!(
            kept_walk(&kept, DOTS),
            Some(("new".to_owned(), walk)),
            "the joined walk is kept for main as it now stands"
        );
        assert_eq!(
            log_of(&kept_walk(&kept, DOTS).unwrap_or_default().1, DOTS).main,
            "m2 300000 Second\nm1 200000 First",
            "the lock's lone move is still left out"
        );
    }

    #[test]
    fn main_moved_elsewhere_or_no_walk_kept_walks_the_whole_window() {
        let kept = kept_at("elsewhere");
        let before = Walk {
            merges: "m1 200000 Dropped by a force push".into(),
            locks: String::new(),
        };
        keep_walk(&kept, "old", DOTS, &before);
        let answers = [
            ("merge-base old new", "base\n"),
            ("new -- home/a/dots/flake.lock", ""),
            ("new -- home/a/dots", "m2 300000 Second"),
        ];
        for (kept, why) in [
            (kept, "main isn't where the kept walk ended"),
            (kept_at("none"), "no walk is kept"),
        ] {
            let asked = std::sync::Mutex::new(Vec::new());
            let git = answering(&asked, &answers);
            let walk = walked(&git, &kept, "new", DOTS, NOW);
            assert_eq!(walk.merges, "m2 300000 Second", "{why}");
            let asked = asked.lock().expect("no ask panicked").clone();
            assert!(
                asked.iter().any(|ask| ask.ends_with(" new -- home/a/dots"))
                    && !asked.iter().any(|ask| ask.contains("..new")),
                "{why}, so the whole window is walked: {asked:?}"
            );
        }
    }

    #[test]
    fn a_walk_git_fails_in_is_never_kept_and_shows_only_what_is_known() {
        let kept = kept_at("fails");
        let before = Walk {
            merges: "m1 200000 First".into(),
            locks: String::new(),
        };
        keep_walk(&kept, "old", DOTS, &before);
        let answers = [
            ("merge-base old new", "old\n"),
            ("old..new -- home/a/dots", FAILS),
        ];
        let asked = std::sync::Mutex::new(Vec::new());
        let git = answering(&asked, &answers);
        assert_eq!(
            walked(&git, &kept, "new", DOTS, NOW),
            before,
            "a failed walk of what main gained hid what was kept"
        );
        assert_eq!(
            kept_walk(&kept, DOTS),
            Some(("old".to_owned(), before)),
            "a failed walk was kept, so the next read wouldn't walk again"
        );
        let none = kept_at("fails-whole");
        let answers = [("new -- home/a/dots", FAILS)];
        let git = answering(&asked, &answers);
        assert_eq!(
            walked(&git, &none, "new", DOTS, NOW),
            Walk::default(),
            "a failed walk of the whole window showed merges git never gave"
        );
        assert_eq!(
            kept_walk(&none, DOTS),
            None,
            "a failed walk of the whole window was kept"
        );
    }

    #[test]
    fn a_lock_walk_git_fails_in_alone_keeps_nothing() {
        let kept = kept_at("lock-fails");
        let before = Walk {
            merges: "m1 200000 First".into(),
            locks: String::new(),
        };
        keep_walk(&kept, "old", DOTS, &before);
        let answers = [
            ("merge-base old new", "old\n"),
            ("old..new -- home/a/dots/flake.lock", FAILS),
            ("old..new -- home/a/dots", "m2 300000 Second"),
        ];
        let asked = std::sync::Mutex::new(Vec::new());
        let git = answering(&asked, &answers);
        assert_eq!(
            walked(&git, &kept, "new", DOTS, NOW),
            before,
            "merges joined with no word of their lock's moves were shown"
        );
        assert_eq!(
            kept_walk(&kept, DOTS),
            Some(("old".to_owned(), before)),
            "a walk whose lock walk failed was kept"
        );
    }

    #[test]
    fn a_merge_base_git_fails_in_walks_the_whole_window_afresh() {
        let kept = kept_at("base-fails");
        let before = Walk {
            merges: "m1 200000 First".into(),
            locks: String::new(),
        };
        keep_walk(&kept, "old", DOTS, &before);
        let answers = [
            ("merge-base", FAILS),
            ("new -- home/a/dots/flake.lock", ""),
            ("new -- home/a/dots", "m2 300000 Second"),
        ];
        let asked = std::sync::Mutex::new(Vec::new());
        let git = answering(&asked, &answers);
        let whole = Walk {
            merges: "m2 300000 Second".into(),
            locks: String::new(),
        };
        assert_eq!(
            walked(&git, &kept, "new", DOTS, NOW),
            whole,
            "with no proof main only gained, the kept walk was joined"
        );
        let asks = asked.lock().expect("no ask panicked").clone();
        assert!(
            !asks.iter().any(|ask| ask.contains("old..new")),
            "only what main gained was walked: {asks:?}"
        );
        assert_eq!(
            kept_walk(&kept, DOTS),
            Some(("new".to_owned(), whole)),
            "the whole walk wasn't kept for main as it stands"
        );
    }

    #[test]
    fn a_walk_kept_for_main_as_it_stands_is_not_written_again() {
        let kept = kept_at("unmoved");
        if let Some(folder) = kept.parent() {
            let _ = fs::create_dir_all(folder);
        }
        let written = serde_json::json!({
            "at": "new", "folder": DOTS, "merges": "", "locks": "", "by": "hand",
        })
        .to_string();
        let _ = fs::write(&*kept, &written);
        let asked = std::sync::Mutex::new(Vec::new());
        let git = answering(&asked, &[]);
        walked(&git, &kept, "new", DOTS, NOW);
        assert_eq!(
            fs::read_to_string(&*kept).unwrap_or_default(),
            written,
            "a walk main hasn't moved from was written again"
        );
    }

    #[test]
    fn a_walk_kept_for_another_folder_reads_as_none() {
        let kept = kept_at("folder");
        let walk = Walk {
            merges: "m1 200000 First".into(),
            locks: String::new(),
        };
        keep_walk(&kept, "new", "home/b/dots", &walk);
        assert_eq!(
            kept_walk(&kept, DOTS),
            None,
            "another folder's merges were taken as this one's"
        );
    }

    #[test]
    fn what_falls_out_of_the_window_is_let_go() {
        let walk = Walk {
            merges: "m2 300000 Second\nm0 100000 Too old\nmx none No time".into(),
            locks: "@m0\nx/flake.lock\n@m2\nx/flake.lock\nx/a".into(),
        };
        assert_eq!(
            within(&walk, NOW - DAYS * DAY),
            Walk {
                merges: "m2 300000 Second".into(),
                locks: "@m2\nx/flake.lock\nx/a".into(),
            },
            "a merge before the window, or with no time, goes, and its lock's names with it"
        );
    }

    #[test]
    fn a_log_kept_before_walks_were_kept_reads_as_no_walk_kept() {
        let kept = kept_at("older");
        if let Some(folder) = kept.parent() {
            let _ = fs::create_dir_all(folder);
        }
        let _ = fs::write(&*kept, r#"{"at":"main1","main":"aaa 1 A","reverts":""}"#);
        assert_eq!(
            kept_walk(&kept, DOTS),
            None,
            "its merges aren't kept whole, so the window is walked once afresh"
        );
    }

    #[test]
    fn a_revert_counts_against_what_it_undoes_and_is_no_change_itself() {
        let main = "\
3333 3 Revert \"Add a thing (#11)\" (#11) (#13)
2222 2 Keep the other (#12)
1111 1 Add a thing (#11)";
        let (named, undoes) = reverted(main);
        assert_eq!(named, "This reverts commit 1111\n");
        assert_eq!(undoes, BTreeSet::from(["3333".to_owned()]));
    }

    #[test]
    fn a_look_names_the_newest_merge_made_before_the_commit_it_built() {
        let journal = "\
100.5 host update-to-main[1]: {\"step\":\"done\",\"commit\":\"x1\",\"followed\":{\"core\":\"c\"}}
200.0 host update-to-main[1]: {\"step\":\"asked\",\"commit\":\"x2\"}
300.0 host update-to-main[1]: {\"step\":\"unchanged\",\"commit\":\"x3\"}
400.0 host update-to-main[1]: {\"step\":\"done\",\"commit\":\"x4\"}";
        let built = built(journal);
        assert_eq!(
            built,
            vec![(100, "x1".into()), (300, "x3".into()), (400, "x4".into())]
        );
        // x1 was made after the first merge, x3 after the second, and x4
        // isn't in the checkout yet.
        let main = "m2 60 Second\nm1 20 First";
        let looks = looks(main, &built, "x1 50\nx3 90");
        let named: Vec<(i64, &str)> = looks.iter().map(|l| (l.at, l.core.as_str())).collect();
        assert_eq!(named, vec![(100, "m1"), (300, "m2")]);
    }

    #[test]
    fn only_a_built_commit_with_no_kept_time_asks_git() {
        let commits: BTreeSet<&str> = ["aa", "bb"].into();
        assert_eq!(
            known_times(&commits, "aa 100\nzz 900\n"),
            ("aa 100\n".to_owned(), vec!["bb"]),
            "a kept time is used, a new commit asked about, one no longer built dropped"
        );
        assert_eq!(
            known_times(&BTreeSet::new(), "aa 100\n"),
            (String::new(), Vec::new()),
            "nothing built wants nothing"
        );
    }

    #[test]
    fn nothing_is_read_where_update_to_main_follows_no_checkout() {
        let s = Sources::default();
        let reading = reading(&s, 1_794_700_000, &TimeZone::UTC);
        assert_eq!(reading.now, None, "no looks, so no whole day to count");
    }
}
