//! What `excuses` asks of the lent builders for a red, what it keeps and
//! writes of one it excuses, whose quarantine it believes, and how the
//! train's log says whose a red is.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::Value;

use super::super::tests::git_in as git;
use super::super::{checkout_in, excused, folder};
use super::checks_said_tests::excused_by_quarantine;
use super::tests::words;
use super::{Blame, Flakes, Lent, Red, built_log, excuses, said, tell};
use crate::ready::built_there;
use crate::state::State;
use crate::world::QUARANTINE;

/// Lent builders that keep each build asked of them, as (flake, checks,
/// commit), and each flake the checks' derivation names are asked at,
/// passing every build and naming derivations as `names` says. What a
/// build is at is its flake; the commit only labels its log line.
#[derive(Default)]
struct Asked {
    builds: Vec<(String, Vec<String>, String)>,
    named_at: RefCell<Vec<String>>,
    names: BTreeMap<String, String>,
}

impl Lent for Asked {
    fn build(
        &mut self,
        _: &Path,
        flake: &str,
        _: &Path,
        checks: &[&str],
        commit: &str,
    ) -> (Option<i32>, String) {
        let checks = checks.iter().map(|check| (*check).to_owned()).collect();
        self.builds
            .push((flake.to_owned(), checks, commit.to_owned()));
        (Some(0), String::new())
    }

    fn named(&self, _: &Path, flake: &str, _: &[String]) -> BTreeMap<String, String> {
        self.named_at.borrow_mut().push(flake.to_owned());
        self.names.clone()
    }
}

/// Which flake each build and naming is at is the whole of the scheme's
/// safety: a rider's red rebuilt at the base would pass, and land as a
/// flake. The failed derivation here is named unlike its check, so which
/// check it builds is asked, at the candidate, whose build named it. The
/// red excused is kept, so a recut of it is judged as a pass's would be.
#[test]
fn a_flake_is_built_at_the_base_then_at_the_candidate_and_its_finding_names_the_red() {
    let root = std::env::temp_dir().join(format!("merge-net-excuses-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let (base, candidate) = ("b".repeat(40), "c".repeat(40));
    let checkout = checkout_in(&root);
    fs::create_dir_all(&checkout).unwrap();
    let log = folder(&root).join(format!("{candidate}.log"));
    State::at(root.clone()).make().unwrap();
    fs::write(&log, words(&["a", "other"], &["builds-a"])).unwrap();
    let red = Red {
        checkout: &checkout,
        base: &base,
        candidate: &candidate,
        log: &log,
    };
    let mut asked = Asked {
        names: BTreeMap::from([("builds-a".to_owned(), "a".to_owned())]),
        ..Asked::default()
    };
    let landed = excuses(
        &root,
        "https://github.com/someone/core.git",
        &red,
        &mut Flakes::default(),
        &mut asked,
    );
    assert!(
        landed,
        "a flake, passing at the base and again, split the train"
    );
    let at = |flake: &str, checks: &[&str], commit: &str| {
        (
            flake.to_owned(),
            checks
                .iter()
                .map(|check| (*check).to_owned())
                .collect::<Vec<_>>(),
            commit.to_owned(),
        )
    };
    assert_eq!(
        asked.builds,
        [
            at(&format!("github:someone/core/{base}"), &["a"], &base),
            at(".", &["a"], &candidate),
            at(".", &["a", "other"], &candidate),
        ],
        "a build wasn't of the flake its judging needs"
    );
    assert_eq!(
        asked.named_at.take(),
        ["."],
        "which check a failed derivation builds wasn't asked of the candidate's flake alone"
    );
    let findings: Vec<Value> = fs::read_dir(root.join("findings"))
        .unwrap()
        .map(|entry| serde_json::from_slice(&fs::read(entry.unwrap().path()).unwrap()).unwrap())
        .collect();
    assert_eq!(findings.len(), 1, "a flake's finding wasn't written once");
    assert_eq!(findings[0]["kind"], "train-flaky");
    assert_eq!(
        findings[0]["log"],
        log.display().to_string(),
        "a flake's finding didn't name the gate's log, which holds its red"
    );
    assert_eq!(
        findings[0]["rebuilt_log"],
        built_log(&root, &candidate, "a", "again")
            .display()
            .to_string(),
        "a flake's finding didn't name the rebuild that passed"
    );
    let kept = excused::record(&checkout, &candidate);
    assert!(
        built_there(&kept, &|at| at == base),
        "the excused red wasn't kept beside the checkout as a run that built on its base: {kept}"
    );
    fs::remove_dir_all(&root).unwrap();
}

/// Only a check the train found the riders broke, or one they added, is
/// said to be theirs outright: the rest the train only takes as theirs,
/// so its log says no more than it found.
#[test]
fn a_red_the_train_couldnt_pin_on_the_riders_is_said_taken_as_theirs() {
    let taken = [
        Blame::Masked,
        Blame::RuleMoved,
        Blame::Untaken,
        Blame::Spent,
        Blame::Unbuilt,
    ];
    for blamed in taken {
        let words = said(blamed);
        assert!(
            words.contains("taken as its riders'"),
            "{blamed:?} was said to be the riders' outright: {words}"
        );
    }
    let moved = said(Blame::RuleMoved);
    assert!(
        moved.contains("git couldn't list what they moved"),
        "rules moved was said as found, though git may only have failed to list the files: {moved}"
    );
}

/// A ride that told a check's finding at a base already doesn't look for
/// its owner again: each merge looked at costs evaluations inside the ride,
/// and the finding it would write is the one kept.
#[test]
fn a_check_told_at_its_base_already_isnt_looked_for_an_owner_again() {
    let root = std::env::temp_dir().join(format!("merge-net-told-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    State::at(root.clone()).make().unwrap();
    let (base, candidate) = ("b".repeat(40), "c".repeat(40));
    let log = folder(&root).join(format!("{candidate}.log"));
    let red = Red {
        checkout: &root,
        base: &base,
        candidate: &candidate,
        log: &log,
    };
    let looked = RefCell::new(Vec::new());
    let owner = |check: &str, _: &[String]| {
        looked.borrow_mut().push(check.to_owned());
        Some(12)
    };
    tell(&root, &red, &[("a".to_owned(), Blame::Flaky)], &owner);
    assert_eq!(looked.take(), ["a"], "a flake's owner wasn't looked for");
    let judged = [
        ("a".to_owned(), Blame::Flaky),
        ("b".to_owned(), Blame::Main),
    ];
    tell(&root, &red, &judged, &owner);
    assert_eq!(
        looked.take(),
        ["b"],
        "an owner was looked for again of a check told at its base already"
    );
    let owners: Vec<Value> = fs::read_dir(root.join("findings"))
        .unwrap()
        .map(|entry| serde_json::from_slice(&fs::read(entry.unwrap().path()).unwrap()).unwrap())
        .map(|finding: Value| finding["owner_pr"].clone())
        .collect();
    assert_eq!(
        owners,
        [12, 12],
        "a finding didn't name the owner looked for"
    );
    fs::remove_dir_all(&root).unwrap();
}

/// Main's quarantine is believed at the base alone: a rider's commit can
/// write any list, and its evaluation can print changed-checks' line for
/// any check, so a check the candidate's list holds and the base's doesn't
/// is judged all the same, built at the base as any is.
#[test]
fn a_check_the_candidates_quarantine_alone_holds_is_judged() {
    let root = std::env::temp_dir().join(format!("merge-net-held-ours-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    State::at(root.clone()).make().unwrap();
    let checkout = checkout_in(&root);
    fs::create_dir_all(checkout.join("tools/merge-net")).unwrap();
    git(&checkout, &["init", "-q", "-b", "main"]).unwrap();
    let commit = |list: &str, subject: &str| {
        fs::write(checkout.join(QUARANTINE), list).unwrap();
        git(&checkout, &["add", QUARANTINE]).unwrap();
        git(&checkout, &["commit", "-q", "-m", subject]).unwrap();
        git(&checkout, &["rev-parse", "HEAD"]).unwrap()
    };
    let base = commit("[]", "main holds nothing");
    let candidate = commit(r#"[{"test": "a"}]"#, "a rider holds a");
    let log = folder(&root).join(format!("{candidate}.log"));
    fs::write(&log, words(&["a"], &["a"]) + &excused_by_quarantine("a")).unwrap();
    let red = Red {
        checkout: &checkout,
        base: &base,
        candidate: &candidate,
        log: &log,
    };
    let mut asked = Asked::default();
    let landed = excuses(
        &root,
        "https://github.com/someone/core.git",
        &red,
        &mut Flakes::default(),
        &mut asked,
    );
    assert!(
        landed,
        "a flake of a check the candidate's quarantine alone holds wasn't judged a flake"
    );
    assert_eq!(
        asked.builds.first(),
        Some(&(
            format!("github:someone/core/{base}"),
            vec!["a".to_owned()],
            base.clone()
        )),
        "a check the candidate's quarantine alone holds was left out of the red"
    );
    fs::remove_dir_all(&root).unwrap();
}
