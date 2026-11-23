//! A red that is main's own excuses the candidate: when its gate ran whole
//! and every check it broke fails at the base it was cut on too, every
//! line the candidate's build of it printed printed there, the riders
//! broke nothing and land. On 22 November at 16:4x two riders in a row
//! fell on the same four nested-niri checks, which count frames under
//! builder load, though neither touched them; each then ran again alone
//! and was held on them again.
//!
//! Lines, not subtests, since those checks aren't pytest and name none: a
//! failure naming nothing would be among anything the base failed with.
//! A line is read with its numbers and store hashes masked, so a count
//! that differs still matches, and a new error, file, trace or subtest
//! doesn't. A build that printed nothing is never excused.
//!
//! The base's word on a check comes first from the watch's runs of it
//! there, then from what a lent builder's run of it there said, else from
//! one such run, bounded by the watch's clock and kept in
//! `train/base/<base>/<check>.json` for the next candidate on that base.
//! A check that ever passed at the base holds the red, as does a failed
//! run there whose words don't say the check itself failed. A red the
//! gate's words don't wholly explain by checks that failed to build of
//! themselves, one breaking more than `MOST` checks, or one breaking a VM
//! test, whose derivation isn't named as its check is, is never asked of
//! the base. A file `train/no-excuse` turns every excuse off.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde_json::{Value, json};

use crate::state::{Ran, State, short};

/// The most checks a red may break and still be asked of the base: a red
/// breaking more is far likelier its riders', and each check asked costs
/// a builder's run while the train waits.
const MOST: usize = 4;

/// The most lines a check's failure at the base may hold and still be
/// compared and kept: a VM test's log runs to tens of megabytes, and
/// one this long is never one the excuse is for.
const MOST_LINES: usize = 20_000;

/// How long what the base said is kept, as long as the watch keeps a
/// run's log, since a base main has moved past is never asked again.
const KEPT_FOR: Duration = Duration::from_hours(72);

/// The system the train's gate checks, whose checks the base runs.
const SYSTEM: &str = "x86_64-linux";

/// What nix says of a derivation it couldn't build.
const CANNOT_BUILD: &str = "error: Cannot build '/nix/store/";

/// Why nix says a derivation failed of itself.
const BUILDER_FAILED: &str = "Reason: builder failed";

/// The start of nix's name for a NixOS VM test's derivation, which is
/// `vm-test-run-` and the test's name, not its check's.
const VM_TEST: &str = "vm-test-run-";

/// The errors nix sums a failed build up with, which name nothing more
/// than its `Cannot build` lines already do.
const SUMMARIES: [&str; 2] = [
    "error: Build failed due to failed dependency",
    "error: build of '",
];

/// What a failed check did at the candidate's base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum AtBase {
    /// It failed there in every run, each printing these lines, masked.
    Fails(BTreeSet<String>),
    /// It passed there, at least once.
    Passes,
}

/// The checks a red gate's `words` say failed of themselves, each by its
/// derivation's name, leaving out those that failed only for a dependency;
/// none when its words hold an error no failed build explains, or a
/// `Cannot build` for any other reason, so that nothing they say can be
/// excused.
pub(super) fn broken(words: &str) -> Option<Vec<String>> {
    let lines: Vec<&str> = words.lines().map(str::trim).collect();
    let mut checks = BTreeSet::new();
    for (at, line) in lines.iter().enumerate() {
        if let Some(rest) = line.strip_prefix(CANNOT_BUILD) {
            let (path, _) = rest.split_once(".drv'")?;
            let (_, name) = path.split_once('-')?;
            let reason = lines.get(at + 1).copied().unwrap_or_default();
            if reason.starts_with(BUILDER_FAILED) {
                checks.insert(name.to_owned());
            } else if !(reason.starts_with("Reason: ") && reason.contains("dependenc")) {
                return None;
            }
        } else if line.starts_with("error:")
            && !SUMMARIES.iter().any(|summary| line.starts_with(summary))
        {
            return None;
        }
    }
    Some(checks.into_iter().collect())
}

/// `line` with each store path's hash and each run of digits masked, so
/// two runs failing alike read alike.
fn masked(line: &str) -> String {
    const STORE: &str = "/nix/store/";
    const HASH: usize = 32;
    let mut unhashed = String::with_capacity(line.len());
    let mut rest = line.trim_end();
    while let Some(at) = rest.find(STORE) {
        let (before, after) = rest.split_at(at + STORE.len());
        unhashed.push_str(before);
        let hashed = after.len() >= HASH
            && after.as_bytes()[..HASH]
                .iter()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit());
        if hashed {
            unhashed.push('*');
            rest = &after[HASH..];
        } else {
            rest = after;
        }
    }
    unhashed.push_str(rest);
    let mut out = String::with_capacity(unhashed.len());
    for character in unhashed.chars() {
        if !character.is_ascii_digit() {
            out.push(character);
        } else if !out.ends_with('#') {
            out.push('#');
        }
    }
    out
}

/// The lines `words` say `check`'s build printed, those nix prefixes with
/// its name, masked, blank ones left out.
pub(super) fn said_by(words: &str, check: &str) -> BTreeSet<String> {
    let prefix = format!("{check}> ");
    words
        .lines()
        .filter_map(|line| line.strip_prefix(&prefix))
        .map(masked)
        .filter(|line| !line.trim().is_empty())
        .collect()
}

/// The checks that excuse a red gate whose words are `words`, each failing
/// at the base as `at_base` says, every line the gate printed of it
/// printed there; why not, when any of them doesn't, or when the red
/// can't be excused at all.
pub(super) fn excused(
    words: &str,
    at_base: impl Fn(&str) -> Option<AtBase> + Sync,
) -> Result<Vec<String>, String> {
    let checks = broken(words).ok_or("its words hold an error no failed build explains")?;
    if checks.is_empty() {
        return Err("no check failed of itself".to_owned());
    }
    if checks.len() > MOST {
        return Err(format!(
            "it broke {} checks, more than the {MOST} the base is asked of",
            checks.len()
        ));
    }
    if let Some(test) = checks.iter().find(|check| check.starts_with(VM_TEST)) {
        return Err(format!(
            "{test} is a VM test, whose derivation isn't named as its check is"
        ));
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
    for (check, answer) in checks.iter().zip(answers) {
        let said = said_by(words, check);
        match answer {
            None => return Err(format!("its base said nothing of {check}")),
            Some(AtBase::Passes) => return Err(format!("{check} passed at its base")),
            Some(AtBase::Fails(_)) if said.is_empty() => {
                return Err(format!("{check}'s build printed nothing to compare"));
            }
            Some(AtBase::Fails(there)) => {
                let new = said.difference(&there).count();
                if new > 0 {
                    return Err(format!(
                        "{check} printed {new} line(s) its base's failure didn't"
                    ));
                }
            }
        }
    }
    Ok(checks)
}

/// The folder what the train's `folder` keeps of `base` is in.
fn base_folder(folder: &Path, base: &str) -> PathBuf {
    folder.join("base").join(base)
}

/// What the train's `folder` keeps of `check` at `base`.
fn kept(folder: &Path, base: &str, check: &str) -> Option<AtBase> {
    let path = base_folder(folder, base).join(format!("{check}.json"));
    let answer: Value = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    if answer["fails"] == true {
        let said = answer["said"]
            .as_array()?
            .iter()
            .filter_map(|line| line.as_str().map(str::to_owned))
            .collect();
        Some(AtBase::Fails(said))
    } else {
        Some(AtBase::Passes)
    }
}

/// Keep `answer` for `check` at `base` in the train's `folder`, a file a
/// check written whole, so checks answered at once never write over each
/// other and a reader never sees half of one.
fn keep(folder: &Path, base: &str, check: &str, answer: &AtBase) {
    let at = base_folder(folder, base);
    let written = match answer {
        AtBase::Fails(said) => json!({ "fails": true, "said": said }),
        AtBase::Passes => json!({ "fails": false }),
    };
    let kept = fs::create_dir_all(&at)
        .map_err(|error| error.to_string())
        .and_then(|()| {
            crate::whole::write(&at.join(format!("{check}.json")), &format!("{written}\n"))
        });
    if let Err(error) = kept {
        eprintln!("merge-net: {check}'s run at {base} wasn't kept: {error}");
    }
}

/// Remove what the train's `folder` keeps of each base last written more
/// than `KEPT_FOR` ago, its lent runs' logs among it.
fn age_out(folder: &Path, now: SystemTime) {
    let Ok(bases) = fs::read_dir(folder.join("base")) else {
        return;
    };
    for base in bases.flatten() {
        let written = base.metadata().and_then(|meta| meta.modified());
        if written.is_ok_and(|at| now.duration_since(at).is_ok_and(|age| age > KEPT_FOR)) {
            let _ = fs::remove_dir_all(base.path());
        }
    }
}

/// What a failed run at the base whose words are `words` says of `check`:
/// the lines it printed, when they say the check itself failed and are
/// few enough to compare; nothing when it failed for anything else.
fn failure_of(words: &str, check: &str) -> Option<BTreeSet<String>> {
    if !broken(words)?.iter().any(|name| name == check) {
        return None;
    }
    let said = said_by(words, check);
    (said.len() <= MOST_LINES).then_some(said)
}

/// What the watch's runs of `check` at `base` say: that it passed there,
/// once is enough, or that it failed there in every run it ended, the
/// lines every one of them printed; nothing when it never ended a run
/// there, or a failed run's log is gone or says it failed for anything
/// but the check itself.
fn watched(state: &State, base: &str, check: &str) -> Option<AtBase> {
    let runs = state.runs(base, check);
    if runs.contains(&Ran::Passed) {
        return Some(AtBase::Passes);
    }
    let mut every: Option<BTreeSet<String>> = None;
    for log in state.failed_logs(base, check) {
        let words = crate::world::lines_of(&log).collect::<Vec<_>>().join("\n");
        let said = failure_of(&words, check)?;
        every = Some(match every {
            None => said,
            Some(before) => before.intersection(&said).cloned().collect(),
        });
    }
    every.map(AtBase::Fails)
}

/// What a lent builder's run of `check` at `base` of the repository
/// `slug` says, `run` running nix-offload with its words into a log under
/// the train's `folder` and saying how it exited: nothing when no builder
/// took it, it ran past its time, or it failed for anything but the check
/// itself.
fn ran(
    folder: &Path,
    slug: &str,
    base: &str,
    check: &str,
    run: &dyn Fn(&[String], &Path) -> Option<i32>,
) -> Option<AtBase> {
    let log = base_folder(folder, base).join(format!("{check}.log"));
    let args: Vec<String> = [
        "there",
        "--lent-only",
        "-L",
        &format!("github:{slug}/{base}"),
        &format!("checks.{SYSTEM}.{check}"),
    ]
    .map(str::to_owned)
    .into();
    let code = run(&args, &log)?;
    if code == 0 {
        return Some(AtBase::Passes);
    }
    if crate::watch::untaken(Some(code)) {
        return None;
    }
    let words = String::from_utf8_lossy(&fs::read(&log).ok()?).into_owned();
    failure_of(&words, check).map(AtBase::Fails)
}

/// The checks that excuse the red gate of `candidate`, whose words are in
/// `log`, by failing at `base` of the repository `slug` as they failed
/// there, said in the net's log as the train lands it; none when they
/// don't, with why said there too.
pub(super) fn mains_own(
    root: &Path,
    slug: &str,
    base: &str,
    candidate: &str,
    log: &Path,
) -> Option<Vec<String>> {
    let state = State::at(root.to_path_buf());
    let train = super::folder(root);
    if train.join("no-excuse").exists() {
        state.say(&format!(
            "the candidate {}'s red isn't asked of main: train/no-excuse turns excuses off",
            short(candidate)
        ));
        return None;
    }
    age_out(&train, SystemTime::now());
    let run = |args: &[String], log: &Path| crate::world::bounded(args, base, log);
    let checks = excused(&super::gated::words_of(log), |check| {
        at_base(&train, &state, slug, base, check, &run)
    });
    match checks {
        Ok(checks) => {
            state.say(&format!(
                "the candidate {} failed changed-checks only on {}, which fail at main's {} \
                 alike, so the train lands it",
                short(candidate),
                checks.join(", "),
                short(base)
            ));
            Some(checks)
        }
        Err(why) => {
            state.say(&format!(
                "the candidate {}'s red isn't main's own at {}: {why}",
                short(candidate),
                short(base)
            ));
            None
        }
    }
}

/// What `check` does at `base` of the repository `slug`: as the watch's
/// runs in `state` say, else as the train's `folder` keeps it, else as
/// one run on a lent builder by `run` says, kept for the next candidate
/// on that base. A Mac's comparison runs nowhere but its own net, so it
/// says nothing.
pub(super) fn at_base(
    folder: &Path,
    state: &State,
    slug: &str,
    base: &str,
    check: &str,
    run: &dyn Fn(&[String], &Path) -> Option<i32>,
) -> Option<AtBase> {
    if crate::macs::base_of(check).is_some() {
        return None;
    }
    if let Some(answer) = watched(state, base, check) {
        return Some(answer);
    }
    if let Some(answer) = kept(folder, base, check) {
        return Some(answer);
    }
    let answer = ran(folder, slug, base, check, run)?;
    keep(folder, base, check, &answer);
    Some(answer)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fmt::Write as _;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    /// The words of a gate in which each of `failed` failed to build,
    /// printing the lines named, and each of `following` failed only for
    /// one of them.
    fn red(failed: &[(&str, &[&str])], following: &[&str]) -> String {
        let mut words = String::from("changed-checks: evaluates 3 of x86_64-linux's 9 checks\n");
        for (check, printed) in failed {
            for line in *printed {
                let _ = writeln!(words, "{check}> {line}");
            }
            let _ = writeln!(words, "{}", cannot_build(check));
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

    /// What nix says of `check` failing to build of itself.
    fn cannot_build(check: &str) -> String {
        format!(
            "error: Cannot build '/nix/store/0x1c-{check}.drv'.\n       Reason: builder failed \
             with exit code 1.\n       Output paths:"
        )
    }

    fn lines(said: &[&str]) -> BTreeSet<String> {
        said.iter().map(|&line| line.to_owned()).collect()
    }

    /// A lent run that must not be made.
    fn never(_: &[String], _: &Path) -> Option<i32> {
        panic!("a lent builder was asked when the base had answered");
    }

    #[test]
    fn a_red_whose_checks_print_only_what_the_base_printed_is_excused() {
        let words = red(
            &[
                ("cat-on-niri", &["counted 12 frames of 30"]),
                ("launcher", &["starting niri", "no window in 9 s"]),
            ],
            &["all"],
        );
        let excused = excused(&words, |check| {
            Some(AtBase::Fails(if check == "cat-on-niri" {
                lines(&["starting niri", "counted # frames of #"])
            } else {
                lines(&["starting niri", "no window in # s"])
            }))
        });
        assert_eq!(
            excused,
            Ok(vec!["cat-on-niri".to_owned(), "launcher".to_owned()]),
            "a red failing at main's own commit alike held its riders, or asked of a check \
             that failed only for another"
        );
    }

    #[test]
    fn a_check_that_passes_at_the_base_holds_the_red() {
        let words = red(&[("cat-on-niri", &["x"]), ("long-files", &["x"])], &[]);
        let excused = excused(&words, |check| {
            Some(if check == "long-files" {
                AtBase::Passes
            } else {
                AtBase::Fails(lines(&["x"]))
            })
        });
        assert!(excused.is_err(), "a check the riders broke was excused");
    }

    /// A rider adding a second lint error to a check main already fails,
    /// or failing a subtest main passes, prints a line main's failure
    /// never did.
    #[test]
    fn a_line_the_base_never_printed_holds_the_red() {
        let words = red(
            &[(
                "long-files",
                &["a.nix runs 900 lines", "b.nix runs 812 lines"],
            )],
            &[],
        );
        let excused = excused(&words, |_| {
            Some(AtBase::Fails(lines(&["a.nix runs # lines"])))
        });
        assert!(
            excused.is_err(),
            "a new failure in a check main already fails was excused"
        );
    }

    /// A failure that names nothing would be among anything the base
    /// failed with, so it is never excused.
    #[test]
    fn a_build_that_printed_nothing_holds_the_red() {
        let words = red(&[("cat-on-niri", &[])], &[]);
        let excused = excused(&words, |_| Some(AtBase::Fails(lines(&["anything"]))));
        assert!(
            excused.is_err(),
            "a failure that printed nothing was excused by one at the base"
        );
    }

    #[test]
    fn a_base_that_says_nothing_holds_the_red() {
        let words = red(&[("cat-on-niri", &["x"])], &[]);
        assert!(
            excused(&words, |_| None).is_err(),
            "a check no run at the base spoke for was excused"
        );
    }

    /// Each asks the base nothing: an evaluation's error, a build nix
    /// gave up on for a reason the excuse can't read, a VM test, and more
    /// checks than `MOST`.
    #[test]
    fn a_red_the_excuse_cant_read_is_never_asked_of_the_base() {
        let failed: Vec<(&str, &[&str])> = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|&check| (check, &["x"] as &[&str]))
            .collect();
        let reds = [
            format!(
                "{}error: attribute 'x' missing\n",
                red(&[("cat", &["x"])], &[])
            ),
            "cat> x\nerror: Cannot build '/nix/store/0x1c-cat.drv'.\n       Reason: timed out \
             after 3600 seconds of silence\n"
                .to_owned(),
            red(&[("vm-test-run-desktop-looks", &["x"])], &[]),
            red(&failed, &[]),
        ];
        for words in reds {
            let asked = AtomicBool::new(false);
            let excused = excused(&words, |_| {
                asked.store(true, Ordering::SeqCst);
                Some(AtBase::Fails(lines(&["x"])))
            });
            assert!(
                excused.is_err(),
                "a red the excuse can't read was excused: {words}"
            );
            assert!(
                !asked.load(Ordering::SeqCst),
                "the base was asked of a red that can't be excused: {words}"
            );
        }
    }

    #[test]
    fn a_line_is_read_with_its_numbers_and_store_hashes_masked() {
        assert_eq!(
            masked("took 12.5 s in /nix/store/0123456789abcdfghijklmnpqrsvwxyz-niri/bin  "),
            "took #.# s in /nix/store/*-niri/bin",
            "a count or a hash that differs between runs failing alike kept them apart"
        );
        assert_eq!(
            masked("/nix/store/short-x"),
            "/nix/store/short-x",
            "a store path with no hash lost its name"
        );
    }

    #[test]
    fn answers_kept_at_once_for_a_base_are_all_read_back() {
        let folder = super::super::tests::root("excused-kept");
        std::thread::scope(|scope| {
            scope.spawn(|| keep(&folder, "b1", "cat", &AtBase::Fails(lines(&["x"]))));
            scope.spawn(|| keep(&folder, "b1", "long-files", &AtBase::Passes));
        });
        let state = State::at(folder.join("state"));
        assert_eq!(
            at_base(&folder, &state, "o/r", "b1", "cat", &never),
            Some(AtBase::Fails(lines(&["x"]))),
            "a kept failure wasn't read back, or lost its lines"
        );
        assert_eq!(
            at_base(&folder, &state, "o/r", "b1", "long-files", &never),
            Some(AtBase::Passes),
            "a pass kept at the same time as a failure wasn't read back"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn what_a_base_keeps_goes_once_main_is_long_past_it() {
        let folder = super::super::tests::root("excused-aged");
        keep(&folder, "b1", "cat", &AtBase::Passes);
        age_out(&folder, SystemTime::now());
        assert!(
            kept(&folder, "b1", "cat").is_some(),
            "a fresh answer was aged out"
        );
        age_out(
            &folder,
            SystemTime::now() + KEPT_FOR + Duration::from_secs(60),
        );
        assert!(
            kept(&folder, "b1", "cat").is_none(),
            "an answer for a base three days gone was kept"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    /// A watch at `folder` that ran `check` at `b1` once each as `runs`
    /// says, a failed run's log holding its words.
    fn watch_of(folder: &Path, runs: &[(Ran, &str)]) -> State {
        let state = State::at(folder.join("state"));
        state.make().unwrap();
        for (at, (ran, words)) in runs.iter().enumerate() {
            let log = folder.join(format!("cat-{at}.log"));
            fs::write(&log, words).unwrap();
            state.record("b1", "cat", *ran, Some(1), &log, 60).unwrap();
        }
        state
    }

    #[test]
    fn the_watchs_failed_runs_at_the_base_answer_with_the_lines_all_printed() {
        let folder = super::super::tests::root("excused-watched");
        let first = format!(
            "cat> counted 3 frames\ncat> slow\n{}\n",
            cannot_build("cat")
        );
        let second = format!("cat> counted 7 frames\n{}\n", cannot_build("cat"));
        let state = watch_of(&folder, &[(Ran::Failed, &first), (Ran::Failed, &second)]);
        assert_eq!(
            at_base(&folder, &state, "o/r", "b1", "cat", &never),
            Some(AtBase::Fails(lines(&["counted # frames"]))),
            "the watch's failed runs weren't read, or a line only one printed counted"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    /// A check flaky at the base, failing in some runs, may be the
    /// riders' when it fails every time on the candidate.
    #[test]
    fn a_check_the_watch_saw_pass_at_the_base_once_passes_there() {
        let folder = super::super::tests::root("excused-flaky");
        let failed = format!("cat> x\n{}\n", cannot_build("cat"));
        let state = watch_of(&folder, &[(Ran::Failed, &failed), (Ran::Passed, "")]);
        assert_eq!(
            at_base(&folder, &state, "o/r", "b1", "cat", &never),
            Some(AtBase::Passes),
            "a check that passed at the base once read as failing there"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    /// A failed run whose words don't say the check itself failed, as a
    /// dependency's failure, or whose log is gone, leaves the base to a
    /// lent run.
    #[test]
    fn a_watch_run_failing_for_anything_else_asks_a_lent_builder() {
        let folder = super::super::tests::root("excused-otherwise");
        let followed = "error: Cannot build '/nix/store/0x1c-cat.drv'.\n       Reason: 1 \
                        dependency failed.\n";
        let state = watch_of(&folder, &[(Ran::Failed, followed)]);
        let asked = AtomicUsize::new(0);
        let untaken = |_: &[String], _: &Path| {
            asked.fetch_add(1, Ordering::SeqCst);
            Some(75)
        };
        assert_eq!(
            at_base(&folder, &state, "o/r", "b1", "cat", &untaken),
            None,
            "a run no builder took said something of the base"
        );
        assert_eq!(asked.load(Ordering::SeqCst), 1, "no lent builder was asked");
        assert!(
            kept(&folder, "b1", "cat").is_none(),
            "a run no builder took was kept as an answer"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_lent_run_at_the_base_is_read_as_the_watchs_are_and_kept() {
        let folder = super::super::tests::root("excused-lent");
        let state = State::at(folder.join("state"));
        let cases: [(&str, i32, String, Option<AtBase>); 3] = [
            ("passes", 0, String::new(), Some(AtBase::Passes)),
            (
                "fails",
                1,
                format!("fails> counted 4 frames\n{}\n", cannot_build("fails")),
                Some(AtBase::Fails(lines(&["counted # frames"]))),
            ),
            (
                "follows",
                1,
                "error: Cannot build '/nix/store/0x1c-follows.drv'.\n       Reason: 1 \
                 dependency failed.\n"
                    .to_owned(),
                None,
            ),
        ];
        for (check, code, words, answer) in cases {
            let run = |args: &[String], log: &Path| {
                assert_eq!(
                    args.last().map(String::as_str),
                    Some(format!("checks.{SYSTEM}.{check}").as_str()),
                    "the lent run asked for another check"
                );
                fs::create_dir_all(log.parent().unwrap()).unwrap();
                fs::write(log, &words).unwrap();
                Some(code)
            };
            assert_eq!(
                at_base(&folder, &state, "o/r", "b1", check, &run),
                answer,
                "a lent run that exited {code} was misread: {words}"
            );
            assert_eq!(
                kept(&folder, "b1", check),
                answer,
                "a lent run's answer wasn't kept as it was read: {check}"
            );
        }
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_net_told_no_excuse_asks_the_base_nothing() {
        let root = super::super::tests::root("excused-off");
        let train = super::super::folder(&root);
        fs::create_dir_all(&train).unwrap();
        fs::write(train.join("no-excuse"), "").unwrap();
        let log = root.join("candidate.log");
        fs::write(&log, red(&[("cat", &["x"])], &[])).unwrap();
        assert_eq!(
            mains_own(&root, "o/r", "b1", "c1", &log),
            None,
            "a red was excused with excuses turned off"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
