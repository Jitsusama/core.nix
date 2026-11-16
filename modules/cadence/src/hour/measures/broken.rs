//! The checks a merge broke that still fail on main, from the net's
//! results. They're read in main's own order, since the net reruns an older
//! merge after a newer one has passed, and a failure main has already
//! answered is left out: its subtests quarantined, or passing alone since.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::Path;

use serde_json::Value;

use super::{Class, DAY, Measure, count, lines};
use crate::command;

/// How many of main's first-parent merges are read to order the results:
/// a busy day of main is some two hundred, so this reaches back days.
const MERGES: &str = "2000";

/// What the measure reads of main beside the net's results: where each
/// merge sits on main's first-parent line, oldest lowest, and the subtests
/// main's quarantine holds, by their test.
#[derive(Debug, Default)]
pub struct Main {
    place: HashMap<String, usize>,
    quarantined: BTreeSet<(String, String)>,
    window: Option<(i64, i64)>,
}

impl Main {
    /// Reads main from the mirror at `core`. A mirror that can't be read
    /// leaves the results in the order the net kept them.
    #[must_use]
    pub fn read(core: &Path, quarantine: &str) -> Self {
        let merges = command(
            "git",
            &[
                "-C",
                &core.to_string_lossy(),
                "log",
                "--first-parent",
                "-n",
                MERGES,
                "--format=%H %ct",
                "main",
            ],
        )
        .unwrap_or_default();
        Self::of(&merges, quarantine)
    }

    /// Main from `git log`'s lines, newest first, each a merge and when it
    /// was made, and the quarantine's JSON. Lines without their times leave
    /// main's window unknown.
    #[must_use]
    pub fn of(merges: &str, quarantine: &str) -> Self {
        let read: Vec<(&str, Option<i64>)> = merges
            .lines()
            .filter_map(|line| {
                let mut words = line.split_whitespace();
                Some((words.next()?, words.next().and_then(|at| at.parse().ok())))
            })
            .collect();
        let window = match (read.first(), read.last()) {
            (Some((_, Some(head))), Some((_, Some(oldest)))) => Some((*oldest, *head)),
            _ => None,
        };
        let place = read
            .iter()
            .rev()
            .enumerate()
            .map(|(at, (commit, _))| ((*commit).to_owned(), at))
            .collect();
        let list: Vec<Value> = serde_json::from_str(quarantine).unwrap_or_default();
        let quarantined = list
            .iter()
            .flat_map(|entry| {
                let test = entry["test"].as_str().unwrap_or_default().to_owned();
                entry["subtests"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(move |subtest| (test.clone(), subtest.to_owned()))
            })
            .collect();
        Self {
            place,
            quarantined,
            window,
        }
    }

    /// Where a result sits: by its merge's place on main, then by when the
    /// net kept it. A merge main doesn't hold sorts last, as one the mirror
    /// hasn't fetched yet, unless its result is older than the oldest merge
    /// read, when it's one past them and sorts first.
    fn key(&self, commit: &str, at: i64) -> (usize, i64) {
        let unheld = match self.window {
            Some((oldest, _)) if at <= oldest => 0,
            _ => usize::MAX,
        };
        (self.place.get(commit).copied().unwrap_or(unheld), at)
    }

    /// Whether a result at `commit`, kept at `at`, was run on something
    /// other than main: a commit main doesn't hold though the result came
    /// between the oldest merge read and main's head, such as merge-net's
    /// trial of a revert. On 16 November two such failures read as main's
    /// newest. A result after main's head may be a merge the mirror hasn't
    /// fetched, and one before the oldest a merge past those read, so both
    /// are kept.
    fn elsewhere(&self, commit: &str, at: i64) -> bool {
        !self.place.contains_key(commit)
            && self
                .window
                .is_some_and(|(oldest, head)| oldest < at && at < head)
    }
}

/// One run of a check, as the net kept it.
struct Run {
    key: (usize, i64),
    at: i64,
    passed: bool,
    log: String,
}

/// The checks a merge broke that still fail on main, from the net's
/// `results.jsonl`: a check whose every run on main since its last pass
/// failed, the newest in the day before `now`. The net runs a failure
/// again before it judges it, so one failure alone isn't yet a break: on
/// 16 November a card counted vm-desktop-looks-vi four seconds after its
/// one failure, minutes before its subtest passed alone. A stalled run
/// proves nothing either way, so it's left out. `failed` reads what a
/// run's log says failed.
#[must_use]
pub fn broken(
    text: &str,
    now: i64,
    owner: &str,
    main: &Main,
    failed: &dyn Fn(&str) -> Failures,
) -> Measure {
    let mut by_check: BTreeMap<String, Vec<Run>> = BTreeMap::new();
    for result in lines(text).filter(|r| r["stalled"] != Value::Bool(true)) {
        let (Some(check), Some(at), Some(passed)) = (
            result["check"].as_str(),
            result["at"].as_i64(),
            result["passed"].as_bool(),
        ) else {
            continue;
        };
        let commit = result["commit"].as_str().unwrap_or_default();
        if main.elsewhere(commit, at) {
            continue;
        }
        by_check.entry(check.to_owned()).or_default().push(Run {
            key: main.key(commit, at),
            at,
            passed,
            log: result["log"].as_str().unwrap_or_default().to_owned(),
        });
    }
    for runs in by_check.values_mut() {
        runs.sort_by_key(|run| run.key);
    }
    let failing: Vec<String> = by_check
        .iter()
        .filter(|(check, runs)| still_broken(check, runs, now, &by_check, main, failed))
        .map(|(check, _)| check.clone())
        .collect();
    Measure {
        key: "broken".to_owned(),
        class: Class::Functional,
        says: "checks a merge broke in the last day, still failing".to_owned(),
        now: count(failing.len()),
        target: Some(0.0),
        owner: owner.to_owned(),
        behind: failing,
    }
}

/// Whether `check`, whose runs are `runs` in main's order, is broken: every
/// run since its last pass failed, the newest in the day before `now`; the
/// net ran it again, or its subtests alone, and that failed too; and main
/// hasn't answered its newest real failure. A run its VM died under counts
/// as a failure, so a merge that kills the VM still counts, but it answers
/// nothing and names no subtest. Logs are read only for a check that's
/// failed twice since passing, newest first, so an hour opens a few.
fn still_broken(
    check: &str,
    runs: &[Run],
    now: i64,
    by_check: &BTreeMap<String, Vec<Run>>,
    main: &Main,
    failed: &dyn Fn(&str) -> Failures,
) -> bool {
    let Some(newest) = runs.last().filter(|run| !run.passed && run.at > now - DAY) else {
        return false;
    };
    let Some(pass) = runs.iter().rposition(|run| run.passed) else {
        return false;
    };
    let since = &runs[pass + 1..];
    let alone = by_check
        .iter()
        .filter(|(name, _)| {
            name.split_once("::")
                .is_some_and(|(whole, _)| whole == check)
        })
        .flat_map(|(_, ran)| ran)
        .filter(|run| !run.passed && run.key > runs[pass].key)
        .count();
    if since.len() + alone < 2 {
        return false;
    }
    if check.contains("::") {
        return !answered(check, newest, &[], by_check, main);
    }
    for (died, run) in since
        .iter()
        .rev()
        .filter(|run| !run.log.is_empty())
        .enumerate()
    {
        let read = failed(&run.log);
        if !read.died {
            return died >= 2 || !answered(check, run, &read.subtests, by_check, main);
        }
    }
    true
}

/// Whether main has answered a check's failure `last` since. A subtest's is
/// answered by its whole test passing after it, which ran it again. A whole
/// test's is answered when each of `subtests`, those its log says failed,
/// is quarantined or passed alone after it. A log that names no failed
/// subtest answers nothing, so the check still counts.
fn answered(
    check: &str,
    last: &Run,
    subtests: &[String],
    by_check: &BTreeMap<String, Vec<Run>>,
    main: &Main,
) -> bool {
    let passed_after = |runs: &[Run]| runs.iter().any(|run| run.passed && run.key > last.key);
    if let Some((whole, _)) = check.split_once("::") {
        return by_check.get(whole).is_some_and(|runs| passed_after(runs));
    }
    !subtests.is_empty()
        && subtests.iter().all(|subtest| {
            main.quarantined
                .contains(&(check.to_owned(), subtest.clone()))
                || by_check.iter().any(|(name, runs)| {
                    name.split_once("::").is_some_and(|(whole, ran)| {
                        whole == check && ran.split("::").any(|one| one == subtest)
                    }) && passed_after(runs)
                })
        })
}

/// One of pytest's closing lines, `FAILED <file>::<subtest> - <why>` or
/// `ERROR ...`: the subtest, whether it failed rather than erred, and
/// whether its VM went away under it.
struct Closing<'a> {
    subtest: &'a str,
    failed: bool,
    lost: bool,
}

/// The closing lines in `log`: those that open with the mark, or whose
/// builder's prefix, up to its first `> `, is followed by it, as pytest
/// prints them. A guest's console line that only holds a mark is none.
fn closing(log: &str) -> impl Iterator<Item = Closing<'_>> {
    log.lines().filter_map(|line| {
        let bodies = [Some(line), line.split_once("> ").map(|(_, body)| body)];
        let (failed, rest) = bodies.into_iter().flatten().find_map(|body| {
            [("FAILED ", true), ("ERROR ", false)]
                .into_iter()
                .find_map(|(mark, failed)| Some((failed, body.strip_prefix(mark)?)))
        })?;
        let named = rest.split_whitespace().next()?;
        let (_, subtest) = named.rsplit_once("::")?;
        Some(Closing {
            subtest,
            failed,
            lost: rest[named.len()..].starts_with(BROKEN_PIPE),
        })
    })
}

/// The subtests a VM test's log says failed on their own checks, from
/// pytest's closing lines, not those its VM went away under.
#[must_use]
pub fn failed_subtests(log: &str) -> Vec<String> {
    let mut found: Vec<String> = closing(log)
        .filter(|line| line.failed && !line.lost)
        .map(|line| line.subtest.to_owned())
        .collect();
    found.sort();
    found.dedup();
    found
}

/// What a run's log says failed: the subtests that failed on their own
/// checks, and whether the VM died under the run instead.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Failures {
    pub subtests: Vec<String>,
    pub died: bool,
}

/// What pytest says of a subtest pending when the test driver lost its
/// machine, as merge-net reads it: nothing the subtest checked failed. On
/// 16 November 20 of the net's logs failed only so.
const BROKEN_PIPE: &str = " - BrokenPipeError";

/// What `log` says failed. The VM died when every subtest pytest closes
/// with as failed or erred went so on a broken pipe.
#[must_use]
pub fn failures(log: &str) -> Failures {
    let (mut any, mut lost) = (false, true);
    for line in closing(log) {
        any = true;
        lost &= line.lost;
    }
    Failures {
        subtests: failed_subtests(log),
        died: any && lost,
    }
}

/// What the log kept at `path` says failed; a log that can't be read says
/// nothing failed.
#[must_use]
pub fn failed_in(path: &str) -> Failures {
    fs::read_to_string(path)
        .map(|log| failures(&log))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_794_000_000;

    fn result(check: &str, commit: &str, at: i64, passed: bool, log: &str) -> String {
        serde_json::json!({
            "check": check, "commit": commit, "at": NOW + at, "passed": passed,
            "stalled": false, "log": log,
        })
        .to_string()
    }

    fn none(_: &str) -> Failures {
        Failures::default()
    }

    #[test]
    fn a_check_failing_after_it_passed_is_broken_and_a_stall_proves_nothing() {
        let stalled = serde_json::json!({"check": "stalled", "at": NOW - 60, "passed": false,
            "stalled": true})
        .to_string();
        let text = [
            result("broke", "", -900, true, ""),
            result("broke", "", -120, false, ""),
            result("broke", "", -60, false, ""),
            result("held", "", -900, true, ""),
            result("held", "", -600, false, ""),
            result("held", "", -300, false, ""),
            result("held", "", -60, false, ""),
            result("always", "", -900, false, ""),
            result("always", "", -60, false, ""),
            result("stalled", "", -900, true, ""),
            stalled,
            result("mended", "", -900, false, ""),
            result("mended", "", -60, true, ""),
            result("whole::sub", "", -900, true, ""),
            result("whole::sub", "", -600, false, ""),
            result("whole::sub", "", -500, false, ""),
            result("whole", "", -300, true, ""),
            result("later::sub", "", -900, true, ""),
            result("later", "", -600, true, ""),
            result("later::sub", "", -300, false, ""),
            result("later::sub", "", -200, false, ""),
        ]
        .join("\n");
        assert_eq!(
            broken(&text, NOW, "D5QEKR", &Main::default(), &none).behind,
            ["broke", "held", "later::sub"],
            "only those a merge broke, however many runs failed since: a subtest's fail gives \
             way to its whole test passing since"
        );
    }

    #[test]
    fn a_failure_the_net_has_not_run_again_is_not_yet_broken() {
        let failed = |log: &str| Failures {
            subtests: if log == "real.log" {
                vec!["test_v".to_owned()]
            } else {
                Vec::new()
            },
            died: false,
        };
        let text = [
            result("once::test_v", "", -950, false, ""),
            result("once", "", -900, true, ""),
            result("once", "", -60, false, "real.log"),
            result("oncemore::test_v", "", -30, false, ""),
            result("alone", "", -900, true, ""),
            result("alone", "", -600, false, "real.log"),
            result("alone::test_v", "", -300, false, ""),
        ]
        .join("\n");
        assert_eq!(
            broken(&text, NOW, "D5QEKR", &Main::default(), &failed).behind,
            ["alone"],
            "a failure counted before the net ran it again, or one its subtest failed again \
             alone didn't"
        );
    }

    #[test]
    fn a_rerun_of_an_older_merge_failing_after_a_newer_one_passed_is_not_broken() {
        let main = Main::of("c3\nc2\nc1\n", "[]");
        let text = [
            result("stale", "c1", -900, true, ""),
            result("stale", "c3", -600, true, ""),
            result("stale", "c2", -60, false, ""),
            result("stale", "c2", -30, false, ""),
            result("broke", "c2", -900, true, ""),
            result("broke", "c3", -600, false, ""),
            result("broke", "c3", -300, false, ""),
        ]
        .join("\n");
        assert_eq!(
            broken(&text, NOW, "D5QEKR", &main, &none).behind,
            ["broke"],
            "a failure kept late for an older merge counted though a newer one passed"
        );
    }

    #[test]
    fn a_commit_main_does_not_hold_within_the_merges_read_is_not_mains() {
        let merges = format!("c2 {}\nc1 {}\n", NOW - 120, NOW - 1000);
        let main = Main::of(&merges, "[]");
        let text = [
            result("ancient", "gone", -2000, true, ""),
            result("ancient", "c2", -800, false, ""),
            result("ancient", "c2", -700, false, ""),
            result("trial", "c1", -900, true, ""),
            result("trial", "c2", -800, true, ""),
            result("trial", "revert-trial", -600, false, ""),
            result("trial", "revert-trial", -300, false, ""),
            result("unfetched", "c2", -800, true, ""),
            result("unfetched", "c3", -90, false, ""),
            result("unfetched", "c3", -60, false, ""),
        ]
        .join("\n");
        assert_eq!(
            broken(&text, NOW, "D5QEKR", &main, &none).behind,
            ["ancient", "unfetched"],
            "a commit main doesn't hold within the merges read, like a revert's trial, \
             counted as main's, or a merge newer than main's head, or a pass older than the \
             merges read, didn't"
        );
    }

    #[test]
    fn a_failure_main_quarantined_or_saw_pass_alone_since_is_not_broken() {
        let quarantine = r#"[{"test": "held", "subtests": ["test_q"]},
            {"test": "half", "subtests": ["test_q"]}]"#;
        let main = Main::of("", quarantine);
        let failed = |log: &str| -> Failures {
            let names: &[&str] = match log {
                "held.log" => &["test_q"],
                "half.log" => &["test_q", "test_r"],
                "flaky.log" => &["test_f", "test_g"],
                _ => &[],
            };
            Failures {
                subtests: names.iter().map(|name| (*name).to_owned()).collect(),
                died: false,
            }
        };
        let text = [
            result("held", "", -900, true, ""),
            result("held", "", -600, false, "held.log"),
            result("held", "", -550, false, "held.log"),
            result("half", "", -900, true, ""),
            result("half", "", -600, false, "half.log"),
            result("half", "", -550, false, "half.log"),
            result("flaky", "", -900, true, ""),
            result("flaky", "", -600, false, "flaky.log"),
            result("flaky", "", -550, false, "flaky.log"),
            result("flaky::test_g::test_f", "", -300, true, ""),
            result("unread", "", -900, true, ""),
            result("unread", "", -600, false, "gone.log"),
            result("unread", "", -550, false, "gone.log"),
        ]
        .join("\n");
        assert_eq!(
            broken(&text, NOW, "D5QEKR", &main, &failed).behind,
            ["half", "unread"],
            "a failure whose every failed subtest is quarantined or passed alone since still \
             counted, or one with a subtest unanswered, or a log unread, didn't"
        );
    }

    #[test]
    fn a_run_the_vm_died_under_gives_way_to_the_failure_before_it() {
        let failed = |log: &str| -> Failures {
            match log {
                "real.log" => Failures {
                    subtests: vec!["test_v".to_owned()],
                    died: false,
                },
                "other.log" => Failures {
                    subtests: vec!["test_w".to_owned()],
                    died: false,
                },
                "died.log" => Failures {
                    subtests: Vec::new(),
                    died: true,
                },
                _ => Failures::default(),
            }
        };
        let text = [
            result("answered", "", -900, true, ""),
            result("answered", "", -600, false, "real.log"),
            result("answered::test_v", "", -400, true, ""),
            result("answered", "", -300, false, "died.log"),
            result("killed", "", -900, true, ""),
            result("killed", "", -600, false, "died.log"),
            result("killed", "", -60, false, "died.log"),
            result("killed_after", "", -900, true, ""),
            result("killed_after", "", -600, false, "real.log"),
            result("killed_after::test_v", "", -500, true, ""),
            result("killed_after", "", -400, false, "died.log"),
            result("killed_after", "", -300, false, "died.log"),
            result("newest", "", -900, true, ""),
            result("newest", "", -600, false, "real.log"),
            result("newest::test_v", "", -500, true, ""),
            result("newest", "", -400, false, "other.log"),
            result("under", "", -900, true, ""),
            result("under", "", -600, false, "died.log"),
            result("under", "", -300, false, "real.log"),
            result("unanswered", "", -900, true, ""),
            result("unanswered", "", -600, false, "real.log"),
            result("unanswered", "", -300, false, "died.log"),
        ]
        .join("\n");
        assert_eq!(
            broken(&text, NOW, "D5QEKR", &Main::default(), &failed).behind,
            ["killed", "killed_after", "newest", "unanswered", "under"],
            "a run the VM died under answered a failure, or hid one, or a merge that kills \
             the VM every time didn't count, even after an answered flake, or an older real \
             failure decided rather than the newest"
        );
    }

    #[test]
    fn a_log_whose_every_failure_is_a_broken_pipe_says_the_vm_died() {
        let died = "run> FAILED vm-pytest-v/x-vm-test.py::test_a - BrokenPipeError: [Errno 32]\n\
                    run> ERROR vm-pytest-v/x-vm-test.py::test_b - BrokenPipeError: [Errno 32]\n";
        let mixed = "run> FAILED vm-pytest-v/x-vm-test.py::test_a - AssertionError: no\n\
                     run> FAILED vm-pytest-v/x-vm-test.py::test_b - BrokenPipeError: [Errno 32]\n";
        let erred = "run> FAILED vm-pytest-v/x-vm-test.py::test_a - BrokenPipeError: [Errno 32]\n\
                     run> ERROR vm-pytest-v/x-vm-test.py::test_a - RuntimeError: no machine\n";
        let quoted =
            "run> FAILED vm-pytest-v/x-vm-test.py::test_a - AssertionError: BrokenPipeError\n";
        let guest = "run> client # ERROR x-vm-test.py::test_a - boom\n\
                     run> FAILED vm-pytest-v/x-vm-test.py::test_a - BrokenPipeError: [Errno 32]\n";
        let says_failed =
            "run> ERROR vm-pytest-v/x-vm-test.py::test_a - RuntimeError: FAILED a::b\n";
        assert_eq!(
            [died, mixed, erred, quoted, guest, says_failed, ""].map(failures),
            [
                Failures {
                    subtests: Vec::new(),
                    died: true
                },
                Failures {
                    subtests: vec!["test_a".to_owned()],
                    died: false
                },
                Failures {
                    subtests: Vec::new(),
                    died: false
                },
                Failures {
                    subtests: vec!["test_a".to_owned()],
                    died: false
                },
                Failures {
                    subtests: Vec::new(),
                    died: true
                },
                Failures::default(),
                Failures::default(),
            ],
            "a broken pipe was read as a subtest's own failure, or a real failure or error, \
             or one quoting a broken pipe, or a log with no closing line, as the VM dying, \
             or the guest's console as pytest's closing lines"
        );
    }

    #[test]
    fn a_logs_failed_subtests_are_read_from_pytests_closing_lines() {
        let log = "run> ____ test_a ____\n\
                   run> FAILED vm-pytest-g6/x-vm-test.py::test_b - AssertionError: no\n\
                   run> FAILED vm-pytest-g6/x-vm-test.py::test_a\n\
                   run> 1.07s call     x-vm-test.py::test_c\n";
        assert_eq!(
            failed_subtests(log),
            ["test_a", "test_b"],
            "the failed subtests weren't read from pytest's FAILED lines alone"
        );
    }
}
