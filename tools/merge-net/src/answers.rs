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
//! own. A run again says `why` besides, what main moved: `lists`, only
//! lists each change adds to, whose own checks run again; or `files`,
//! files its checks read, so changed-checks runs again over it, alone or
//! on the merge train, as the train's own record says. Lines kept
//! before 18 November have no `why`. An answer that turns on no
//! file, a head off main or one never run, isn't kept, so a change
//! edited after its pass is never counted here. A run again is kept only
//! once something runs: a head that boards the merge train, or one on
//! main that runs alone; a head behind main that can't board keeps none,
//! nor does one off for a reason with its own step, such as a draft.

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
    /// It runs changed-checks again, or a list's own check, as `Rerun`
    /// says.
    Regate(Rerun),
}

/// Why a change that passed earlier runs again: main moved files it
/// shares since. On 17 November most of these turned on
/// check-descriptions.nix alone, a minute's check, where a file
/// changed-checks reads costs a whole run; a day's count of them can't
/// tell the two apart without it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rerun {
    /// Main moved only lists each change adds to, so their own checks run.
    Lists,
    /// Main moved a file its checks read, so changed-checks runs again
    /// over it, alone or on the merge train.
    Files,
}

impl Verdict {
    fn said(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Hold => "hold",
            Self::Regate(_) => "regate",
        }
    }
}

impl Rerun {
    fn said(self) -> &'static str {
        match self {
            Self::Lists => "lists",
            Self::Files => "files",
        }
    }
}

/// The line kept for an answer of `verdict` on `head` at `at`.
pub fn line(at: u64, head: &str, verdict: Verdict, files: &[String]) -> String {
    let mut said = serde_json::json!({
        "at": at,
        "head": head,
        "verdict": verdict.said(),
        "files": files,
    });
    if let Verdict::Regate(rerun) = verdict {
        said["why"] = rerun.said().into();
    }
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
        let ran = line(6, "def", Verdict::Regate(Rerun::Files), &[]);
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

    #[test]
    fn a_run_again_says_whether_main_moved_only_lists_or_files_its_checks_read() {
        let said = |rerun| {
            let kept = line(7, "abc", Verdict::Regate(rerun), &["a.nix".to_owned()]);
            serde_json::from_str::<serde_json::Value>(&kept).unwrap_or_default()
        };
        assert_eq!(
            said(Rerun::Lists)["why"],
            "lists",
            "a run again as main moved only lists didn't say so"
        );
        assert_eq!(
            said(Rerun::Files)["why"],
            "files",
            "a run again as main moved files its checks read didn't say so"
        );
        assert_eq!(
            said(Rerun::Files)["verdict"],
            "regate",
            "a run again wasn't kept as the verdict cadence counts"
        );
    }
}
