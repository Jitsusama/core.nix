//! What `excuses` asks of the lent builders for a red, what it keeps and
//! writes of one it excuses, and how the train's log says whose a red is.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::Value;

use super::super::{excused, folder};
use super::tests::words;
use super::{Blame, Flakes, Lent, Red, built_log, excuses, said};
use crate::ready::built_there;
use crate::state::State;

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
    let checkout = folder(&root).join("checkout");
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
