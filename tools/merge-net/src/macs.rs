//! A Mac's comparisons a gate passed without, read from nix-offload's record
//! of the commit, so `needs` names each with the command that runs it.
//!
//! Since #4824 a gate that passes on Linux also compares a Mac's checks,
//! when the change touches a file they read. A Mac that's away, stops
//! answering, runs past its limit or is kept out by `--lent-only` doesn't
//! hold the merge: the record keeps the comparison as skipped, one line
//! `{dir, skipped, because, at}` whose `skipped` is the run's arguments for
//! the Mac's system. At 13:07 on 15 November one gate kept a skipped line
//! when equilibrius ran past 20 minutes, and nothing said so to its owner.
//!
//! A comparison run later at the same commit, by hand or by a gate again,
//! keeps a `run` line with those same arguments beside it, which clears it.

use std::fmt::Write as _;

/// A Mac's comparison a gate skipped: its arguments, and why.
#[derive(Debug, PartialEq, Eq)]
pub struct Skipped {
    pub arguments: Vec<String>,
    pub because: String,
}

/// Each comparison `record` keeps as skipped that no run with its
/// arguments has passed since, the newest reason for each, oldest first.
pub fn skipped(record: &str) -> Vec<Skipped> {
    let _ = record;
    Vec::new()
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
        let record = [run(LINUX), skip(MAC, "equilibrius nearby doesn't answer", 2)].concat();
        assert_eq!(
            said(&record, "abcd1234"),
            "a Mac's checks weren't compared at abcd1234, since equilibrius nearby doesn't \
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
    fn a_record_with_nothing_skipped_says_nothing() {
        assert_eq!(
            said(&run(LINUX), "abcd1234"),
            "",
            "a record with nothing skipped named a Mac's comparison"
        );
    }
}
