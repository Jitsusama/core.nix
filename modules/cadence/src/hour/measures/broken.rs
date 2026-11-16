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
                "rev-list",
                "--first-parent",
                "-n",
                MERGES,
                "main",
            ],
        )
        .unwrap_or_default();
        Self::of(&merges, quarantine)
    }

    /// Main from `git rev-list`'s lines, newest first, and the quarantine's
    /// JSON.
    #[must_use]
    pub fn of(merges: &str, quarantine: &str) -> Self {
        let place = merges
            .lines()
            .rev()
            .enumerate()
            .map(|(at, commit)| (commit.trim().to_owned(), at))
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
        Self { place, quarantined }
    }

    /// Where a result sits: by its merge's place on main, then by when the
    /// net kept it. A merge the mirror doesn't hold yet is newer than it,
    /// so its results sort last.
    fn key(&self, commit: &str, at: i64) -> (usize, i64) {
        (self.place.get(commit).copied().unwrap_or(usize::MAX), at)
    }
}

/// One run of a check, as the net kept it.
struct Run {
    key: (usize, i64),
    at: i64,
    passed: bool,
    log: String,
}

/// The checks still failing on main that last passed before a merge in
/// the day before `now`, from the net's `results.jsonl`: a check whose
/// newest result on main failed where the one before it passed. A stalled
/// run proves nothing either way, so it's left out. `failed` reads the
/// subtests a run's log says failed.
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
        by_check.entry(check.to_owned()).or_default().push(Run {
            key: main.key(result["commit"].as_str().unwrap_or_default(), at),
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
        .filter_map(|(check, runs)| {
            let [.., before, last] = alive(runs, failed) else {
                return None;
            };
            (before.passed
                && !last.passed
                && last.at > now - DAY
                && !answered(check, last, &by_check, main, failed))
            .then(|| check.clone())
        })
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

/// A check's runs without the newest that the VM died under: like a stall,
/// such a run proves nothing, so the run before it decides. Only the newest
/// failures' logs are read, so an hour opens a few logs, not thousands.
fn alive<'a>(runs: &'a [Run], failed: &dyn Fn(&str) -> Failures) -> &'a [Run] {
    let mut runs = runs;
    while let [rest @ .., last] = runs {
        if last.passed || last.log.is_empty() || !failed(&last.log).died {
            break;
        }
        runs = rest;
    }
    runs
}

/// Whether main has answered a check's last failure since. A subtest's is
/// answered by its whole test passing after it, which ran it again. A whole
/// test's is answered when each subtest its log says failed is quarantined,
/// or passed alone after it. A log that names no failed subtest answers
/// nothing, so the check still counts.
fn answered(
    check: &str,
    last: &Run,
    by_check: &BTreeMap<String, Vec<Run>>,
    main: &Main,
    failed: &dyn Fn(&str) -> Failures,
) -> bool {
    let passed_after = |runs: &[Run]| runs.iter().any(|run| run.passed && run.key > last.key);
    if let Some((whole, _)) = check.split_once("::") {
        return by_check.get(whole).is_some_and(|runs| passed_after(runs));
    }
    let subtests = failed(&last.log).subtests;
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

/// The subtests a VM test's log says failed, from pytest's closing lines,
/// `FAILED <file>::<subtest> - <why>`, but those its VM died under.
#[must_use]
pub fn failed_subtests(log: &str) -> Vec<String> {
    let mut found: Vec<String> = log
        .lines()
        .filter(|line| !line.contains(BROKEN_PIPE))
        .filter_map(|line| {
            let (_, rest) = line.split_once("FAILED ")?;
            let (_, subtest) = rest.split_whitespace().next()?.rsplit_once("::")?;
            Some(subtest.to_owned())
        })
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

/// What pytest says of a subtest whose VM went away under it: the test
/// driver's pipe to the machine broke, so nothing the subtest checked
/// failed. On 16 November 20 of the net's logs failed only so.
const BROKEN_PIPE: &str = "BrokenPipeError";

/// What `log` says failed. The VM died when every subtest pytest closes
/// with as failed or erred did so on a broken pipe.
#[must_use]
pub fn failures(log: &str) -> Failures {
    let closing: Vec<&str> = log
        .lines()
        .filter(|line| {
            ["FAILED ", "ERROR "].iter().any(|mark| {
                line.split_once(mark)
                    .is_some_and(|(_, rest)| rest.contains("::"))
            })
        })
        .collect();
    Failures {
        subtests: failed_subtests(log),
        died: !closing.is_empty() && closing.iter().all(|line| line.contains(BROKEN_PIPE)),
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
            result("broke", "", -60, false, ""),
            result("always", "", -900, false, ""),
            result("always", "", -60, false, ""),
            result("stalled", "", -900, true, ""),
            stalled,
            result("mended", "", -900, false, ""),
            result("mended", "", -60, true, ""),
            result("whole::sub", "", -900, true, ""),
            result("whole::sub", "", -600, false, ""),
            result("whole", "", -300, true, ""),
            result("later::sub", "", -900, true, ""),
            result("later", "", -600, true, ""),
            result("later::sub", "", -300, false, ""),
        ]
        .join("\n");
        assert_eq!(
            broken(&text, NOW, "D5QEKR", &Main::default(), &none).behind,
            ["broke", "later::sub"],
            "only those a merge broke: a subtest's fail gives way to its whole test passing since"
        );
    }

    #[test]
    fn a_rerun_of_an_older_merge_failing_after_a_newer_one_passed_is_not_broken() {
        let main = Main::of("c3\nc2\nc1\n", "[]");
        let text = [
            result("stale", "c1", -900, true, ""),
            result("stale", "c3", -600, true, ""),
            result("stale", "c2", -60, false, ""),
            result("broke", "c2", -900, true, ""),
            result("broke", "c3", -600, false, ""),
        ]
        .join("\n");
        assert_eq!(
            broken(&text, NOW, "D5QEKR", &main, &none).behind,
            ["broke"],
            "a failure kept late for an older merge counted though a newer one passed"
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
            result("half", "", -900, true, ""),
            result("half", "", -600, false, "half.log"),
            result("flaky", "", -900, true, ""),
            result("flaky", "", -600, false, "flaky.log"),
            result("flaky::test_g::test_f", "", -300, true, ""),
            result("unread", "", -900, true, ""),
            result("unread", "", -600, false, "gone.log"),
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
            result("died", "", -900, true, ""),
            result("died", "", -60, false, "died.log"),
            result("unanswered", "", -900, true, ""),
            result("unanswered", "", -600, false, "real.log"),
            result("unanswered", "", -300, false, "died.log"),
        ]
        .join("\n");
        assert_eq!(
            broken(&text, NOW, "D5QEKR", &Main::default(), &failed).behind,
            ["unanswered"],
            "a run the VM died under was taken for the newest failure, or hid a real one"
        );
    }

    #[test]
    fn a_log_whose_every_failure_is_a_broken_pipe_says_the_vm_died() {
        let died = "run> FAILED vm-pytest-v/x-vm-test.py::test_a - BrokenPipeError: [Errno 32]\n\
                    run> ERROR vm-pytest-v/x-vm-test.py::test_b - BrokenPipeError: [Errno 32]\n";
        let mixed = "run> FAILED vm-pytest-v/x-vm-test.py::test_a - AssertionError: no\n\
                     run> FAILED vm-pytest-v/x-vm-test.py::test_b - BrokenPipeError: [Errno 32]\n";
        assert_eq!(
            (failures(died), failures(mixed)),
            (
                Failures {
                    subtests: Vec::new(),
                    died: true
                },
                Failures {
                    subtests: vec!["test_a".to_owned()],
                    died: false
                },
            ),
            "a broken pipe was read as a subtest's own failure, or a real one as the VM dying"
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
