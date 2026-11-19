//! A search's runs cut down to the subtests that fail at its head. On
//! 11 November vm-desktop-looks' search ran its 20 subtests at each step,
//! 30 to 67 minutes a run, so five hours passed before it named the merge
//! that broke one of them; other subtests of it, flaky at every commit,
//! moved the failure about meanwhile. A search instead runs only the
//! subtests that failed in both of the head's runs, as `run-subtests
//! --there` builds them, and keeps the whole test where none can be named.
//!
//! A narrowed check is written `test::subtest::subtest`, so the runs kept
//! for it stand apart from the whole test's; `whole` gives the test back
//! for evaluating it, and `subtests_of` the subtests for building them.

use std::collections::BTreeSet;

/// What joins a test to its subtests in a narrowed check's name. No
/// check's name holds it.
const JOINED: &str = "::";

/// The longest a narrowed check's name may be, since it names its logs:
/// the commit and the run beside it stay under a file name's 255 bytes.
const LONGEST: usize = 200;

/// The test a check, narrowed or not, runs subtests of.
#[must_use]
pub fn whole(check: &str) -> &str {
    check.split(JOINED).next().unwrap_or(check)
}

/// The subtests a narrowed check runs, none for a whole test.
#[must_use]
pub fn subtests_of(check: &str) -> Vec<&str> {
    check.split(JOINED).skip(1).collect()
}

/// `test` narrowed to the subtests that failed in every one of `runs`, what
/// `failed_in` read from the head's failed runs of it, or `test` itself
/// when no subtest failed in all of them or the name would be too long. A
/// subtest failing in only some of them is as likely flaky as broken, and
/// a search on it would blame whichever merge its flake fell on.
#[must_use]
pub fn narrowed(test: &str, runs: &[BTreeSet<String>]) -> String {
    let mut each = runs.iter().cloned();
    let Some(first) = each.next() else {
        return test.to_owned();
    };
    let failing = each.fold(first, |kept, next| &kept & &next);
    if failing.is_empty() {
        return test.to_owned();
    }
    let name = std::iter::once(test)
        .chain(failing.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(JOINED);
    if name.len() > LONGEST {
        test.to_owned()
    } else {
        name
    }
}

/// The subtests pytest says failed in a run's log, read a line at a time,
/// each by its function's name, a parametrised one's parameters dropped,
/// since run-subtests picks by name.
#[must_use]
pub fn failed_in(log: impl IntoIterator<Item = impl AsRef<str>>) -> BTreeSet<String> {
    log.into_iter()
        .filter_map(|line| failed_on(line.as_ref()).map(str::to_owned))
        .collect()
}

/// The subtest one line of a log says failed, if it says one did. One the
/// driver failed for losing the machine, which breaks the pipe to its shell,
/// it never ran: on 16 November three runs that lost it named four subtests
/// still to come, and #4953 was blamed for breaking three that never failed
/// otherwise. One that erred in a fixture, as pytest's `ERROR` says, failed
/// as surely: on 18 November both runs of vm-the-connection-stays-quick
/// at 7030d16c46ae erred so in two subtests each, named none, and could
/// be neither excused by its list nor searched by its subtests.
/// changed-checks' quarantine.rs, nix-offload's changed/pytest.rs and
/// run-subtests.py read a line alike, so the four change together.
fn failed_on(line: &str) -> Option<&str> {
    if line.contains(LOST) {
        return None;
    }
    let said = pytests_own(line);
    let rest = said
        .strip_prefix("FAILED ")
        .or_else(|| said.strip_prefix("ERROR "))?;
    let (_, name) = rest.split_whitespace().next()?.rsplit_once(JOINED)?;
    let name = name.split('[').next().unwrap_or(name);
    (name.starts_with("test_") && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .then_some(name)
}

/// What pytest wrote on one line of a build's log, past what Nix puts
/// before it: `vm-test-run-x> ` on a line, or spaces then `> ` on those
/// that follow it. What a guest echoes stays behind the driver's
/// `machine # `, so a `FAILED` it prints names no subtest.
fn pytests_own(line: &str) -> &str {
    if let Some(rest) = line.trim_start().strip_prefix("> ") {
        return rest;
    }
    match line.split_once("> ") {
        Some((build, rest)) if !build.is_empty() && !build.contains(' ') => rest,
        _ => line,
    }
}

/// Whether a narrowed run's log says pytest selected none of its subtests:
/// a commit from before they were written, which can't have broken them,
/// though `vm_pytest` fails the run all the same.
#[must_use]
pub fn none_selected(log: impl IntoIterator<Item = impl AsRef<str>>) -> bool {
    log.into_iter()
        .any(|line| line.as_ref().contains(" / 0 selected"))
}

/// Whether a run's log ends in pytest's summary, so every subtest it
/// selected ran, and those it names no failure of passed: a run killed
/// at its time limit never says which of the rest would have failed.
/// Only pytest's own line sums it up, as for a failure, so a guest's
/// echoed summary finishes nothing.
#[must_use]
pub fn finished(log: impl IntoIterator<Item = impl AsRef<str>>) -> bool {
    log.into_iter().any(|line| {
        let line = pytests_own(line.as_ref()).trim_end();
        line.starts_with('=')
            && (line.contains(" passed") || line.contains(" failed") || line.contains(" error"))
            && line.contains(" in ")
            && line.ends_with("==")
    })
}

/// Whether a narrowed run's log says its subtests can't be built alone,
/// so the whole test runs there instead: a commit from before 29 October
/// has no `subtests`, and a check on a nested niri isn't a VM test, which
/// run-subtests refuses. On 14 November that refusal counted as the
/// chords failing again, and the net held main 18 minutes on a flake.
#[must_use]
pub fn unbuildable(log: impl IntoIterator<Item = impl AsRef<str>>) -> bool {
    let (mut missing, mut subtests) = (false, false);
    for line in log {
        let line = line.as_ref();
        if line.contains(NOT_A_VM_TEST) {
            return true;
        }
        missing |= line.contains("does not provide attribute");
        subtests |= line.contains("subtests");
    }
    missing && subtests
}

/// What pytest says of a subtest pending when the driver lost the machine.
const LOST: &str = " - BrokenPipeError";

/// What run-subtests' `subtests` throws for a check that isn't a VM test.
const NOT_A_VM_TEST: &str = "isn't a VM test, so it has no subtests";

#[cfg(test)]
mod tests {
    use super::*;

    /// What a search reads from a run whose log says `failed` failed.
    fn run(failed: &[&str]) -> BTreeSet<String> {
        failed_in(log(failed).lines())
    }

    fn log(failed: &[&str]) -> String {
        let lines = failed.iter().map(|name| {
            format!(
                "vm-test-run-desktop-looks> FAILED vm-pytest-ab/desktop-looks-vm-test.py::{name} \
                 - AssertionError: the shell never reported 'showing ' as wanted\n       \
                 > FAILED vm-pytest-ab/desktop-looks-vm-test.py::{name} - AssertionError\n"
            )
        });
        std::iter::once("vm-test-run-desktop-looks> collected 20 items\n".to_owned())
            .chain(lines)
            .collect()
    }

    #[test]
    fn a_search_runs_only_the_subtests_that_failed_in_every_run_at_its_head() {
        let logs = [
            run(&["test_the_budgets", "test_an_unplugged_ergodox"]),
            run(&["test_the_home_network", "test_the_budgets"]),
        ];
        let check = narrowed("vm-desktop-looks", &logs);
        assert_eq!(
            check, "vm-desktop-looks::test_the_budgets",
            "a subtest failing in one run only is left out as likely flaky"
        );
        assert_eq!(
            whole(&check),
            "vm-desktop-looks",
            "the test is given back whole"
        );
        assert_eq!(
            subtests_of(&check),
            ["test_the_budgets"],
            "and its subtests apart"
        );
    }

    #[test]
    fn a_test_whose_failures_name_no_subtest_in_common_is_searched_whole() {
        let apart = [run(&["test_a"]), run(&["test_b"])];
        assert_eq!(
            narrowed("vm-x", &apart),
            "vm-x",
            "no subtest failed in both"
        );
        let unnamed = vec![failed_in("machine: must succeed: false\n".lines()); 2];
        assert_eq!(
            narrowed("vm-x", &unnamed),
            "vm-x",
            "a script not run by pytest"
        );
        assert_eq!(narrowed("vm-x", &[]), "vm-x", "no run to read");
        let long = format!("test_{}", "a".repeat(LONGEST));
        let too_long = [run(&[&long]), run(&[&long])];
        assert_eq!(
            narrowed("vm-x", &too_long),
            "vm-x",
            "a name too long for its logs"
        );
        assert!(
            subtests_of("vm-x").is_empty(),
            "a whole test has no subtests"
        );
    }

    #[test]
    fn a_parametrised_subtest_is_named_by_its_function() {
        let logs = [run(&["test_at[1.5]"]), run(&["test_at[2]"])];
        assert_eq!(
            narrowed("vm-x", &logs),
            "vm-x::test_at",
            "its parameters are dropped"
        );
    }

    #[test]
    fn a_subtest_failed_only_by_losing_the_machine_is_not_named() {
        let lost = "vm-test-run-x> FAILED vm-pytest-ab/x-vm-test.py::test_the_vms - \
                    BrokenPipeError: [Errno 32] Broken pipe\n\
                    vm-test-run-x> FAILED vm-pytest-ab/x-vm-test.py::test_shortcuts - \
                    AssertionError: the launcher never took the keys\n";
        assert_eq!(
            failed_in(lost.lines()),
            BTreeSet::from(["test_shortcuts".to_owned()]),
            "the driver losing the machine says nothing of the subtest it was in"
        );
        let erred = "vm-test-run-x> ERROR vm-pytest-ab/x-vm-test.py::test_the_vms - \
                     BrokenPipeError: [Errno 32] Broken pipe\n";
        assert!(
            failed_in(erred.lines()).is_empty(),
            "losing the machine in a fixture says nothing of its subtest either"
        );
    }

    #[test]
    fn a_subtest_pytest_says_erred_in_its_fixture_is_named() {
        let erred = "vm-test-run-shaper-rate> ERROR vm-pytest-1w/shaper-rate-vm-test.py::\
                     test_climbs_back - test_driver.errors.RequestedAssertionFailed: action \
                     timed out after 91.01 s\n       \
                     > ERROR vm-pytest-1w/shaper-rate-vm-test.py::test_holds[2] - Timeout\n\
                     vm-test-run-shaper-rate> ____ ERROR at setup of test_settles ____\n";
        assert_eq!(
            failed_in(erred.lines()),
            BTreeSet::from(["test_climbs_back".to_owned(), "test_holds".to_owned()]),
            "an error in a fixture fails its subtest as a failed assertion does"
        );
    }

    #[test]
    fn a_guests_own_output_naming_a_failure_names_no_subtest() {
        let guest = "vm-test-run-x> machine # [  12.1] FAILED x-vm-test.py::test_z - echoed\n\
                     vm-test-run-x> machine # ERROR x-vm-test.py::test_w - echoed\n";
        assert!(
            failed_in(guest.lines()).is_empty(),
            "only pytest's own summary line, at the line's start, says a subtest failed"
        );
    }

    #[test]
    fn a_run_that_selected_no_subtest_and_a_commit_without_subtests_are_told() {
        let none = "collected 18 items / 18 deselected / 0 selected\n";
        assert!(none_selected(none.lines()), "pytest selected nothing");
        assert!(
            !none_selected("collected 20 items / 19 deselected / 1 selected\n".lines()),
            "one ran"
        );
        let old = "error: flake 'github:o/r/c' does not provide attribute \
                   'legacyPackages.x86_64-linux.subtests'";
        assert!(unbuildable(old.lines()), "the commit has no subtests");
        let nested = "       error: run-subtests: the-launcher-opens-on-what-it-kept-on-niri isn't \
                      a VM test, so it has no subtests; build it whole";
        assert!(
            unbuildable(nested.lines()),
            "a check on a nested niri has no subtests to build alone"
        );
        assert!(
            !unbuildable("error: builder failed".lines()),
            "a failure of the run itself"
        );
    }

    #[test]
    fn a_run_is_finished_once_pytest_sums_it_up() {
        let summed = "vm-test-run-x> ===== 1 failed, 19 passed in 752.38s (0:12:32) =====";
        assert!(finished(summed.lines()), "pytest summed the run up");
        let erred = "vm-test-run-x> ======== 6 errors in 120.95s (0:02:00) ========";
        assert!(finished(erred.lines()), "a run whose every subtest erred");
        let all = "vm-test-run-x> ==== 1 failed, 2 passed, 3 errors in 9.10s ====";
        assert!(finished(all.lines()), "a run that did each");
        let one = "       > ==== 1 error in 3.20s ====";
        assert!(
            finished(one.lines()),
            "a run with one error, on a line that follows"
        );
        let echoed = "vm-test-run-x> machine # ==== 9 passed in 1.00s ====";
        assert!(!finished(echoed.lines()), "a guest's echoed summary");
        let killed = "vm-test-run-x> [3601.91s] !!! Test \"A\" failed with error: \"[Errno 9]\"\n\
                      error: Cannot build '/nix/store/x.drv'.\n";
        assert!(!finished(killed.lines()), "a run killed at its limit");
    }
}
