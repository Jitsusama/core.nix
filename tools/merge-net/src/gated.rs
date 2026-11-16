//! What a changed-checks run that passed at a commit already named as moved,
//! read from nix-offload's record of it, so `needs` at that commit asks
//! for no evaluation of its own.
//!
//! `needs` asks changed-checks which VM tests moved, an evaluation of every
//! check on a builder, and since #3654 most changes pay it. Over the 30
//! hours to 07:50 on 12 November it ran 169 times; 73 were at a commit a
//! gate had passed at too, 52 with the gate started first, a median of 15
//! minutes before, so the answer was already kept. With four sessions
//! asking at once those evaluations queued for eight minutes behind each
//! other, after their gates had passed.
//!
//! A run against an older base than the one asked about names more than
//! moved since that base, never less, so it's read too: it may ask for a VM
//! test the newer base would have spared, but never spares one it needs. A
//! run against a base that isn't the merge base or before it, such as a
//! branch it was stacked on, isn't.
//!
//! A head rebased over a main that moved none of its files has no run of
//! its own, but an earlier commit of the same change does, and `ready`
//! already merges it on that run (`ready::Before::Clean`). At 09:09 on 16
//! November `needs` at #5018, so rebased, evaluated for 534 seconds what
//! its earlier pass had named, so that run is read for it too.
//!
//! Each `needs` keeps a line in `needs.jsonl` under the net's state folder,
//! `{at, head, seconds, asked}`: when, the head, how long it took, and
//! whether it read a gate's run (`gate`), an earlier commit's (`earlier`),
//! asked changed-checks (`changed`) or asked nothing (`none`), so the card
//! can say what `needs` costs.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

/// What a run that passed at the commit printed, from nix-offload's
/// `record` of it: the newest that named what moved for `system` from
/// `base`, or from a base `before` says came before it.
pub fn named(
    record: &str,
    base: &str,
    system: &str,
    before: &dyn Fn(&str) -> bool,
) -> Option<String> {
    record
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|line| line["dir"].is_null())
        .filter_map(|line| {
            let printed = line["printed"].as_str()?.to_owned();
            let run: Vec<String> = line["run"]
                .as_array()?
                .iter()
                .filter_map(|word| word.as_str().map(str::to_owned))
                .collect();
            let given = |flag: &str| {
                run.iter()
                    .position(|word| word == flag)
                    .and_then(|at| run.get(at + 1))
            };
            // A run that only kept its base named nothing.
            let named = !run.iter().any(|word| word == "--keep-base");
            let for_system = given("--system").is_none_or(|said| said == system);
            // A run given no base compared with its merge base then, which
            // a commit's merge base only ever moves on from.
            let from = given("--base").is_none_or(|from| from == base || before(from));
            (named && for_system && from).then_some(printed)
        })
        .last()
}

/// What a run that passed already named as moved from `base` for the
/// head, and whose run it was, when the working tree is `clean`, since a
/// run saw only the commit: the head's own `record`, or else the record
/// `earlier` finds of an earlier commit of the same change.
pub fn kept(
    record: &str,
    clean: bool,
    base: &str,
    before: &dyn Fn(&str) -> bool,
    earlier: impl FnOnce() -> Option<String>,
) -> Option<(String, Asked)> {
    if !clean {
        return None;
    }
    if let Some(said) = named(record, base, crate::SYSTEM, before) {
        return Some((said, Asked::Gate));
    }
    named(&earlier()?, base, crate::SYSTEM, before).map(|said| (said, Asked::Earlier))
}

/// What moved from `base` at the head, and how it was learned: from what
/// a run that passed `kept`, and otherwise by `asking` changed-checks.
pub fn moved(
    kept: Option<(String, Asked)>,
    asking: impl FnOnce() -> Option<String>,
) -> (Option<String>, Asked) {
    match kept {
        Some((said, how)) => (Some(said), how),
        None => (asking(), Asked::Changed),
    }
}

/// How a `needs` learned what moved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asked {
    /// From a run that passed at the head.
    Gate,
    /// From a run that passed at an earlier commit of the same change.
    Earlier,
    /// From changed-checks, which evaluated.
    Changed,
    /// It asked nothing, needing no answer.
    Nothing,
}

impl Asked {
    fn said(self) -> &'static str {
        match self {
            Self::Gate => "gate",
            Self::Earlier => "earlier",
            Self::Changed => "changed",
            Self::Nothing => "none",
        }
    }
}

/// The line kept for a `needs` on `head` at `at` that took `seconds`.
pub fn line(at: u64, head: &str, seconds: u64, asked: Asked) -> String {
    let said = serde_json::json!({
        "at": at,
        "head": head,
        "seconds": seconds,
        "asked": asked.said(),
    });
    format!("{said}\n")
}

/// Keep a `needs` that began at `began`, saying so when it can't be, since
/// its answer stands either way.
pub fn keep(head: &str, began: u64, asked: Asked) {
    let root = crate::state::State::from_env().root;
    let now = crate::state::now();
    let kept = line(now, head, now.saturating_sub(began), asked);
    if let Err(error) = keep_in(&root, &kept) {
        eprintln!(
            "merge-net: this needs wasn't kept in {}: {error}",
            root.display()
        );
    }
}

/// Whether `needs` was asked at any of `commits`, by the lines kept in
/// `needs.jsonl`'s text `kept`. On 15 November #4653 merged with its gate
/// passed but `needs` never asked, though it moved the laptop's initrd,
/// which needs would have answered with the boot and hibernation tests.
pub fn asked_at(kept: &str, commits: &[&str]) -> bool {
    kept.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .any(|line| {
            line["head"]
                .as_str()
                .is_some_and(|head| commits.contains(&head))
        })
}

/// Whether `needs` was asked at any of `commits` on this machine.
pub fn asked(commits: &[&str]) -> bool {
    let root = crate::state::State::from_env().root;
    asked_at(
        &fs::read_to_string(root.join("needs.jsonl")).unwrap_or_default(),
        commits,
    )
}

/// Append `line` to `needs.jsonl` in `root`, in one write.
fn keep_in(root: &Path, line: &str) -> io::Result<()> {
    fs::create_dir_all(root)?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("needs.jsonl"))?
        .write_all(line.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "bbbb";

    /// Only `aaaa` came before the base.
    fn before(commit: &str) -> bool {
        commit == "aaaa"
    }

    fn run(words: &str, printed: &str) -> String {
        format!("{{\"dir\":null,\"printed\":\"{printed}\",\"run\":[{words}]}}\n")
    }

    #[test]
    fn a_gate_that_passed_at_the_head_says_what_moved() {
        let record = format!(
            "{{\"check\":\"checks.x86_64-linux.b\",\"dir\":null}}\n{}{}",
            run("\"--system\",\"x86_64-linux\"", "changed  vm-a\\n"),
            run("\"--base\",\"aaaa\",\"--name-only\"", "changed  vm-b\\n"),
        );
        assert_eq!(
            named(&record, BASE, "x86_64-linux", &before).as_deref(),
            Some("changed  vm-b\n"),
            "the newest run that named what moved wasn't read"
        );
    }

    #[test]
    fn a_run_that_named_nothing_or_from_elsewhere_says_nothing() {
        let record = [
            run("\"--keep-base\",\"--system\",\"x86_64-linux\"", ""),
            run("\"--system\",\"aarch64-darwin\"", "changed  vm-a\\n"),
            run("\"--base\",\"cccc\",\"--name-only\"", "changed  vm-a\\n"),
        ]
        .concat();
        assert_eq!(
            named(&record, BASE, "x86_64-linux", &before),
            None,
            "a kept base, another system's run or one from a base after the merge base was read"
        );
        assert_eq!(
            named("", BASE, "x86_64-linux", &before),
            None,
            "nothing passed, yet something was read"
        );
    }

    #[test]
    fn a_clean_head_a_gate_passed_at_asks_for_no_evaluation() {
        let record = run("\"--system\",\"x86_64-linux\"", "changed  vm-a\\n");
        let mut asked = 0;
        let at_head = kept(&record, true, BASE, &before, || None);
        let (said, how) = moved(at_head, || {
            asked += 1;
            Some("changed  vm-z\n".to_owned())
        });
        assert_eq!(
            (said.as_deref(), how, asked),
            (Some("changed  vm-a\n"), Asked::Gate, 0),
            "changed-checks was asked to evaluate what the gate at the head already named"
        );
        let edited = kept(&record, false, BASE, &before, || Some(record.clone()));
        let (said, how) = moved(edited, || Some("changed  vm-z\n".to_owned()));
        assert_eq!(
            (said.as_deref(), how),
            (Some("changed  vm-z\n"), Asked::Changed),
            "a tree with edits the gate never saw was answered from the gate"
        );
    }

    #[test]
    fn a_rebased_head_is_answered_by_its_change_s_earlier_pass() {
        // On 16 November #5018's gate passed at f8b9be72; rebased over
        // main moving none of its files, needs at the new head had no run
        // of its own and evaluated every check again for nine minutes.
        let earlier = run("\"--system\",\"x86_64-linux\"", "changed  vm-a\\n");
        let mut asked = 0;
        let rebased = kept("", true, BASE, &before, || Some(earlier.clone()));
        let (said, how) = moved(rebased, || {
            asked += 1;
            Some("changed  vm-z\n".to_owned())
        });
        assert_eq!(
            (said.as_deref(), how, asked),
            (Some("changed  vm-a\n"), Asked::Earlier, 0),
            "a rebased head evaluated what its change's earlier pass already named"
        );
        let none_earlier = kept("", true, BASE, &before, || None);
        assert_eq!(
            moved(none_earlier, || Some("changed  vm-z\n".to_owned())),
            (Some("changed  vm-z\n".to_owned()), Asked::Changed),
            "a head with no pass of its change's wasn't asked of changed-checks"
        );
    }

    #[test]
    fn only_a_needs_asked_at_one_of_the_commits_counts() {
        let kept = [
            line(10, "h1", 3, Asked::Gate),
            "not a line\n".to_owned(),
            line(20, "h2", 0, Asked::Nothing),
        ]
        .concat();
        assert!(
            asked_at(&kept, &["h9", "h2"]),
            "a needs asked at the change's earlier commit wasn't seen"
        );
        assert!(
            !asked_at(&kept, &["h3"]),
            "a change needs was never asked of was taken as asked"
        );
        assert!(
            !asked_at("", &["h1"]),
            "no needs was kept, yet one was seen"
        );
    }

    #[test]
    fn each_needs_is_kept_with_how_long_it_took() {
        let root = std::env::temp_dir().join(format!("merge-net-needs-{}", std::process::id()));
        keep_in(&root, &line(10, "h1", 3, Asked::Gate)).expect("the first wasn't kept");
        keep_in(&root, &line(20, "h2", 480, Asked::Changed)).expect("the second wasn't kept");
        let kept = fs::read_to_string(root.join("needs.jsonl")).expect("nothing was kept");
        fs::remove_dir_all(&root).expect("the folder wasn't cleared");
        assert_eq!(
            kept,
            "{\"asked\":\"gate\",\"at\":10,\"head\":\"h1\",\"seconds\":3}\n\
             {\"asked\":\"changed\",\"at\":20,\"head\":\"h2\",\"seconds\":480}\n",
            "a needs wasn't kept a line after the last"
        );
    }
}
