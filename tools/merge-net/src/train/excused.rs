//! A red that is main's own excuses the candidate: when every check its
//! gate broke fails at the base it was cut on too, each subtest it failed
//! among those failing there, the riders broke nothing and land. On 22
//! November at 16:4x two riders in a row fell on the same four nested-niri
//! checks, which count frames under builder load, though neither touched
//! them; each then ran again alone and was held on them again.
//!
//! The base's word on a check comes first from the watch's own runs of it
//! there, then from one run of it on a lent builder, kept in
//! `train/base/<base>.json` so the next candidate on that base asks none.
//! A red the gate's words don't wholly explain by checks that failed to
//! build, such as an evaluation that failed, or one breaking more than
//! `MOST` checks, is never excused, and neither is a check whose run at
//! the base says nothing of it.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

use crate::state::{Ran, State, short};
use crate::subtests::failed_in;

/// The most checks a red may break and still be asked of the base: a red
/// breaking more is far likelier its riders', and each check asked costs
/// a builder's run while the train waits.
const MOST: usize = 4;

/// The system the train's gate checks, whose checks the base runs.
const SYSTEM: &str = "x86_64-linux";

/// What nix says of a derivation it couldn't build.
const CANNOT_BUILD: &str = "error: Cannot build '/nix/store/";

/// Why nix says a derivation failed of itself, not for a dependency.
const BUILDER_FAILED: &str = "Reason: builder failed";

/// The errors nix sums a failed build up with, which name nothing more
/// than its `Cannot build` lines already do.
const SUMMARIES: [&str; 2] = [
    "error: Build failed due to failed dependency",
    "error: build of '",
];

/// What a failed check did at the candidate's base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum AtBase {
    /// It failed there too, these subtests failing.
    Fails(BTreeSet<String>),
    /// It passed there.
    Passes,
}

/// The checks a red gate's `words` say failed of themselves, each by its
/// derivation's name, leaving out those that failed only for one of them;
/// none when its words hold an error no failed build explains, so that
/// nothing they say can be excused.
pub(super) fn broken(words: &str) -> Option<Vec<String>> {
    let lines: Vec<&str> = words.lines().map(str::trim).collect();
    let mut checks = BTreeSet::new();
    for (at, line) in lines.iter().enumerate() {
        if let Some(rest) = line.strip_prefix(CANNOT_BUILD) {
            let (path, _) = rest.split_once(".drv'")?;
            let (_, name) = path.split_once('-')?;
            if lines
                .get(at + 1)
                .is_some_and(|next| next.starts_with(BUILDER_FAILED))
            {
                checks.insert(name.to_owned());
            }
        } else if line.starts_with("error:")
            && !SUMMARIES.iter().any(|summary| line.starts_with(summary))
        {
            return None;
        }
    }
    Some(checks.into_iter().collect())
}

/// The subtests `words` say `check` failed, read from the lines nix
/// prefixes with its name.
pub(super) fn failed_by(words: &str, check: &str) -> BTreeSet<String> {
    let prefix = format!("{check}> ");
    failed_in(words.lines().filter(|line| line.starts_with(&prefix)))
}

/// The checks that excuse a red gate whose words are `words`, each failing
/// at the base as `at_base` says, with every subtest the gate failed
/// failing there too; none when any of them doesn't, or when the red
/// can't be excused at all.
pub(super) fn excused(
    words: &str,
    at_base: impl Fn(&str) -> Option<AtBase> + Sync,
) -> Option<Vec<String>> {
    let checks = broken(words)?;
    if checks.is_empty() || checks.len() > MOST {
        return None;
    }
    let answers: Vec<Option<AtBase>> = std::thread::scope(|scope| {
        let asked: Vec<_> = checks
            .iter()
            .map(|check| scope.spawn(|| at_base(check)))
            .collect();
        asked
            .into_iter()
            .map(|answer| answer.join().ok().flatten())
            .collect()
    });
    let all = checks
        .iter()
        .zip(&answers)
        .all(|(check, answer)| match answer {
            Some(AtBase::Fails(there)) => failed_by(words, check).is_subset(there),
            _ => false,
        });
    all.then_some(checks)
}

/// The file the base's answers are kept in, under the train's `folder`.
fn kept_at(folder: &Path, base: &str) -> PathBuf {
    folder.join("base").join(format!("{base}.json"))
}

/// What the train's `folder` keeps of `check` at `base`.
fn kept(folder: &Path, base: &str, check: &str) -> Option<AtBase> {
    let text = fs::read_to_string(kept_at(folder, base)).ok()?;
    let all: Value = serde_json::from_str(&text).ok()?;
    let answer = all.get(check)?;
    if answer["fails"] == true {
        let subtests = answer["subtests"]
            .as_array()?
            .iter()
            .filter_map(|subtest| subtest.as_str().map(str::to_owned))
            .collect();
        Some(AtBase::Fails(subtests))
    } else {
        Some(AtBase::Passes)
    }
}

/// Keep `answer` for `check` at `base` in the train's `folder`, beside the
/// others there.
fn keep(folder: &Path, base: &str, check: &str, answer: &AtBase) {
    let path = kept_at(folder, base);
    let mut all: BTreeMap<String, Value> = fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    let written = match answer {
        AtBase::Fails(subtests) => json!({ "fails": true, "subtests": subtests }),
        AtBase::Passes => json!({ "fails": false }),
    };
    all.insert(check.to_owned(), written);
    let kept = path
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fs::write(&path, format!("{}\n", json!(all))));
    if let Err(error) = kept {
        eprintln!("merge-net: {check}'s run at {base} wasn't kept: {error}");
    }
}

/// What the watch's runs of `check` at `base` say: that it failed there,
/// with the subtests its failed runs' logs name, or that it passed there
/// and never failed; nothing when it never ran there to an end.
fn watched(state: &State, base: &str, check: &str) -> Option<AtBase> {
    let runs = state.runs(base, check);
    if runs.contains(&Ran::Failed) {
        let subtests = state
            .failed_logs(base, check)
            .iter()
            .flat_map(|log| failed_in(crate::world::lines_of(log)))
            .collect();
        return Some(AtBase::Fails(subtests));
    }
    runs.contains(&Ran::Passed).then_some(AtBase::Passes)
}

/// What one run of `check` at `base` of the repository `slug` on a lent
/// builder says, its words kept under the train's `folder`: nothing when
/// no builder took it, or it failed for anything but the check itself.
fn ran(folder: &Path, slug: &str, base: &str, check: &str) -> Option<AtBase> {
    let log = folder.join("base").join(format!("{base}-{check}.log"));
    fs::create_dir_all(log.parent()?).ok()?;
    let words = fs::File::create(&log).ok()?;
    let errors = words.try_clone().ok()?;
    let status = Command::new("nix-offload")
        .args([
            "there",
            "--lent-only",
            "-L",
            &format!("github:{slug}/{base}"),
            &format!("checks.{SYSTEM}.{check}"),
        ])
        .env_remove("SSH_AUTH_SOCK")
        .stdin(Stdio::null())
        .stdout(words)
        .stderr(errors)
        .status()
        .ok()?;
    if status.success() {
        return Some(AtBase::Passes);
    }
    let said = String::from_utf8_lossy(&fs::read(&log).ok()?).into_owned();
    broken(&said)?
        .contains(&check.to_owned())
        .then(|| AtBase::Fails(failed_by(&said, check)))
}

/// The checks that excuse the red gate of `candidate`, whose words are in
/// `log`, by failing at `base` of the repository `slug` as they failed
/// there, said as the train lands it; none when they don't.
pub(super) fn mains_own(
    root: &Path,
    slug: &str,
    base: &str,
    candidate: &str,
    log: &Path,
) -> Option<Vec<String>> {
    let state = State::at(root.to_path_buf());
    let train = super::folder(root);
    let checks = excused(&super::gated::words_of(log), |check| {
        at_base(&train, &state, slug, base, check)
    })?;
    println!(
        "merge-net: the candidate {} failed changed-checks only on {}, which fail at \
         main's {} too, so the train lands it",
        short(candidate),
        checks.join(", "),
        short(base)
    );
    Some(checks)
}

/// What `check` does at `base` of the repository `slug`: as the train's
/// `folder` keeps it, else as the watch's runs in `state` say, else as one
/// run on a lent builder says, kept for the next candidate on that base.
/// A Mac's comparison runs nowhere but its own net, so it says nothing.
pub(super) fn at_base(
    folder: &Path,
    state: &State,
    slug: &str,
    base: &str,
    check: &str,
) -> Option<AtBase> {
    if crate::macs::base_of(check).is_some() {
        return None;
    }
    if let Some(answer) = kept(folder, base, check) {
        return Some(answer);
    }
    let answer = watched(state, base, check).or_else(|| ran(folder, slug, base, check))?;
    keep(folder, base, check, &answer);
    Some(answer)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fmt::Write as _;

    /// The words of a gate in which each of `failed` failed to build, its
    /// subtests failing as named, and each of `following` failed only for
    /// one of them.
    fn red(failed: &[(&str, &[&str])], following: &[&str]) -> String {
        let mut words = String::from("changed-checks: evaluates 3 of x86_64-linux's 9 checks\n");
        for (check, subtests) in failed {
            for subtest in *subtests {
                let _ = writeln!(
                    words,
                    "{check}> FAILED vm-pytest-ab/{check}-vm-test.py::{subtest} - Error"
                );
            }
            let _ = writeln!(
                words,
                "error: Cannot build '/nix/store/0x1c-{check}.drv'.\n       Reason: builder \
                 failed with exit code 1.\n       Output paths:"
            );
        }
        for check in following {
            let _ = writeln!(
                words,
                "error: Cannot build '/nix/store/j5fh-{check}.drv'.\n       Reason: 1 \
                 dependency failed."
            );
        }
        words.push_str("error: Build failed due to failed dependency\n");
        words.push_str("error: build of '/nix/store/0x1c-x.drv^*' failed\n");
        words.push_str("\u{2715} a check the change touched failed\n");
        words
    }

    fn named(subtests: &[&str]) -> BTreeSet<String> {
        subtests.iter().map(|&subtest| subtest.to_owned()).collect()
    }

    #[test]
    fn a_red_whose_checks_all_fail_at_the_base_is_excused() {
        let words = red(
            &[("cat-on-niri", &["test_frames"]), ("launcher", &[])],
            &["all"],
        );
        let excused = excused(&words, |check| {
            Some(AtBase::Fails(if check == "cat-on-niri" {
                named(&["test_frames", "test_more"])
            } else {
                BTreeSet::new()
            }))
        });
        assert_eq!(
            excused,
            Some(vec!["cat-on-niri".to_owned(), "launcher".to_owned()]),
            "a red failing at main's own commit held its riders, or asked of a check that \
             failed only for another"
        );
    }

    #[test]
    fn a_check_that_passes_at_the_base_holds_the_red() {
        let words = red(&[("cat-on-niri", &[]), ("long-files", &[])], &[]);
        let excused = excused(&words, |check| {
            Some(if check == "long-files" {
                AtBase::Passes
            } else {
                AtBase::Fails(BTreeSet::new())
            })
        });
        assert_eq!(excused, None, "a check the riders broke was excused");
    }

    #[test]
    fn a_subtest_failing_only_on_the_candidate_holds_the_red() {
        let words = red(&[("cat-on-niri", &["test_frames", "test_keys"])], &[]);
        let excused = excused(&words, |_| Some(AtBase::Fails(named(&["test_frames"]))));
        assert_eq!(
            excused, None,
            "a subtest that passes at main's commit was excused for another failing there"
        );
    }

    #[test]
    fn a_base_that_says_nothing_holds_the_red() {
        let words = red(&[("cat-on-niri", &[])], &[]);
        assert_eq!(
            excused(&words, |_| None),
            None,
            "a check no run at the base spoke for was excused"
        );
    }

    #[test]
    fn a_red_no_failed_build_explains_is_never_asked_of_the_base() {
        let mut words = red(&[("cat-on-niri", &[])], &[]);
        words.push_str("error: attribute 'x' missing\n");
        let asked = std::sync::atomic::AtomicBool::new(false);
        let excused = excused(&words, |_| {
            asked.store(true, std::sync::atomic::Ordering::SeqCst);
            Some(AtBase::Fails(BTreeSet::new()))
        });
        assert_eq!(excused, None, "a red with an evaluation error was excused");
        assert!(
            !asked.load(std::sync::atomic::Ordering::SeqCst),
            "the base was asked of a red that can't be excused"
        );
    }

    #[test]
    fn a_red_breaking_more_than_the_most_is_never_asked_of_the_base() {
        let failed: Vec<(&str, &[&str])> = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|&check| (check, &[] as &[&str]))
            .collect();
        let words = red(&failed, &[]);
        assert_eq!(
            excused(&words, |_| Some(AtBase::Fails(BTreeSet::new()))),
            None,
            "a red breaking five checks was asked of the base"
        );
    }

    #[test]
    fn an_answer_kept_for_a_base_is_read_back_by_the_next_candidate() {
        let folder = super::super::tests::root("excused-kept");
        keep(
            &folder,
            "b1",
            "cat",
            &AtBase::Fails(named(&["test_frames"])),
        );
        keep(&folder, "b1", "long-files", &AtBase::Passes);
        let state = State::at(folder.join("state"));
        assert_eq!(
            at_base(&folder, &state, "o/r", "b1", "cat"),
            Some(AtBase::Fails(named(&["test_frames"]))),
            "a kept failure wasn't read back, or lost its subtests"
        );
        assert_eq!(
            at_base(&folder, &state, "o/r", "b1", "long-files"),
            Some(AtBase::Passes),
            "a kept pass wasn't read back beside a failure"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn the_watchs_failed_run_at_the_base_answers_with_its_subtests() {
        let folder = super::super::tests::root("excused-watched");
        let state = State::at(folder.join("state"));
        state.make().unwrap();
        let log = folder.join("cat.log");
        fs::write(
            &log,
            "cat> FAILED vm-pytest-ab/cat-vm-test.py::test_frames - Error\n",
        )
        .unwrap();
        state
            .record("b1", "cat", Ran::Failed, Some(1), &log, 60)
            .unwrap();
        assert_eq!(
            at_base(&folder, &state, "o/r", "b1", "cat"),
            Some(AtBase::Fails(named(&["test_frames"]))),
            "the watch's failed run at the base wasn't read, or lost its subtests"
        );
        assert_eq!(
            kept(&folder, "b1", "cat"),
            Some(AtBase::Fails(named(&["test_frames"]))),
            "the watch's answer wasn't kept for the next candidate"
        );
        let _ = fs::remove_dir_all(&folder);
    }
}
