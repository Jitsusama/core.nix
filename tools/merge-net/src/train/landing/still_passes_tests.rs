//! Whether the record the train keeps of an excused red lets its
//! candidate, cut again on a main that moved, land as a passed one's
//! would (`still_passes`).
//!
//! `still_passes` moves the whole process into the train's checkout while
//! it asks git, as the train does nothing else meanwhile. Tests run beside
//! one another in one process, and some read the process's folder, so the
//! judging runs alone, in a run of this test binary of its own.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::super::flaky::{Blame, Red};
use super::super::tests::{based, change, git_in, merging, root};
use super::super::{checkout_in, cut, excused};
use super::still_passes;

/// Run `test`, one of this file's tests marked ignored, alone in a run of
/// this test binary of its own, and fail unless it passed.
fn alone(test: &str) {
    let path = module_path!().split_once("::").map_or("", |(_, path)| path);
    let alone = format!("{path}::{test}");
    // An empty state of the account's own, so no record nix-offload kept
    // there is read beside the train's.
    let state = root(&format!("still-passes-state-{test}"));
    fs::create_dir_all(&state).unwrap();
    let ran = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "--ignored", "--test-threads=1", &alone])
        .env("XDG_STATE_HOME", &state)
        .output()
        .unwrap();
    fs::remove_dir_all(&state).unwrap();
    let said = String::from_utf8_lossy(&ran.stdout);
    let errors = String::from_utf8_lossy(&ran.stderr);
    assert!(
        ran.status.success() && said.contains("1 passed"),
        "{test} run alone didn't pass once: {said}{errors}"
    );
}

/// An excused red's record speaks for its candidate cut again on a main
/// that moved none of its files, as a pass's would; no record, or one of
/// another candidate, doesn't.
#[test]
fn an_excused_reds_record_speaks_for_its_recut_as_a_pass() {
    alone("an_excused_reds_record_judged_alone");
}

/// The record is found from a checkout named relative to the process's
/// folder, as a relative `MERGE_NET_STATE` names it, though the judging
/// moves the process into the checkout.
#[test]
fn a_relative_checkout_still_finds_its_record() {
    alone("a_relative_checkout_judged_alone");
}

/// A record that exists but can't be read speaks for no recut, so it's
/// built again, as one with no record is.
#[test]
fn an_unreadable_record_speaks_for_no_recut() {
    alone("an_unreadable_record_judged_alone");
}

/// A candidate and another cut on `base` in the train's checkout under
/// `place`, and the candidate's riders cut again on a main that moved
/// none of their files.
struct Cuts {
    checkout: PathBuf,
    base: String,
    candidate: String,
    another: String,
    main: String,
    recut: String,
}

fn cuts_under(place: &Path) -> Cuts {
    let checkout = checkout_in(place);
    let git = |arguments: &[&str]| git_in(&checkout, arguments);
    let base = based(&checkout);
    let riding = [merging(&change(&checkout, &base, "one", "one\n"), 1)];
    let other = [merging(&change(&checkout, &base, "two", "two\n"), 2)];
    cut(&git, &base, &riding).unwrap();
    let candidate = git(&["rev-parse", "HEAD"]).unwrap();
    cut(&git, &base, &other).unwrap();
    let another = git(&["rev-parse", "HEAD"]).unwrap();
    let main = change(&checkout, &base, "elsewhere", "elsewhere\n");
    cut(&git, &main, &riding).unwrap();
    let recut = git(&["rev-parse", "HEAD"]).unwrap();
    Cuts {
        checkout,
        base,
        candidate,
        another,
        main,
        recut,
    }
}

/// Keep, as the train does once it excuses one, the record of the red
/// `which` candidate of `cuts` under `place`.
fn excuse(place: &Path, cuts: &Cuts, which: &str) {
    let log = place.join("gate.log");
    let red = Red {
        checkout: &cuts.checkout,
        base: &cuts.base,
        candidate: which,
        log: &log,
    };
    excused::keep(place, &red, &[("vm-pi".to_owned(), Blame::Flaky)]);
}

/// The judging itself, which the first test above runs alone.
#[test]
#[ignore = "it moves the process's folder, so a test above runs it alone"]
fn an_excused_reds_record_judged_alone() {
    let place = root("still-passes");
    let cuts = cuts_under(&place);
    assert!(
        !still_passes(&cuts.checkout, &cuts.recut, &cuts.main),
        "a recut whose candidate's red has no record was taken as passed"
    );
    excuse(&place, &cuts, &cuts.another);
    assert!(
        !still_passes(&cuts.checkout, &cuts.recut, &cuts.main),
        "the record of another candidate's excused red spoke for this one's recut"
    );
    excuse(&place, &cuts, &cuts.candidate);
    assert!(
        still_passes(&cuts.checkout, &cuts.recut, &cuts.main),
        "an excused red's record didn't speak for its candidate cut again on a main that \
         moved none of its files"
    );
    fs::remove_dir_all(&place).unwrap();
}

/// The judging from a relative checkout, which the second test above runs
/// alone.
#[test]
#[ignore = "it moves the process's folder, so a test above runs it alone"]
fn a_relative_checkout_judged_alone() {
    let place = root("still-passes-relative");
    let cuts = cuts_under(&place);
    excuse(&place, &cuts, &cuts.candidate);
    let here = std::env::current_dir().unwrap();
    std::env::set_current_dir(&place).unwrap();
    let relative = checkout_in(Path::new(""));
    assert!(
        relative.is_relative() && relative.join(".git").exists(),
        "{} isn't the checkout named from the state folder",
        relative.display()
    );
    let passes = still_passes(&relative, &cuts.recut, &cuts.main);
    std::env::set_current_dir(here).unwrap();
    assert!(
        passes,
        "the record kept beside a checkout named relative to the process's folder wasn't \
         found once the judging moved into it"
    );
    fs::remove_dir_all(&place).unwrap();
}

/// The judging of a record that can't be read, which the third test above
/// runs alone.
#[test]
#[ignore = "it moves the process's folder, so a test above runs it alone"]
fn an_unreadable_record_judged_alone() {
    let place = root("still-passes-unreadable");
    let cuts = cuts_under(&place);
    excuse(&place, &cuts, &cuts.candidate);
    assert!(
        still_passes(&cuts.checkout, &cuts.recut, &cuts.main),
        "the candidate's readable record didn't speak for its recut, so this tests nothing"
    );
    // A folder where the record was is a record that exists but can't be
    // read as text.
    let record = excused::beside(&cuts.checkout).join(&cuts.candidate);
    fs::remove_file(&record).unwrap();
    fs::create_dir(&record).unwrap();
    assert!(
        !still_passes(&cuts.checkout, &cuts.recut, &cuts.main),
        "a record that couldn't be read spoke for the recut, so it wasn't built again"
    );
    fs::remove_dir_all(&place).unwrap();
}
