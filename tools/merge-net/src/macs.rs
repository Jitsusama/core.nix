//! A Mac's comparisons a gate passed without, read from nix-offload's record
//! of the commit, so `needs` names each with the command that runs it.
//!
//! Since #4824 a gate that passes on Linux also compares a Mac's checks,
//! when the change touches a file they read. A Mac that's away, stops
//! answering, runs past its limit or is kept out by `--lent-only` doesn't
//! hold the merge: the record keeps the comparison as skipped, one line
//! `{dir, skipped, because, at}` whose `skipped` is the run's arguments for
//! the Mac's system. At 13:07 on 15 November one gate kept a skipped line
//! when the Mac ran past 20 minutes, and nothing said so to its owner.
//!
//! A comparison run later at the same commit, by hand or by a gate again,
//! keeps a `run` line with those same arguments beside it, which clears it.
//!
//! The net runs each one itself once a Mac answers. Most skipped lines
//! aren't kept under the head a PR merged at: of 17 in a day on 15
//! November, 4 were, since a gate replayed onto a newer main keeps its
//! record under a commit of its own tree. So `ready`, which reads every
//! record that speaks for the head, marks the head in the net's state,
//! and the net compares the merge of a marked head on a Mac, against its
//! first parent, as a check of its own name.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// The system a Mac's checks are built for.
pub const SYSTEM: &str = "aarch64-darwin";

/// What the check comparing a Mac's checks at a merge is named, before
/// the merge's first parent, which it compares against.
pub const CHECK: &str = "a-macs-checks-since-";

/// The check that compares a Mac's checks at a merge against `base`.
pub fn check(base: &str) -> String {
    format!("{CHECK}{base}")
}

/// The base a Mac's comparison `check` compares against, if it is one.
pub fn base_of(check: &str) -> Option<&str> {
    check.strip_prefix(CHECK).filter(|base| !base.is_empty())
}

/// Mark `head` in `root` as merging with a Mac's comparison skipped,
/// when any of `records`, those that speak for it, keeps one.
pub fn keep(root: &Path, head: &str, records: &[String]) -> std::io::Result<()> {
    if records.iter().all(|record| skipped(record).is_empty()) {
        return Ok(());
    }
    fs::create_dir_all(root.join("macs"))?;
    fs::write(root.join("macs").join(head), "")
}

/// Whether `head` was marked in `root` as merging with a comparison skipped.
pub fn skipped_at(root: &Path, head: &str) -> bool {
    !head.is_empty() && root.join("macs").join(head).is_file()
}

/// A Mac's comparison a gate skipped: its arguments, and why.
#[derive(Debug, PartialEq, Eq)]
pub struct Skipped {
    pub arguments: Vec<String>,
    pub because: String,
}

/// Each comparison `record` keeps as skipped that no run with its
/// arguments has passed since, the newest reason for each, oldest first.
pub fn skipped(record: &str) -> Vec<Skipped> {
    let words = |line: &serde_json::Value, key: &str| -> Option<Vec<String>> {
        let words = line[key].as_array()?;
        Some(
            words
                .iter()
                .filter_map(|word| word.as_str().map(str::to_owned))
                .collect(),
        )
    };
    let lines: Vec<serde_json::Value> = record
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .filter(|line: &serde_json::Value| line["dir"].is_null())
        .collect();
    let ran: Vec<Vec<String>> = lines.iter().filter_map(|line| words(line, "run")).collect();
    let mut skipped: Vec<Skipped> = Vec::new();
    for line in &lines {
        let Some(arguments) = words(line, "skipped") else {
            continue;
        };
        if ran.contains(&arguments) {
            continue;
        }
        let because = line["because"]
            .as_str()
            .unwrap_or("nothing said why")
            .to_owned();
        match skipped.iter_mut().find(|kept| kept.arguments == arguments) {
            Some(kept) => kept.because = because,
            None => skipped.push(Skipped { arguments, because }),
        }
    }
    skipped
}

/// What `needs` says of the comparisons skipped at `head`, or nothing.
pub fn said(record: &str, head: &str) -> String {
    let mut said = String::new();
    for skipped in skipped(record) {
        let _ = writeln!(
            said,
            "a Mac's checks weren't compared at {head}, since {}: once a Mac answers, run \
             this in a tree at {head}\n  nix-offload there --changed {}",
            skipped.because,
            skipped.arguments.join(" ")
        );
    }
    said
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAC: &str = r#""--build","--no-vm","--base","bbbb","--system","aarch64-darwin""#;
    const LINUX: &str = r#""--build","--no-vm","--base","bbbb","--system","x86_64-linux""#;

    fn skip(arguments: &str, because: &str, at: u64) -> String {
        format!(
            "{{\"at\":{at},\"because\":\"{because}\",\"dir\":null,\"skipped\":[{arguments}]}}\n"
        )
    }

    fn run(arguments: &str) -> String {
        format!("{{\"dir\":null,\"printed\":\"\",\"run\":[{arguments}],\"started\":1}}\n")
    }

    #[test]
    fn a_skipped_comparison_is_named_with_why_and_how_to_run_it() {
        let record = [run(LINUX), skip(MAC, "orchard nearby doesn't answer", 2)].concat();
        assert_eq!(
            said(&record, "abcd1234"),
            "a Mac's checks weren't compared at abcd1234, since orchard nearby doesn't \
             answer: once a Mac answers, run this in a tree at abcd1234\n  \
             nix-offload there --changed --build --no-vm --base bbbb --system aarch64-darwin\n",
            "a skipped comparison wasn't named with its reason and the command that runs it"
        );
    }

    #[test]
    fn a_comparison_run_since_at_the_commit_clears_it() {
        let record = [skip(MAC, "away", 2), run(LINUX), run(MAC)].concat();
        assert_eq!(
            skipped(&record),
            Vec::new(),
            "a comparison that passed later at the commit was still named as skipped"
        );
    }

    #[test]
    fn a_skipped_comparison_is_named_once_with_its_newest_reason() {
        let record = [
            skip(MAC, "away", 2),
            skip(MAC, "ran past 20 minutes", 3),
            skip(MAC, "in a tree", 4).replace("\"dir\":null", "\"dir\":\"tools\""),
        ]
        .concat();
        let words: Vec<String> = ["--build", "--no-vm", "--base", "bbbb"]
            .into_iter()
            .chain(["--system", "aarch64-darwin"])
            .map(str::to_owned)
            .collect();
        assert_eq!(
            skipped(&record),
            vec![Skipped {
                arguments: words,
                because: "ran past 20 minutes".to_owned(),
            }],
            "a comparison skipped twice wasn't named once, with its newest reason"
        );
    }

    #[test]
    fn a_comparison_check_gives_back_its_base() {
        assert_eq!(
            base_of(&check("bbbb")),
            Some("bbbb"),
            "a Mac's comparison check didn't give back the base it was named for"
        );
        assert_eq!(
            base_of("vm-boots"),
            None,
            "a VM test was read as a comparison"
        );
    }

    #[test]
    fn a_head_is_marked_only_when_a_record_for_it_skipped() {
        let root = std::env::temp_dir().join(format!("merge-net-macs-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        keep(&root, "aaaa", &[run(LINUX), String::new()]).expect("the mark wasn't written");
        keep(&root, "cccc", &[run(LINUX), skip(MAC, "away", 2)]).expect("the mark wasn't written");
        let marked = (skipped_at(&root, "aaaa"), skipped_at(&root, "cccc"));
        let _ = fs::remove_dir_all(&root);
        assert_eq!(
            marked,
            (false, true),
            "a head was marked when no record skipped, or unmarked when one did"
        );
    }

    #[test]
    fn a_record_with_nothing_skipped_says_nothing() {
        assert_eq!(
            said(&run(LINUX), "abcd1234"),
            "",
            "a record with nothing skipped named a Mac's comparison"
        );
    }
}
