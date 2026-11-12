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
//! Each `needs` keeps a line in `needs.jsonl` under the net's state folder,
//! `{at, head, seconds, asked}`: when, the head, how long it took, and
//! whether it read a gate's run (`gate`), asked changed-checks (`changed`)
//! or asked nothing (`none`), so the card can say what `needs` costs.

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

/// What moved from `base` at the head, and how it was learned: from the
/// head's `record` when the working tree is `clean`, since a run there saw
/// only the commit, and otherwise by `asking` changed-checks.
pub fn moved(
    record: &str,
    clean: bool,
    base: &str,
    before: &dyn Fn(&str) -> bool,
    asking: impl FnOnce() -> Option<String>,
) -> (Option<String>, Asked) {
    if clean && let Some(said) = named(record, base, crate::SYSTEM, before) {
        return (Some(said), Asked::Gate);
    }
    (asking(), Asked::Changed)
}

/// How a `needs` learned what moved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asked {
    /// From a run that passed at the head.
    Gate,
    /// From changed-checks, which evaluated.
    Changed,
    /// It asked nothing, needing no answer.
    Nothing,
}

impl Asked {
    fn said(self) -> &'static str {
        match self {
            Self::Gate => "gate",
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
        let (said, how) = moved(&record, true, BASE, &before, || {
            asked += 1;
            Some("changed  vm-z\n".to_owned())
        });
        assert_eq!(
            (said.as_deref(), how, asked),
            (Some("changed  vm-a\n"), Asked::Gate, 0),
            "changed-checks was asked to evaluate what the gate at the head already named"
        );
        let (said, how) = moved(&record, false, BASE, &before, || {
            Some("changed  vm-z\n".to_owned())
        });
        assert_eq!(
            (said.as_deref(), how),
            (Some("changed  vm-z\n"), Asked::Changed),
            "a tree with edits the gate never saw was answered from the gate"
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
