//! The candidate whose gate runs, kept under the train's folder while it
//! runs, so a train started again runs that same candidate again rather
//! than cutting a fresh one over every rider. On 27 November
//! `merge-net-train.service` was restarted by hand at 17:10 and 18:06, and
//! each start cut afresh, losing a run 30 to 45 minutes in and the halving
//! it was part of. Run again, the candidate's derivations already built
//! come back from the store at once, so only what the stop cut short is
//! built.
//!
//! It runs again only while it still speaks for its riders: main is where
//! it was cut, and each of its riders is still aboard, still its pull
//! request's head and fallen from nothing. A rider dropped by hand, its
//! `riders/<head>.json` taken away or its `fell/<head>` written, is no
//! longer aboard, so the train cuts afresh without it. The search a split
//! was in starts afresh either way, as after a hand-over.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{Cut, Merging, fell, rider_at};
use crate::state::short;

/// The file under the net's state folder `root` naming the candidate whose
/// gate runs.
fn running_file(root: &Path) -> PathBuf {
    super::folder(root).join("running.json")
}

/// A candidate whose gate runs: its commit, the main it was cut on, and
/// the heads it merged, in the order it merged them.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Running {
    pub candidate: String,
    pub base: String,
    pub merged: Vec<String>,
}

/// Keep `running` under `root` as the candidate whose gate runs. One that
/// can't be kept is said, and the gate runs anyway: a stop then only cuts
/// afresh, as before.
pub(super) fn keep(root: &Path, running: &Running) {
    let said = json!({
        "candidate": running.candidate,
        "base": running.base,
        "merged": running.merged,
    });
    if let Err(why) = fs::write(running_file(root), format!("{said}\n")) {
        println!(
            "merge-net: the train couldn't keep {} as running, so a stop cuts afresh: {why}",
            short(&running.candidate)
        );
    }
}

/// Say under `root` that no candidate's gate runs, as once one ends.
pub(super) fn ended(root: &Path) {
    // A file already gone says the same: no candidate's gate runs.
    let _ = fs::remove_file(running_file(root));
}

/// The candidate whose gate ran when the last ride under `root` stopped,
/// if one did.
pub(super) fn kept(root: &Path) -> Option<Running> {
    // No file says the last ride stopped between candidates, so the train
    // cuts afresh, as it does when the file can't be read.
    let words = fs::read_to_string(running_file(root)).ok()?;
    let said: Value = serde_json::from_str(&words).ok()?;
    Some(Running {
        candidate: said["candidate"].as_str()?.to_owned(),
        base: said["base"].as_str()?.to_owned(),
        merged: said["merged"]
            .as_array()?
            .iter()
            .map(|head| head.as_str().map(str::to_owned))
            .collect::<Option<Vec<String>>>()?,
    })
}

/// The candidate `kept` from a ride that stopped while its gate ran, put
/// back in the train's checkout through `git`, when it still speaks for
/// its riders: cut on `base`, main now, and each of its riders aboard
/// under `root` and among `mergings`, those still their pull request's
/// head. None, and the train cuts afresh, otherwise.
pub(super) fn resumed(
    root: &Path,
    kept: Option<Running>,
    git: &dyn Fn(&[&str]) -> Result<String, String>,
    base: &str,
    mergings: &[Merging],
) -> Option<Cut> {
    let kept = kept?;
    let aboard = |head: &String| {
        rider_at(root, head).is_some()
            && fell(root, head).is_none()
            && mergings.iter().any(|merging| &merging.head == head)
    };
    let speaks = kept.base == base && !kept.merged.is_empty() && kept.merged.iter().all(aboard);
    let put_back = speaks
        && git(&[
            "checkout",
            "--quiet",
            "--force",
            "-B",
            "train",
            &kept.candidate,
        ])
        .is_ok()
        && git(&["rev-parse", "HEAD"]).as_deref() == Ok(kept.candidate.as_str());
    if !put_back {
        println!(
            "merge-net: the train started again while {} ran, but main moved, a rider left or \
             the candidate is gone, so it cuts afresh",
            short(&kept.candidate)
        );
        return None;
    }
    println!(
        "merge-net: the train started again while {} ran, its riders aboard and main where it \
         was, so it runs that candidate again",
        short(&kept.candidate)
    );
    Some(Cut {
        merged: kept.merged,
        conflicted: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::super::tests::{based, change, git_in, merging, root};
    use super::super::{Rider, board, cut, fall, fell_folder, riders_folder};
    use super::*;

    /// A state folder holding riders `a` and `b` aboard, both merged into a
    /// candidate cut on main in a repository of the test's own, kept as
    /// running: the state folder, the repository, main, and the candidate.
    fn stopped_mid_run(name: &str) -> (PathBuf, PathBuf, String, Running) {
        let (state, repo) = (root(&format!("{name}-state")), root(name));
        let base = based(&repo);
        let heads = [
            change(&repo, &base, "a", "a\n"),
            change(&repo, &base, "b", "b\n"),
        ];
        let git = |arguments: &[&str]| git_in(&repo, arguments);
        let mergings = [merging(&heads[0], 1), merging(&heads[1], 2)];
        let done = cut(&git, &base, &mergings).unwrap();
        assert_eq!(
            done.merged, heads,
            "the test's candidate didn't merge both riders"
        );
        for (head, at) in heads.iter().zip(0..) {
            let rider = Rider {
                head: head.clone(),
                passed: None,
                branch: format!("jitsusama/{at}"),
                at,
            };
            board(&state, &rider).unwrap();
        }
        let running = Running {
            candidate: git(&["rev-parse", "HEAD"]).unwrap(),
            base: base.clone(),
            merged: done.merged,
        };
        keep(&state, &running);
        // A stop leaves the checkout anywhere: the next cut moves it.
        git(&["checkout", "--quiet", "--detach", &base]).unwrap();
        (state, repo, base, running)
    }

    /// Each rider of `running` as `aboard` gives it, still its head.
    fn mergings_of(running: &Running) -> Vec<Merging> {
        running
            .merged
            .iter()
            .zip(1..)
            .map(|(head, number)| merging(head, number))
            .collect()
    }

    #[test]
    fn a_train_started_again_runs_its_running_candidate_again() {
        let (state, repo, base, running) = stopped_mid_run("resumes");
        let git = |arguments: &[&str]| git_in(&repo, arguments);
        let mergings = mergings_of(&running);
        let candidate = running.candidate.clone();
        assert_eq!(
            kept(&state).as_ref(),
            Some(&running),
            "the running candidate wasn't kept as it was"
        );
        assert_eq!(
            resumed(&state, kept(&state), &git, &base, &mergings),
            Some(Cut {
                merged: running.merged,
                conflicted: Vec::new(),
            }),
            "a train started again cut afresh though its candidate still spoke for its riders"
        );
        assert_eq!(
            git(&["rev-parse", "HEAD"]).unwrap(),
            candidate,
            "the checkout wasn't put back at the running candidate"
        );
        ended(&state);
        assert_eq!(
            kept(&state),
            None,
            "a gate that ended left its candidate running"
        );
        fs::remove_dir_all(&state).unwrap();
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn a_train_started_again_cuts_afresh_once_main_moved_or_a_rider_left() {
        let (state, repo, base, running) = stopped_mid_run("recuts");
        let git = |arguments: &[&str]| git_in(&repo, arguments);
        let mergings = mergings_of(&running);
        let moved = change(&repo, &base, "main", "moved\n");
        assert_eq!(
            resumed(&state, kept(&state), &git, &moved, &mergings),
            None,
            "a candidate was run again on a main that moved under it"
        );
        assert_eq!(
            resumed(&state, kept(&state), &git, &base, &mergings[..1]),
            None,
            "a candidate was run again though a rider is no longer its pull request's head"
        );
        // Dropped by hand, as Joel drops one: its file taken, its fall said.
        let dropped = &running.merged[1];
        fs::remove_file(riders_folder(&state).join(format!("{dropped}.json"))).unwrap();
        assert_eq!(
            resumed(&state, kept(&state), &git, &base, &mergings),
            None,
            "a candidate was run again though a rider's file was taken away"
        );
        fs::create_dir_all(fell_folder(&state)).unwrap();
        fs::write(fell_folder(&state).join(dropped), "dropped by hand\n").unwrap();
        assert_eq!(
            resumed(&state, kept(&state), &git, &base, &mergings),
            None,
            "a candidate was run again though a rider was dropped by hand"
        );
        fs::remove_dir_all(&state).unwrap();
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn a_rider_fallen_but_still_filed_counts_as_gone() {
        let (state, repo, base, running) = stopped_mid_run("fallen");
        let git = |arguments: &[&str]| git_in(&repo, arguments);
        let mergings = mergings_of(&running);
        fall(&state, &running.merged[0], "dropped by hand").unwrap();
        board(
            &state,
            &Rider {
                head: running.merged[0].clone(),
                passed: None,
                branch: "jitsusama/0".to_owned(),
                at: 0,
            },
        )
        .unwrap();
        assert_eq!(
            resumed(&state, kept(&state), &git, &base, &mergings),
            None,
            "a candidate was run again though a rider's fall was written"
        );
        fs::remove_dir_all(&state).unwrap();
        fs::remove_dir_all(&repo).unwrap();
    }
}
