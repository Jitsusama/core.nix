//! Whether the record the train keeps of an excused red lets its
//! candidate, cut again on a main that moved, land as a passed one's
//! would (`still_passes`).
//!
//! `still_passes` moves the whole process into the train's checkout while
//! it asks git, as the train does nothing else meanwhile. Tests run beside
//! one another in one process, and some read the process's folder, so the
//! judging runs alone, in a run of this test binary of its own.

use std::fs;
use std::process::Command;

use super::super::flaky::{Blame, Red};
use super::super::tests::{based, change, git_in, merging, root};
use super::super::{cut, excused, folder};
use super::still_passes;

/// An excused red's record speaks for its candidate cut again on a main
/// that moved none of its files, as a pass's would; no record, or one of
/// another candidate, doesn't.
#[test]
fn an_excused_reds_record_speaks_for_its_recut_as_a_pass() {
    let path = module_path!().split_once("::").map_or("", |(_, path)| path);
    let alone = format!("{path}::an_excused_reds_record_judged_alone");
    // An empty state of the account's own, so no record nix-offload kept
    // there is read beside the train's.
    let state = root("still-passes-state");
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
        "the judging run alone didn't pass once: {said}{errors}"
    );
}

/// The judging itself, which the test above runs alone.
#[test]
#[ignore = "it moves the process's folder, so the test above runs it alone"]
fn an_excused_reds_record_judged_alone() {
    let place = root("still-passes");
    let checkout = folder(&place).join("checkout");
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
    assert!(
        !still_passes(&checkout, &recut, &main),
        "a recut whose candidate's red has no record was taken as passed"
    );
    let log = place.join("gate.log");
    let judged = [("vm-pi".to_owned(), Blame::Flaky)];
    let excuse = |which: &str| {
        let red = Red {
            checkout: &checkout,
            base: &base,
            candidate: which,
            log: &log,
        };
        excused::keep(&place, &red, &judged);
    };
    excuse(&another);
    assert!(
        !still_passes(&checkout, &recut, &main),
        "the record of another candidate's excused red spoke for this one's recut"
    );
    excuse(&candidate);
    assert!(
        still_passes(&checkout, &recut, &main),
        "an excused red's record didn't speak for its candidate cut again on a main that \
         moved none of its files"
    );
    fs::remove_dir_all(&place).unwrap();
}
