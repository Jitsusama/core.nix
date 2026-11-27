//! How `ready` puts a head that main moved beside on the merge train: it
//! asks `boarding.rs`'s questions of the real gates, git and the train,
//! and says what to do when the head stays off.

use std::process::{Command, Stdio};

use crate::{answers, boarding, git, mark_skipped_macs, needs_heard, run_again, train, turn};

/// What to do about `commit`, on main, once it `boarded` the merge train,
/// which merges it with the others on one run (`train.rs`), or didn't:
/// the step its own for what a run again doesn't end, as
/// `boarding::own_step` says, or else run it again alone.
pub fn ride_or_run_again(commit: &str, boarded: &Result<String, boarding::Off>) -> String {
    let off = match boarded {
        Ok(said) => return said.clone(),
        Err(off) => off,
    };
    if let Some(step) = boarding::own_step(off) {
        return step;
    }
    let again = run_again(commit);
    match train::fell_said(commit) {
        Some(fell) => format!("{fell}; {again}"),
        None => again,
    }
}

/// Whether `commit`, whose pass at `checked` main moved beside, boards the
/// merge train, asked as `boarding.rs` says, with what the train said once
/// it did.
pub fn boards(
    commit: &str,
    main: &str,
    checked: &str,
    files: &[String],
) -> Result<String, boarding::Off> {
    let passes = [commit, checked];
    boarding::boarding(&boarding::Asks {
        kit_first: &|| {
            turn::waits_to_run(commit).map(|said| said.trim_start_matches("merge-net: ").to_owned())
        },
        heard: &|| needs_heard(commit, main, &[checked], files),
        // Marked as may_merge marks it, before the hold that would keep it.
        held: &|| {
            mark_skipped_macs(commit, &passes);
            turn::waiting(commit, &passes, files)
        },
        conflicts: &|| conflicts(commit, main),
        branch: &|| git(&["symbolic-ref", "--quiet", "--short", "HEAD"]).unwrap_or_default(),
        drafted: &train::drafted,
        offer: &|branch| train::offer(commit, (checked != commit).then_some(checked), branch),
        fell: &|| train::fell_said(commit),
    })
}

/// Whether `commit` conflicts with origin's main at `main`, as merging the
/// two would. Where git can't say, the train finds out at its own merge,
/// and what git printed is kept from the session, which it doesn't help.
fn conflicts(commit: &str, main: &str) -> bool {
    Command::new("git")
        .args(["merge-tree", "--write-tree", "--quiet", main, commit])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.code() == Some(1))
}

/// Keep that `commit` runs changed-checks again over `shared`, when it
/// does: on the train, or alone once it couldn't board. A head kept off
/// by what has a step of its own (`boarding::own_step`) keeps none, since
/// it asks `ready` again after that step, and a hold keeps its own line.
/// Its caller keeps none for a head behind main that can't board, since
/// the head it rebases to keeps the one line its change counts.
pub fn keep_regated(commit: &str, boarded: &Result<String, boarding::Off>, shared: &[String]) {
    if boarded
        .as_ref()
        .err()
        .and_then(boarding::own_step)
        .is_none()
    {
        answers::keep(
            commit,
            answers::Verdict::Regate(answers::Rerun::Files),
            shared,
        );
    }
}
