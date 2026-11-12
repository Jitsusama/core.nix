//! What `ready` answered, kept a line each in `answers.jsonl` under the
//! net's state folder, so a day's holds and runs again can be counted by
//! the files that caused them. Only the session asking saw an answer
//! before: on 12 November they were counted from every session's log,
//! 44 holds and 35 runs again by 07:00, 8 of the holds on flake.nix.
//!
//! A line is `{at, head, verdict, files}`: when, in seconds since the
//! epoch; the head asked about; `ok`, `hold` or `regate`; and the files
//! the answer turned on. For a hold, those an earlier gate shares; for a
//! run again, those main moved under the change; for `ok`, the change's
//! own. An answer that turns on no file, a head off main or one never
//! run, isn't kept.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

/// Which way `ready` answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// It may merge.
    Ok,
    /// It waits for an earlier gate sharing its files.
    Hold,
    /// It runs changed-checks again, or a list's own check.
    Regate,
}

impl Verdict {
    fn said(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Hold => "hold",
            Self::Regate => "regate",
        }
    }
}

/// The line kept for an answer of `verdict` on `head` at `at`.
pub fn line(at: u64, head: &str, verdict: Verdict, files: &[String]) -> String {
    let said = serde_json::json!({
        "at": at,
        "head": head,
        "verdict": verdict.said(),
        "files": files,
    });
    format!("{said}\n")
}

/// Keep an answer of `verdict` on `head`, saying so when it can't be,
/// since the answer stands either way: the session asking was told it.
pub fn keep(head: &str, verdict: Verdict, files: &[String]) {
    let root = crate::state::State::from_env().root;
    let kept = line(crate::state::now(), head, verdict, files);
    if let Err(error) = keep_in(&root, &kept) {
        eprintln!(
            "merge-net: this answer wasn't kept in {}: {error}",
            root.display()
        );
    }
}

/// Append `line` to `answers.jsonl` in `root`, in one write, so answers
/// given at once in two trees never run into each other.
fn keep_in(root: &Path, line: &str) -> io::Result<()> {
    fs::create_dir_all(root)?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("answers.jsonl"))?
        .write_all(line.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_answer_is_kept_a_line_after_the_last() {
        let root = std::env::temp_dir().join(format!("merge-net-answers-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let held = line(5, "abc", Verdict::Hold, &["flake.nix".to_owned()]);
        let ran = line(6, "def", Verdict::Regate, &[]);
        keep_in(&root, &held).unwrap();
        keep_in(&root, &ran).unwrap();
        let kept = fs::read_to_string(root.join("answers.jsonl")).unwrap_or_default();
        let _ = fs::remove_dir_all(&root);
        assert_eq!(
            kept,
            held.clone() + &ran,
            "the answers weren't kept in turn"
        );
        assert_eq!(
            held, "{\"at\":5,\"files\":[\"flake.nix\"],\"head\":\"abc\",\"verdict\":\"hold\"}\n",
            "an answer was kept in another shape"
        );
    }
}
