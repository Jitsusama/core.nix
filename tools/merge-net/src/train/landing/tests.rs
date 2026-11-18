//! Landing a passed candidate through a stand-in for GitHub that merges
//! and reverts in a clone of the test's own origin, each way GitHub can
//! answer.

use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};

use super::super::tests::{based, change, git_in, merging, root};
use super::super::{Merging, cut};
use super::*;

/// GitHub as the train asks it, merging and reverting pull requests in a
/// clone of the test's origin, and noting each ask.
struct Fake {
    clone: PathBuf,
    asked: RefCell<Vec<String>>,
    /// The pull request whose merge GitHub refuses.
    refuses: Option<u64>,
    /// The pull request whose first merge GitHub refuses as its base moved.
    refuses_once: Cell<Option<u64>>,
    /// A pull request GitHub refuses as not mergeable this many times
    /// more, while it works out whether it can merge.
    refuses_for: Cell<Option<(u64, u32)>>,
    /// The pull request GitHub merges but answers with an error.
    merges_then_errs: Option<u64>,
    /// The pull request GitHub merges, answering with an error, and then
    /// can't be asked about.
    cant_say: Option<u64>,
    /// The pull request before whose merge another change lands on main.
    moved_before: Option<u64>,
    /// The pull request after whose merge another change lands on main.
    moved_after: Option<u64>,
    /// The pull request whose revert GitHub opens but won't merge.
    revert_fails: Option<u64>,
}

impl Fake {
    fn of(place: &Path, origin: &Path) -> Self {
        let clone = place.join("github");
        let (from, to) = (origin.display().to_string(), clone.display().to_string());
        git_in(place, &["clone", "--quiet", &from, &to]).unwrap();
        Self {
            clone,
            asked: RefCell::default(),
            refuses: None,
            refuses_once: Cell::new(None),
            refuses_for: Cell::new(None),
            merges_then_errs: None,
            cant_say: None,
            moved_before: None,
            moved_after: None,
            revert_fails: None,
        }
    }

    /// Git in the clone, on origin's main as it stands.
    fn on_main(&self) -> Result<(), String> {
        git_in(&self.clone, &["fetch", "--quiet", "origin"])?;
        git_in(
            &self.clone,
            &["checkout", "--quiet", "-B", "main", "origin/main"],
        )
        .map(|_| ())
    }

    fn push(&self) -> Result<(), String> {
        git_in(&self.clone, &["push", "--quiet", "origin", "main"]).map(|_| ())
    }

    /// Another change landing on main beside the train.
    fn elsewhere(&self) -> Result<(), String> {
        self.on_main()?;
        fs::write(self.clone.join("elsewhere"), "theirs\n").unwrap();
        git_in(&self.clone, &["add", "elsewhere"])?;
        git_in(&self.clone, &["commit", "--quiet", "-m", "elsewhere"])?;
        self.push()
    }

    /// The merge of pull request `number` on origin's main.
    fn merge_of(&self, number: u64) -> Option<String> {
        self.on_main().ok()?;
        let found = git_in(
            &self.clone,
            &[
                "log",
                "-1",
                "--format=%H",
                "--grep",
                // Anchored, as a revert's message names the merge it reverts.
                &format!("^Merge pull request #{number} "),
            ],
        )
        .ok()?;
        (!found.is_empty()).then_some(found)
    }
}

impl Hub for Fake {
    fn merge(&self, merging: &Merging) -> Result<String, String> {
        let number = merging.number;
        self.asked.borrow_mut().push(format!("merge {number}"));
        if self.refuses == Some(number) {
            return Err("Pull Request is not mergeable".to_owned());
        }
        if self.refuses_once.get() == Some(number) {
            self.refuses_once.set(None);
            return Err("Base branch was modified. Review and try the merge again.".to_owned());
        }
        if let Some((refused, left)) = self.refuses_for.get()
            && refused == number
        {
            self.refuses_for
                .set(left.checked_sub(1).map(|left| (refused, left)));
            return Err("Pull Request is not mergeable".to_owned());
        }
        if self.moved_before == Some(number) {
            self.elsewhere()?;
        }
        self.on_main()?;
        git_in(
            &self.clone,
            &[
                "merge",
                "--quiet",
                "--no-ff",
                "-m",
                &merging.subject,
                "-m",
                &merging.title,
                // GitHub's merge is a commit of its own, not the
                // candidate's, though its tree is the same.
                "-m",
                "Merged by GitHub",
                &merging.head,
            ],
        )?;
        self.push()?;
        let made = git_in(&self.clone, &["rev-parse", "HEAD"])?;
        if self.moved_after == Some(number) {
            self.elsewhere()?;
        }
        if self.merges_then_errs == Some(number) || self.cant_say == Some(number) {
            return Err("HTTP 502: Bad Gateway".to_owned());
        }
        Ok(made)
    }

    fn merged(&self, number: u64) -> Result<Option<String>, String> {
        if self.cant_say == Some(number) {
            return Err("gh api: HTTP 502: Bad Gateway".to_owned());
        }
        Ok(self.merge_of(number))
    }

    fn tree(&self, commit: &str) -> Result<String, String> {
        self.on_main()?;
        git_in(&self.clone, &["rev-parse", &format!("{commit}^{{tree}}")])
    }

    fn revert(&self, merging: &Merging) -> Result<u64, String> {
        let number = merging.number;
        self.asked.borrow_mut().push(format!("revert {number}"));
        if self.revert_fails == Some(number) {
            return Err(format!(
                "GitHub's revert, pull request #{}, stays open",
                100 + number
            ));
        }
        let merge = self.merge_of(number).ok_or("no merge to revert")?;
        git_in(&self.clone, &["revert", "--no-edit", "-m", "1", &merge])?;
        self.push()?;
        Ok(100 + number)
    }
}

/// A checkout and the bare origin it pushes to, under a folder named
/// `name`, both on one commit of main, which it gives last.
fn hubbed(name: &str) -> (PathBuf, PathBuf, PathBuf, String) {
    let place = root(name);
    let (origin, checkout) = (place.join("origin.git"), place.join("checkout"));
    let base = based(&checkout);
    let url = origin.display().to_string();
    git_in(&place, &["init", "--quiet", "--bare", "-b", "main", &url]).unwrap();
    git_in(&checkout, &["remote", "add", "origin", &url]).unwrap();
    git_in(&checkout, &["push", "--quiet", "origin", "main"]).unwrap();
    git_in(&checkout, &["fetch", "--quiet", "origin", "main"]).unwrap();
    (place, origin, checkout, base)
}

/// A rider's change of `file` on `base`, pushed as pull request
/// `number`'s branch, as a change on GitHub is.
fn proposed(checkout: &Path, base: &str, file: &str, number: u64) -> String {
    let head = change(checkout, base, file, &format!("{file}\n"));
    let branch = format!("{head}:refs/heads/pull-{number}");
    git_in(checkout, &["push", "--quiet", "origin", &branch]).unwrap();
    head
}

/// The tree `commit` holds in `repo`.
fn tree(repo: &Path, commit: &str) -> String {
    git_in(repo, &["rev-parse", &format!("{commit}^{{tree}}")]).unwrap()
}

/// Two riders, pull requests 1 and 2, cut into a candidate on main by
/// the test's `name`, with what landing them through `hub` did.
fn landed_by(name: &str, hub: impl FnOnce(&Path, &Path) -> Fake) -> Landed {
    let (place, origin, checkout, base) = hubbed(name);
    let git = |arguments: &[&str]| git_in(&checkout, arguments);
    let one = proposed(&checkout, &base, "one", 1);
    let two = proposed(&checkout, &base, "two", 2);
    let riding = [merging(&one, 1), merging(&two, 2)];
    let done = cut(&git, &base, &riding).unwrap();
    let candidate = git(&["rev-parse", "HEAD"]).unwrap();
    let hub = hub(&place, &origin);
    let landing = land(
        &checkout,
        &git,
        &hub,
        &base,
        &candidate,
        &riding,
        &done.merged,
    );
    let main = git_in(&origin, &["rev-parse", "main"]).unwrap();
    let landed = Landed {
        landing,
        asked: hub.asked.take(),
        main_tree: tree(&origin, &main),
        tested_tree: tree(&checkout, &candidate),
        base_tree: tree(&checkout, &base),
        // A main GitHub never merged into holds one commit, with none under it.
        under_main: git_in(&origin, &["rev-parse", "main^"]).unwrap_or_default(),
        holds: ["one", "two", "elsewhere"]
            .into_iter()
            .filter(|file| git_in(&origin, &["cat-file", "-e", &format!("main:{file}")]).is_ok())
            .collect(),
        main,
        one,
        two,
        candidate,
        base,
    };
    fs::remove_dir_all(&place).unwrap();
    landed
}

/// What landing two riders did: how it went, what GitHub was asked,
/// origin's main after, and the trees to judge it by.
struct Landed {
    landing: Landing,
    asked: Vec<String>,
    main: String,
    /// Main's first parent, under whatever landed last.
    under_main: String,
    main_tree: String,
    tested_tree: String,
    base_tree: String,
    /// Which of the riders' files, and another change's, main holds.
    holds: Vec<&'static str>,
    one: String,
    two: String,
    candidate: String,
    base: String,
}

#[test]
fn a_passed_candidate_lands_through_githubs_merges() {
    let landed = landed_by("github-merges", Fake::of);
    assert_eq!(
        landed.landing,
        Landing::Landed(landed.main.clone()),
        "a passed candidate on a main that stood still didn't land as GitHub's merges"
    );
    assert_eq!(
        landed.asked,
        ["merge 1", "merge 2"],
        "GitHub wasn't asked to merge each rider, in the order the candidate did"
    );
    assert_ne!(
        landed.main, landed.candidate,
        "the train pushed its own candidate to main, whose key main doesn't trust"
    );
    assert_eq!(
        landed.main_tree, landed.tested_tree,
        "main's tree after GitHub's merges isn't the one the train tested"
    );
}

#[test]
fn githubs_merges_onto_a_main_that_moved_are_reverted() {
    let landed = landed_by("github-moved", |place, origin| Fake {
        moved_before: Some(2),
        ..Fake::of(place, origin)
    });
    assert!(
        matches!(
            &landed.landing,
            Landing::Reverted { reverted, kept, .. }
                if *reverted == [(landed.two.clone(), 102), (landed.one.clone(), 101)]
                    && kept.is_empty()
        ),
        "the merges onto a main the train didn't test weren't each reverted: {:?}",
        landed.landing
    );
    assert_eq!(
        landed.asked,
        ["merge 1", "merge 2", "revert 2", "revert 1"],
        "each merge the train made wasn't reverted, the newest first"
    );
    assert_eq!(
        landed.holds,
        ["elsewhere"],
        "main after the reverts doesn't hold just the other change"
    );
}

#[test]
fn a_first_merge_github_refuses_leaves_main_alone() {
    let landed = landed_by("github-refuses", |place, origin| Fake {
        refuses: Some(1),
        ..Fake::of(place, origin)
    });
    assert!(
        matches!(landed.landing, Landing::Refused(_)),
        "a merge GitHub refused didn't fall back: {:?}",
        landed.landing
    );
    assert_eq!(
        landed.asked,
        vec!["merge 1"; MERGE_TRIES],
        "the train went on, reverted, or gave up early after GitHub refused its first merge"
    );
    assert_eq!(landed.main, landed.base, "main moved though nothing merged");
}

#[test]
fn the_riders_merged_before_one_github_refuses_are_reverted() {
    let landed = landed_by("github-partly", |place, origin| Fake {
        refuses: Some(2),
        ..Fake::of(place, origin)
    });
    assert!(
        matches!(
            &landed.landing,
            Landing::Reverted { reverted, kept, .. }
                if *reverted == [(landed.one.clone(), 101)] && kept.is_empty()
        ),
        "the rider merged before GitHub refused the next, a tree no gate ran, stayed: {:?}",
        landed.landing
    );
    let mut asked = vec!["merge 1"];
    asked.extend(vec!["merge 2"; MERGE_TRIES]);
    asked.push("revert 1");
    assert_eq!(
        landed.asked, asked,
        "the merge made before the refusal wasn't reverted alone, once GitHub kept refusing"
    );
    assert_eq!(
        landed.main_tree, landed.base_tree,
        "main after the revert doesn't hold the tree it had before the train"
    );
}

#[test]
fn a_change_merged_just_after_the_train_leaves_its_merges_landed() {
    let landed = landed_by("github-after", |place, origin| Fake {
        moved_after: Some(2),
        ..Fake::of(place, origin)
    });
    assert_eq!(
        landed.landing,
        Landing::Landed(landed.under_main.clone()),
        "the train didn't land as its own last merge, under the change after it"
    );
    assert_eq!(
        landed.asked,
        ["merge 1", "merge 2"],
        "the train reverted right merges for a change that landed after them"
    );
    assert_eq!(
        landed.holds,
        ["one", "two", "elsewhere"],
        "main doesn't hold both riders and the change after them"
    );
}

#[test]
fn a_merge_refused_as_its_base_moved_is_asked_again() {
    let landed = landed_by("github-again", |place, origin| Fake {
        refuses_once: Cell::new(Some(2)),
        ..Fake::of(place, origin)
    });
    assert_eq!(
        landed.landing,
        Landing::Landed(landed.main.clone()),
        "a merge GitHub refused as the train's own last merge moved main didn't land"
    );
    assert_eq!(
        landed.asked,
        ["merge 1", "merge 2", "merge 2"],
        "the refused merge wasn't asked again"
    );
}

#[test]
fn a_merge_refused_while_github_works_it_out_is_asked_again() {
    let landed = landed_by("github-working", |place, origin| Fake {
        refuses_for: Cell::new(Some((2, 2))),
        ..Fake::of(place, origin)
    });
    assert_eq!(
        landed.landing,
        Landing::Landed(landed.main.clone()),
        "a merge GitHub refused while it worked out whether it could merge didn't land"
    );
    assert_eq!(
        landed.asked,
        ["merge 1", "merge 2", "merge 2", "merge 2", "merge 2"],
        "the refused merge wasn't asked again until GitHub took it"
    );
}

#[test]
fn a_merge_github_cant_say_it_made_keeps_every_merge_and_says_so() {
    let landed = landed_by("github-unsaid", |place, origin| Fake {
        cant_say: Some(2),
        ..Fake::of(place, origin)
    });
    assert!(
        matches!(
            &landed.landing,
            Landing::Reverted { reverted, kept, why }
                if reverted.is_empty()
                    && *kept == [landed.one.clone(), landed.two.clone()]
                    && why.contains("#2")
        ),
        "a merge GitHub couldn't say it made wasn't kept and said, with those under it: {:?}",
        landed.landing
    );
    assert_eq!(
        landed.asked,
        ["merge 1", "merge 2"],
        "the train reverted under a merge that may stand, leaving a tree nobody wrote"
    );
}

#[test]
fn a_merge_github_made_but_answered_with_an_error_counts_as_made() {
    let landed = landed_by("github-lost", |place, origin| Fake {
        merges_then_errs: Some(1),
        ..Fake::of(place, origin)
    });
    assert_eq!(
        landed.landing,
        Landing::Landed(landed.main.clone()),
        "a merge GitHub made, its answer lost, didn't count as made"
    );
    assert_eq!(
        landed.main_tree, landed.tested_tree,
        "main's tree isn't the one the train tested"
    );
}

#[test]
fn a_revert_github_wont_merge_keeps_it_and_those_under_it_on_main() {
    let landed = landed_by("github-kept", |place, origin| Fake {
        moved_before: Some(2),
        revert_fails: Some(2),
        ..Fake::of(place, origin)
    });
    assert!(
        matches!(
            &landed.landing,
            Landing::Reverted { reverted, kept, why }
                if reverted.is_empty()
                    && *kept == [landed.one.clone(), landed.two.clone()]
                    && why.contains("#102")
        ),
        "a revert that failed read as made, or didn't name its pull request: {:?}",
        landed.landing
    );
    assert_eq!(
        landed.asked,
        ["merge 1", "merge 2", "revert 2"],
        "the train reverted under a merge it couldn't revert, leaving a tree nobody wrote"
    );
}
