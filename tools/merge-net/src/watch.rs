//! The net's round: for each new head of main, run every VM test the
//! merges since the last head it passed changed, and revert the merge that
//! broke one.
//!
//! A test fails for real when it fails twice at the same commit, which is
//! the same input. One that fails and then passes is flaky: it goes on the
//! quarantine list with the merge's PR as its owner, and nothing is
//! reverted for it, nor for a quarantined test that fails. A run the
//! builder never took (`nix-offload` exits 75) is tried again later, and is
//! no failure. A test that passed at a commit never runs there again.
//!
//! The round talks to git, nix and GitHub through `World`, so each of
//! those choices is tested against a stand-in (the tests below).

use std::path::Path;
use std::sync::Mutex;

use serde_json::{Value, json};

use crate::state::{Ran, State, now, short};

/// What `nix-offload` exits with when no builder took the build.
pub const UNSENT: i32 = 75;

/// How many times a run the builder never took is sent again, two minutes
/// apart, before the round leaves it for the next.
const SENDS: u32 = 15;

/// How many VM tests run at once on the VM builder.
pub const AT_ONCE: usize = 2;

/// What changed-checks named between two heads.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Changed {
    /// The checks added or changed, VM tests and the rest.
    pub moved: Vec<String>,
    /// The checks that no longer evaluate.
    pub broken: Vec<String>,
}

impl Changed {
    /// Read changed-checks' lines, `kind name` each.
    pub fn parse(said: &str) -> Self {
        let mut changed = Self::default();
        for line in said.lines() {
            let mut words = line.split_whitespace();
            match (words.next(), words.next()) {
                (Some("added" | "changed"), Some(name)) => changed.moved.push(name.to_owned()),
                (Some("broken"), Some(name)) => changed.broken.push(name.to_owned()),
                _ => {}
            }
        }
        changed
    }

    /// The VM tests among those that moved.
    pub fn vm_tests(&self) -> Vec<String> {
        self.moved
            .iter()
            .filter(|name| name.starts_with("vm-"))
            .cloned()
            .collect()
    }
}

/// A merge that broke a test, and what the revert needs to say of it.
#[derive(Debug, Clone)]
pub struct Bad {
    pub commit: String,
    pub subject: String,
    pub pr: Option<u64>,
    pub test: String,
    pub log: String,
}

/// A change the net made and merged on main.
#[derive(Debug, Clone)]
pub struct Merged {
    pub pr: u64,
    pub commit: Option<String>,
}

/// What the round asks of git, nix and GitHub.
pub trait World: Sync {
    /// Why nothing may start now, if something says so: a pause, or a
    /// hold on the whole machine.
    fn held(&self) -> Option<String>;
    /// Fetch main and give its head.
    fn head(&self) -> Result<String, String>;
    /// The first-parent commits after `base` up to `head`, oldest first.
    fn merges(&self, base: &str, head: &str) -> Result<Vec<String>, String>;
    /// A commit's subject line.
    fn subject(&self, commit: &str) -> String;
    /// What changed-checks names between `base` and `head`.
    fn changed(&self, base: &str, head: &str) -> Result<Changed, String>;
    /// Whether `commit` has the check at all.
    fn has_check(&self, commit: &str, check: &str) -> Result<bool, String>;
    /// Build a check at a commit, its log written to `log`, and give its
    /// exit, none when it was stopped.
    fn build(&self, commit: &str, check: &str, log: &Path) -> Option<i32>;
    /// The tests quarantined at `head`.
    fn quarantined(&self, head: &str) -> Vec<String>;
    /// Revert a bad merge on main through a PR merged on its own checks.
    fn revert(&self, bad: &Bad) -> Result<Merged, String>;
    /// Put a flaky test on the quarantine list through a merged PR.
    fn quarantine(
        &self,
        test: &str,
        owner: Option<u64>,
        commit: &str,
        reason: &str,
    ) -> Result<Merged, String>;
    /// Whether main holds `commit`.
    fn on_main(&self, commit: &str) -> bool;
    /// Wait this many seconds.
    fn wait(&self, seconds: u64);
}

/// How a test ended at a commit, over every run of it there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Passed,
    /// Failed, then passed, at the same commit.
    Flaky,
    /// Failed twice at the same commit.
    Failed,
    /// The builder never took it.
    Unsent,
}

/// What a round came to.
#[derive(Debug, PartialEq, Eq)]
pub enum Round {
    /// Nothing new to look at, or held.
    Idle,
    /// Every test the head changed passed, so it's the next base.
    Passed(String),
    /// A test failed for real at the head.
    Failed(String),
    /// Some test wasn't taken, so the head is looked at again.
    Unfinished(String),
}

/// Run `check` at `commit` until it has passed or failed twice there,
/// counting the runs it had before.
pub fn attempt(world: &dyn World, state: &State, commit: &str, check: &str) -> Outcome {
    let before = state.runs(commit, check);
    let mut failures = before.iter().filter(|ran| **ran == Ran::Failed).count();
    if before.contains(&Ran::Passed) {
        return if failures > 0 {
            Outcome::Flaky
        } else {
            Outcome::Passed
        };
    }
    while failures < 2 {
        // Every run before was a failure, so the next is one past them.
        let log = state.log(commit, check, u32::try_from(failures + 1).unwrap_or(0));
        let mut sends = 0;
        let code = loop {
            let code = world.build(commit, check, &log);
            if code != Some(UNSENT) {
                break code;
            }
            sends += 1;
            if sends >= SENDS {
                state.say(&format!("no builder took {check} at {}", short(commit)));
                return Outcome::Unsent;
            }
            world.wait(120);
        };
        let ran = if code == Some(0) {
            Ran::Passed
        } else {
            Ran::Failed
        };
        if let Err(why) = state.record(commit, check, ran, code, &log) {
            state.say(&why);
        }
        if ran == Ran::Passed {
            return if failures > 0 {
                Outcome::Flaky
            } else {
                Outcome::Passed
            };
        }
        failures += 1;
        state.say(&format!(
            "{check} failed at {} ({failures} of 2)",
            short(commit)
        ));
    }
    Outcome::Failed
}

/// Run each check at `commit`, `AT_ONCE` at a time, and give each outcome
/// in the order asked.
pub fn attempt_all(
    world: &dyn World,
    state: &State,
    commit: &str,
    checks: &[String],
) -> Vec<Outcome> {
    let next = Mutex::new(0_usize);
    let outcomes = Mutex::new(vec![Outcome::Unsent; checks.len()]);
    std::thread::scope(|scope| {
        for _ in 0..AT_ONCE.min(checks.len()) {
            scope.spawn(|| {
                loop {
                    let index = {
                        let mut next = next
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        let index = *next;
                        *next += 1;
                        index
                    };
                    let Some(check) = checks.get(index) else {
                        break;
                    };
                    if world.held().is_some() {
                        continue;
                    }
                    let outcome = attempt(world, state, commit, check);
                    outcomes
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)[index] = outcome;
                }
            });
        }
    });
    outcomes
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Find the merge among `merges` (oldest first, the last failing `check`)
/// that broke it, the commit before the first being where it passed. A
/// commit without the check counts as passing it. None when a run wasn't
/// taken, so it can't be told.
pub fn bisect(world: &dyn World, state: &State, merges: &[String], check: &str) -> Option<String> {
    // `good` is how many of the merges are known to pass, `bad` the index
    // of the first known to fail.
    let (mut good, mut bad) = (0_usize, merges.len().checked_sub(1)?);
    while good < bad {
        let middle = good + (bad - good) / 2;
        let commit = &merges[middle];
        let passes = match world.has_check(commit, check) {
            Ok(false) => true,
            Ok(true) => match attempt(world, state, commit, check) {
                Outcome::Passed | Outcome::Flaky => true,
                Outcome::Failed => false,
                Outcome::Unsent => return None,
            },
            Err(why) => {
                state.say(&format!(
                    "couldn't tell whether {} has {check}: {why}",
                    short(commit)
                ));
                return None;
            }
        };
        if passes {
            good = middle + 1;
        } else {
            bad = middle;
        }
    }
    merges.get(bad).cloned()
}

/// The PR a merge commit's subject names, as GitHub writes it.
pub fn pr_of(subject: &str) -> Option<u64> {
    let rest = subject.strip_prefix("Merge pull request #")?;
    rest.split_whitespace().next()?.parse().ok()
}

/// One round: look at main's head, if there's something new to look at.
/// `start` is the base of the first round, before any head has passed.
pub fn round(world: &dyn World, state: &State, start: &str) -> Result<Round, String> {
    if let Some(why) = world.held() {
        state.say(&format!("held, starting nothing: {why}"));
        return Ok(Round::Idle);
    }
    let head = world.head()?;
    let hold = state.hold();
    if let Some(revert) = hold
        .as_ref()
        .and_then(|hold| hold["revert_commit"].as_str())
        && world.on_main(revert)
    {
        state.clear_hold()?;
        state.say(&format!(
            "the revert {} is on main, so the hold is cleared",
            short(revert)
        ));
    }
    let hold = state.hold();
    let (passed, seen) = state.heads();
    let base = passed.clone().unwrap_or_else(|| start.to_owned());
    if seen.as_deref() == Some(head.as_str()) || base == head {
        return Ok(Round::Idle);
    }
    state.say(&format!("comparing {} with {}", short(&base), short(&head)));
    let changed = world.changed(&base, &head)?;
    if !changed.broken.is_empty() {
        let finding = json!({
            "kind": "broken",
            "test": changed.broken.join(" "),
            "commit": head,
            "since": base,
            "owner_pr": pr_of(&world.subject(&head)),
            "at": now(),
        });
        state.find(&finding)?;
    }
    // A test held for a revert not yet merged is left until it is; one held
    // with no revert, as when the revert wouldn't apply, runs again, since
    // a merge since may have fixed it.
    let pending = hold
        .as_ref()
        .filter(|hold| hold["revert_commit"].is_string())
        .and_then(|hold| hold["test"].as_str().map(str::to_owned));
    let tests: Vec<String> = changed
        .vm_tests()
        .into_iter()
        .filter(|test| Some(test) != pending.as_ref())
        .collect();
    let outcomes = attempt_all(world, state, &head, &tests);
    let quarantined = world.quarantined(&head);
    let owner = pr_of(&world.subject(&head));
    let mut failed = false;
    let mut unfinished = false;
    for (test, outcome) in tests.iter().zip(outcomes) {
        let held = quarantined.contains(test);
        match outcome {
            Outcome::Passed => {
                if hold
                    .as_ref()
                    .is_some_and(|hold| hold["test"] == test.as_str())
                {
                    state.clear_hold()?;
                    state.say(&format!(
                        "{test} passes again at {}, so the hold is cleared",
                        short(&head)
                    ));
                }
            }
            Outcome::Flaky if !held && !state.found("flaky", test, &head) => {
                let reason = format!("failed, then passed, at {}", short(&head));
                let made = world.quarantine(test, owner, &head, &reason);
                state.find(&json!({
                    "kind": "flaky",
                    "test": test,
                    "commit": head,
                    "owner_pr": owner,
                    "log": state.log(&head, test, 1).display().to_string(),
                    "quarantine_pr": made.as_ref().ok().map(|merged| merged.pr),
                    "quarantine_error": made.err(),
                    "at": now(),
                }))?;
            }
            Outcome::Flaky => {}
            Outcome::Failed if held => {
                state.find(&json!({
                    "kind": "failed-in-quarantine",
                    "test": test,
                    "commit": head,
                    "owner_pr": owner,
                    "log": state.log(&head, test, 2).display().to_string(),
                    "at": now(),
                }))?;
            }
            Outcome::Failed => {
                failed = true;
                if hold
                    .as_ref()
                    .is_some_and(|hold| hold["test"] == test.as_str())
                {
                    continue;
                }
                broke(world, state, &base, &head, test)?;
            }
            Outcome::Unsent => unfinished = true,
        }
    }
    if unfinished && !failed {
        return Ok(Round::Unfinished(head));
    }
    // A head that failed is the next base all the same: the test that
    // failed is held, and its revert, or a fix forward, changes it, so the
    // next head runs it again, while the tests that passed here aren't
    // run again at every head until it's fixed.
    if failed {
        state.set_heads(&head, &head)?;
        return Ok(Round::Failed(head));
    }
    state.set_heads(&head, &head)?;
    state.say(&format!("{} passed every VM test it changed", short(&head)));
    Ok(Round::Passed(head))
}

/// A test failed for real at `head`: find the merge since `base` that
/// broke it, hold switching on it, revert it, and tell its owner.
fn broke(
    world: &dyn World,
    state: &State,
    base: &str,
    head: &str,
    test: &str,
) -> Result<(), String> {
    let merges = world.merges(base, head)?;
    let bad = if merges.len() == 1 {
        merges.first().cloned()
    } else {
        state.say(&format!(
            "finding which of {} merges broke {test}",
            merges.len()
        ));
        bisect(world, state, &merges, test)
    };
    let failing = bad.clone().unwrap_or_else(|| head.to_owned());
    let log = state.log(&failing, test, 2).display().to_string();
    let Some(commit) = bad else {
        state.set_hold(&json!({
            "test": test, "bad": Value::Null, "since": base, "head": head, "at": now(),
        }))?;
        state.find(&json!({
            "kind": "failed", "test": test, "commit": head, "since": base,
            "owner_pr": Value::Null, "log": log, "revert_error": "couldn't tell which merge broke it",
            "at": now(),
        }))?;
        return Ok(());
    };
    let subject = world.subject(&commit);
    let found = Bad {
        commit: commit.clone(),
        pr: pr_of(&subject),
        subject,
        test: test.to_owned(),
        log: log.clone(),
    };
    let mut hold = json!({
        "test": test, "bad": commit, "bad_pr": found.pr, "revert_pr": Value::Null,
        "revert_commit": Value::Null, "at": now(),
    });
    state.set_hold(&hold)?;
    let reverted = world.revert(&found);
    if let Ok(merged) = &reverted {
        hold["revert_pr"] = json!(merged.pr);
        hold["revert_commit"] = json!(merged.commit);
        state.set_hold(&hold)?;
    }
    state.find(&json!({
        "kind": "failed", "test": test, "commit": commit, "owner_pr": found.pr, "log": log,
        "revert_pr": reverted.as_ref().ok().map(|merged| merged.pr),
        "revert_error": reverted.as_ref().err(),
        "at": now(),
    }))?;
    match reverted {
        Ok(merged) => state.say(&format!(
            "reverted {} for {test} in #{}",
            short(&commit),
            merged.pr
        )),
        Err(why) => state.say(&format!(
            "couldn't revert {} for {test}: {why}",
            short(&commit)
        )),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;

    /// A stand-in world: each (commit, check) gives the exits listed, in
    /// turn, the last again once they run out; checks named in `missing`
    /// don't exist at a commit.
    #[derive(Default)]
    struct Stand {
        head: String,
        merges: Vec<String>,
        changed: Changed,
        exits: Mutex<HashMap<(String, String), Vec<i32>>>,
        built: Mutex<Vec<(String, String)>>,
        missing: Vec<(String, String)>,
        quarantined: Vec<String>,
        reverted: Mutex<Vec<String>>,
        put_in_quarantine: Mutex<Vec<String>>,
        held: Option<String>,
    }

    impl Stand {
        fn exits(self, commit: &str, check: &str, exits: &[i32]) -> Self {
            self.exits
                .lock()
                .unwrap()
                .insert((commit.to_owned(), check.to_owned()), exits.to_vec());
            self
        }
        fn built(&self) -> Vec<(String, String)> {
            self.built.lock().unwrap().clone()
        }
    }

    impl World for Stand {
        fn held(&self) -> Option<String> {
            self.held.clone()
        }
        fn head(&self) -> Result<String, String> {
            Ok(self.head.clone())
        }
        fn merges(&self, _: &str, _: &str) -> Result<Vec<String>, String> {
            Ok(self.merges.clone())
        }
        fn subject(&self, commit: &str) -> String {
            format!("Merge pull request #{} from someone/branch", commit.len())
        }
        fn changed(&self, _: &str, _: &str) -> Result<Changed, String> {
            Ok(self.changed.clone())
        }
        fn has_check(&self, commit: &str, check: &str) -> Result<bool, String> {
            Ok(!self
                .missing
                .contains(&(commit.to_owned(), check.to_owned())))
        }
        fn build(&self, commit: &str, check: &str, _: &Path) -> Option<i32> {
            self.built
                .lock()
                .unwrap()
                .push((commit.to_owned(), check.to_owned()));
            let mut exits = self.exits.lock().unwrap();
            let left = exits
                .entry((commit.to_owned(), check.to_owned()))
                .or_insert_with(|| vec![0]);
            Some(if left.len() > 1 {
                left.remove(0)
            } else {
                left[0]
            })
        }
        fn quarantined(&self, _: &str) -> Vec<String> {
            self.quarantined.clone()
        }
        fn revert(&self, bad: &Bad) -> Result<Merged, String> {
            self.reverted.lock().unwrap().push(bad.commit.clone());
            Ok(Merged {
                pr: 9,
                commit: Some("revert".to_owned()),
            })
        }
        fn quarantine(
            &self,
            test: &str,
            _: Option<u64>,
            _: &str,
            _: &str,
        ) -> Result<Merged, String> {
            self.put_in_quarantine.lock().unwrap().push(test.to_owned());
            Ok(Merged {
                pr: 8,
                commit: None,
            })
        }
        fn on_main(&self, _: &str) -> bool {
            false
        }
        fn wait(&self, _: u64) {}
    }

    fn state(name: &str) -> State {
        let root =
            std::env::temp_dir().join(format!("merge-net-watch-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let state = State { root };
        state.make().unwrap();
        state
    }

    fn names(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    fn changed(text: &str) -> Changed {
        Changed {
            moved: names(text),
            broken: Vec::new(),
        }
    }

    fn findings(state: &State) -> Vec<Value> {
        let mut found: Vec<Value> = fs::read_dir(state.root.join("findings"))
            .unwrap()
            .map(|entry| {
                serde_json::from_str(&fs::read_to_string(entry.unwrap().path()).unwrap()).unwrap()
            })
            .collect();
        found.sort_by_key(|finding| finding["kind"].as_str().unwrap_or_default().to_owned());
        found
    }

    #[test]
    fn changed_checks_lines_are_read_by_their_kind() {
        let read = Changed::parse(
            "added    vm-a\nchanged  b\nremoved  vm-c\nbroken   vm-d\nchanged  vm-e\n",
        );
        assert_eq!(read.vm_tests(), names("vm-a vm-e"));
        assert_eq!(read.broken, names("vm-d"));
    }

    #[test]
    fn a_merge_subject_names_its_pr() {
        assert_eq!(
            pr_of("Merge pull request #1660 from someone/branch"),
            Some(1660)
        );
        assert_eq!(pr_of("fix(zsh): something"), None);
    }

    #[test]
    fn a_head_whose_tests_pass_is_the_next_base_and_runs_only_vm_tests() {
        let state = state("passes");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a b vm-c"),
            ..Stand::default()
        };
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h".into())
        );
        let mut built = world.built();
        built.sort();
        assert_eq!(
            built,
            vec![("h".into(), "vm-a".into()), ("h".into(), "vm-c".into())]
        );
        assert_eq!(state.heads(), (Some("h".into()), Some("h".into())));
        assert_eq!(round(&world, &state, "s").unwrap(), Round::Idle);
        assert_eq!(
            world.built().len(),
            2,
            "a head already passed runs nothing again"
        );
    }

    #[test]
    fn a_test_that_passed_at_a_commit_never_runs_there_again() {
        let state = state("never-again");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            ..Stand::default()
        };
        assert_eq!(attempt(&world, &state, "h", "vm-a"), Outcome::Passed);
        assert_eq!(attempt(&world, &state, "h", "vm-a"), Outcome::Passed);
        assert_eq!(world.built().len(), 1);
    }

    #[test]
    fn a_run_no_builder_took_is_sent_again_and_is_no_failure() {
        let state = state("unsent");
        let world = Stand::default().exits("h", "vm-a", &[UNSENT, UNSENT, 0]);
        assert_eq!(attempt(&world, &state, "h", "vm-a"), Outcome::Passed);
        assert_eq!(state.runs("h", "vm-a"), vec![Ran::Passed]);
        let world = Stand::default().exits("h", "vm-b", &[UNSENT]);
        assert_eq!(attempt(&world, &state, "h", "vm-b"), Outcome::Unsent);
        assert_eq!(state.runs("h", "vm-b"), vec![]);
    }

    #[test]
    fn a_test_that_fails_then_passes_is_quarantined_not_reverted() {
        let state = state("flaky");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 0]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h".into())
        );
        assert_eq!(*world.put_in_quarantine.lock().unwrap(), names("vm-a"));
        assert!(world.reverted.lock().unwrap().is_empty());
        assert_eq!(findings(&state)[0]["kind"], "flaky");
        assert!(state.hold().is_none());
    }

    #[test]
    fn a_test_that_fails_twice_reverts_its_merge_and_holds_the_switch() {
        let state = state("fails");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a vm-b"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("h".into())
        );
        assert_eq!(*world.reverted.lock().unwrap(), names("h"));
        let hold = state.hold().unwrap();
        assert_eq!(hold["bad"], "h");
        assert_eq!(hold["revert_pr"], 9);
        assert_eq!(hold["revert_commit"], "revert");
        let found = findings(&state);
        assert_eq!(found[0]["kind"], "failed");
        assert_eq!(found[0]["owner_pr"], 1);
        assert_eq!(
            state.heads(),
            (Some("h".into()), Some("h".into())),
            "the held test moves the base on"
        );
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Idle,
            "a head is failed once"
        );
    }

    #[test]
    fn a_quarantined_test_that_fails_reverts_nothing() {
        let state = state("quarantined");
        let world = Stand {
            head: "h".into(),
            merges: names("h"),
            changed: changed("vm-a"),
            quarantined: names("vm-a"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1, 1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h".into())
        );
        assert!(world.reverted.lock().unwrap().is_empty());
        assert_eq!(findings(&state)[0]["kind"], "failed-in-quarantine");
    }

    #[test]
    fn the_merge_that_broke_a_test_is_found_among_several() {
        let state = state("bisect");
        let world = Stand {
            head: "m5".into(),
            merges: names("m1 m2 m3 m4 m5"),
            changed: changed("vm-a"),
            missing: vec![("m1".into(), "vm-a".into())],
            ..Stand::default()
        }
        .exits("m2", "vm-a", &[1])
        .exits("m3", "vm-a", &[1])
        .exits("m4", "vm-a", &[1])
        .exits("m5", "vm-a", &[1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Failed("m5".into())
        );
        assert_eq!(*world.reverted.lock().unwrap(), names("m2"));
        assert!(
            !world.built().contains(&("m1".into(), "vm-a".into())),
            "m1 hasn't the test"
        );
    }

    #[test]
    fn a_test_held_for_a_revert_waits_for_it() {
        let state = state("pending");
        state
            .set_hold(&json!({ "test": "vm-a", "bad": "b", "revert_pr": 9, "revert_commit": "r" }))
            .unwrap();
        let world = Stand {
            head: "h".into(),
            merges: names("b h"),
            changed: changed("vm-a vm-b"),
            ..Stand::default()
        }
        .exits("h", "vm-a", &[1]);
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h".into())
        );
        assert_eq!(world.built(), vec![("h".into(), "vm-b".into())]);
        assert!(state.hold().is_some(), "the hold waits for the revert");
        assert!(world.reverted.lock().unwrap().is_empty());
    }

    #[test]
    fn a_test_fixed_forward_clears_a_hold_with_no_revert() {
        let state = state("forward");
        state
            .set_hold(&json!({ "test": "vm-a", "bad": "b", "revert_pr": null }))
            .unwrap();
        let world = Stand {
            head: "h".into(),
            merges: names("b h"),
            changed: changed("vm-a"),
            ..Stand::default()
        };
        assert_eq!(
            round(&world, &state, "s").unwrap(),
            Round::Passed("h".into())
        );
        assert!(state.hold().is_none());
    }

    #[test]
    fn a_held_machine_starts_nothing() {
        let state = state("held");
        let world = Stand {
            head: "h".into(),
            changed: changed("vm-a"),
            held: Some("a reboot".into()),
            ..Stand::default()
        };
        assert_eq!(round(&world, &state, "s").unwrap(), Round::Idle);
        assert!(world.built().is_empty());
    }
}
