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

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

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
    let git = |args: &[&str]| {
        let mut all = vec!["-C", &at.checkout];
        all.extend_from_slice(args);
        command("git", &all).unwrap_or_default()
    };
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
        let log = kept_log(kept, &main).unwrap_or_else(|| {
            let log = read_log(&git, &since, &main, &at.folder);
            keep_log(kept, &main, &log);
            log
        });
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

/// Reads the folder's merges from git. On a busy machine this took up to
/// six seconds, and main as fetched moves far less often than the Spec
/// reads, so what it finds is kept by main's commit.
fn read_log(
    git: &(impl Fn(&[&str]) -> String + Sync),
    since: &str,
    main: &str,
    folder: &str,
) -> Log {
    let lock = format!("{folder}/flake.lock");
    let format = "--format=%H %ct %s";
    let (merges, locks, ()) = landings::side_by_side(
        || git(&["log", "--first-parent", since, format, main, "--", folder]),
        || {
            // A pathspec that leaves the lock out reads every commit's tree
            // and took 17 s on a busy machine; the lock's own log, then the
            // files each of its few merges moved, take a fraction of one.
            let moved = git(&[
                "log",
                "--first-parent",
                since,
                "--format=%H",
                main,
                "--",
                &lock,
            ]);
            let shas: Vec<&str> = moved.split_whitespace().collect();
            if shas.is_empty() {
                String::new()
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
    let (reverts, undoes) = reverted(&merges);
    let mut left_out = lock_only(&locks, &lock);
    left_out.extend(undoes);
    Log {
        main: without(&merges, &left_out),
        reverts,
    }
}

/// The log kept for main's commit `at`, when that's the one kept.
fn kept_log(kept: &Path, at: &str) -> Option<Log> {
    let all: Value = serde_json::from_str(&fs::read_to_string(kept).ok()?).ok()?;
    (all["at"].as_str() == Some(at)).then(|| Log {
        main: all["main"].as_str().unwrap_or_default().to_owned(),
        reverts: all["reverts"].as_str().unwrap_or_default().to_owned(),
    })
}

/// Keeps the log for main's commit `at`; a failure only costs the next
/// read its git.
fn keep_log(kept: &Path, at: &str, log: &Log) {
    let all = serde_json::json!({"at": at, "main": log.main, "reverts": log.reverts});
    if let Some(folder) = kept.parent() {
        let _ = fs::create_dir_all(folder);
    }
    let _ = write_whole(kept, &all.to_string());
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

    #[test]
    fn a_kept_log_is_read_only_for_the_commit_it_was_kept_for() {
        let folder =
            std::env::temp_dir().join(format!("cadence-repository-{}", std::process::id()));
        let kept = folder.join("cadence/repository.json");
        let log = Log {
            main: "aaa 1 Add a thing (#1)".into(),
            reverts: String::new(),
        };
        keep_log(&kept, "main1", &log);
        assert_eq!(kept_log(&kept, "main1"), Some(log));
        assert_eq!(
            kept_log(&kept, "main2"),
            None,
            "main moved, so git is read again"
        );
        let _ = fs::remove_dir_all(folder);
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
