//! Landing a passed candidate through a stand-in for GitHub that merges
//! and reverts in a clone of the test's own origin, each way GitHub can
//! answer.
//!
//! Long because each way GitHub answers a merge or a revert, and each
//! order of them, is a case of its own against the one stand-in.

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
    /// A pull request whose every merge GitHub answers with a server
    /// error, and which it reads unmerged this many times more before it
    /// finishes the merge after all.
    merges_late: Cell<Option<(u64, usize)>>,
    /// The merge GitHub finishes late, once it was asked for.
    late: RefCell<Option<Merging>>,
    /// Whether GitHub answers the late merge's asks after its first with
    /// a stated refusal rather than a server error, as it may while it
    /// finishes the merge.
    refuses_after_error: bool,
    /// How many times GitHub was asked whether pull request 2 merged.
    reads: Cell<usize>,
    /// From which of those reads on, and for how many, GitHub can't say.
    read_fails: Cell<Option<(usize, usize)>>,
    /// The pull request before whose merge another change lands on main.
    moved_before: Option<u64>,
    /// The pull request after whose merge another change lands on main.
    moved_after: Option<u64>,
    /// The pull request whose revert GitHub opens but won't merge.
    revert_fails: Option<u64>,
    /// How many times more GitHub can't say what a merge's tree is.
    tree_fails: Cell<u32>,
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
            merges_late: Cell::new(None),
            late: RefCell::default(),
            refuses_after_error: false,
            reads: Cell::new(0),
            read_fails: Cell::new(None),
            moved_before: None,
            moved_after: None,
            revert_fails: None,
            tree_fails: Cell::new(0),
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

    /// Merge `merging` on origin's main as GitHub does, answering its
    /// merge.
    fn merge_in(&self, merging: &Merging) -> Result<String, String> {
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
        git_in(&self.clone, &["rev-parse", "HEAD"])
    }
}

impl Hub for Fake {
    fn merge(&self, merging: &Merging) -> Result<String, String> {
        let number = merging.number;
        self.asked.borrow_mut().push(format!("merge {number}"));
        if self.refuses == Some(number) {
            return Err(NOT_MERGEABLE.to_owned());
        }
        if self.refuses_once.get() == Some(number) {
            self.refuses_once.set(None);
            return Err(
                "gh api: gh: Base branch was modified. Review and try the merge again. \
                 (HTTP 409)"
                    .to_owned(),
            );
        }
        if let Some((refused, left)) = self.refuses_for.get()
            && refused == number
            && left > 0
        {
            self.refuses_for.set(Some((refused, left - 1)));
            return Err(NOT_MERGEABLE.to_owned());
        }
        if let Some((late, _)) = self.merges_late.get()
            && late == number
        {
            let mut asked_late = self.late.borrow_mut();
            if asked_late.is_some() && self.refuses_after_error {
                return Err(NOT_MERGEABLE.to_owned());
            }
            asked_late.get_or_insert_with(|| merging.clone());
            return Err("gh api: gh: Server Error (HTTP 502)".to_owned());
        }
        if self.moved_before == Some(number) {
            self.elsewhere()?;
        }
        let made = self.merge_in(merging)?;
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
        if number == 2 {
            let read = self.reads.get() + 1;
            self.reads.set(read);
            if let Some((from, count)) = self.read_fails.get()
                && (from..from.saturating_add(count)).contains(&read)
            {
                return Err("gh api: timed out".to_owned());
            }
        }
        if let Some((late, left)) = self.merges_late.get()
            && late == number
        {
            if left > 0 {
                self.merges_late.set(Some((late, left - 1)));
                return Ok(None);
            }
            if let Some(merging) = self.late.borrow_mut().take() {
                return self.merge_in(&merging).map(Some);
            }
        }
        Ok(self.merge_of(number))
    }

    fn tree(&self, commit: &str) -> Result<String, String> {
        if self.tree_fails.get() > 0 {
            self.tree_fails.set(self.tree_fails.get() - 1);
            return Err("gh api: HTTP 502: Bad Gateway".to_owned());
        }
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

/// How GitHub, through `gh`, refuses a merge it can't make.
const NOT_MERGEABLE: &str = "gh api: gh: Pull Request is not mergeable (HTTP 405)";

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
        reads: hub.reads.get(),
        main_tree: tree(&origin, &main),
        tested_tree: tree(&checkout, &candidate),
        // A main GitHub never merged into holds one commit, with none under it.
        under_main: git_in(&origin, &["rev-parse", "main^"]).unwrap_or_default(),
        holds: ["one", "two", "elsewhere"]
            .into_iter()
            .filter(|file| git_in(&origin, &["cat-file", "-e", &format!("main:{file}")]).is_ok())
            .collect(),
        unsigned: git_in(
            &origin,
            &[
                "log",
                "--first-parent",
                "--format=%H %B%x00",
                &format!("{base}..main"),
            ],
        )
        .unwrap()
        .split('\0')
        .map(str::trim)
        .filter(|commit| !commit.is_empty())
        .filter(|commit| !commit.contains("Merged by GitHub") && !commit.ends_with("elsewhere"))
        .filter(|commit| !commit.contains(" Revert "))
        .map(|commit| commit.split(' ').next().unwrap_or_default().to_owned())
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
    /// How many times GitHub was asked whether pull request 2 merged.
    reads: usize,
    main: String,
    /// Main's first parent, under whatever landed last.
    under_main: String,
    main_tree: String,
    tested_tree: String,
    /// Which of the riders' files, and another change's, main holds.
    holds: Vec<&'static str>,
    /// The commits main gained along its first parent that GitHub didn't
    /// make, which a follower trusting only GitHub's key refuses as its
    /// tip, besides another change's.
    unsigned: Vec<String>,
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
    assert!(
        landed.unsigned.is_empty(),
        "main gained commits GitHub didn't make, which a follower trusting it refuses: {:?}",
        landed.unsigned
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
        matches!(
            &landed.landing,
            Landing::Turned { refused, why }
                if *refused == landed.one && why.contains("not mergeable")
        ),
        "a first merge GitHub refused didn't turn that rider alone away: {:?}",
        landed.landing
    );
    assert_eq!(
        landed.asked,
        vec!["merge 1"; MERGE_TRIES],
        "the train went on, reverted, or gave up early after GitHub refused its first merge"
    );
    assert_eq!(landed.main, landed.base, "main moved though nothing merged");
}

// At 02:58 on 19 November GitHub refused a draft, the third rider, and
// the train reverted the two riders it had merged, whose checks had
// passed. A rider GitHub refuses leaves the train, and those landed stay.
#[test]
fn the_riders_merged_before_one_github_refuses_stay_landed() {
    let landed = landed_by("github-partly", |place, origin| Fake {
        refuses: Some(2),
        ..Fake::of(place, origin)
    });
    assert!(
        matches!(
            &landed.landing,
            Landing::Partly { landed: at, kept, refused, why }
                if *at == landed.main
                    && *kept == [landed.one.clone()]
                    && *refused == landed.two
                    && why.contains("not mergeable")
        ),
        "the rider merged before GitHub refused the next didn't stay landed alone: {:?}",
        landed.landing
    );
    let mut asked = vec!["merge 1"];
    asked.extend(vec!["merge 2"; MERGE_TRIES]);
    assert_eq!(
        landed.asked, asked,
        "the train reverted a merge, or went on, once GitHub kept refusing the next"
    );
    assert_eq!(
        landed.holds,
        ["one"],
        "main doesn't hold the landed rider alone"
    );
}

// Three riders, the middle one refused: the first stays, held to the
// candidate's own merge of it rather than its top or the merge after, and
// the third is never asked to merge, so it rides again.
#[test]
fn a_rider_after_the_one_github_refuses_isnt_merged() {
    let (place, origin, checkout, base) = hubbed("github-partly-three");
    let git = |arguments: &[&str]| git_in(&checkout, arguments);
    let one = proposed(&checkout, &base, "one", 1);
    let two = proposed(&checkout, &base, "two", 2);
    let three = proposed(&checkout, &base, "three", 3);
    let riding = [merging(&one, 1), merging(&two, 2), merging(&three, 3)];
    let done = cut(&git, &base, &riding).unwrap();
    let candidate = git(&["rev-parse", "HEAD"]).unwrap();
    let hub = Fake {
        refuses: Some(2),
        ..Fake::of(&place, &origin)
    };

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
    assert!(
        matches!(
            &landing,
            Landing::Partly { landed, kept, refused, .. }
                if *landed == main && *kept == [one.clone()] && *refused == two
        ),
        "the first rider didn't stay landed alone: {landing:?}"
    );
    let mut asked = vec!["merge 1"];
    asked.extend(vec!["merge 2"; MERGE_TRIES]);
    assert_eq!(
        hub.asked.take(),
        asked,
        "the rider after the refused one was asked to merge"
    );
    let held: Vec<&str> = ["one", "two", "three"]
        .into_iter()
        .filter(|file| git_in(&origin, &["cat-file", "-e", &format!("main:{file}")]).is_ok())
        .collect();
    assert_eq!(held, ["one"], "main doesn't hold the first rider alone");
    fs::remove_dir_all(&place).unwrap();
}

// A rider whose head main already held leaves no merge in the candidate,
// nor on main: made last before a refusal, it is held to the candidate
// as it stood before the refused rider's merge, not reverted for want
// of a merge of its own.
#[test]
fn a_rider_main_already_held_before_a_refusal_stays_landed() {
    let (place, origin, checkout, first) = hubbed("github-partly-held");
    let git = |arguments: &[&str]| git_in(&checkout, arguments);
    let one = proposed(&checkout, &first, "one", 1);
    git(&[
        "push",
        "--quiet",
        "origin",
        &format!("{one}:refs/heads/main"),
    ])
    .unwrap();
    git(&["fetch", "--quiet", "origin", "main"]).unwrap();
    let base = one.clone();
    let two = proposed(&checkout, &base, "two", 2);
    let three = proposed(&checkout, &base, "three", 3);
    let riding = [merging(&one, 1), merging(&two, 2), merging(&three, 3)];
    let done = cut(&git, &base, &riding).unwrap();
    let candidate = git(&["rev-parse", "HEAD"]).unwrap();
    let hub = Fake {
        refuses: Some(2),
        ..Fake::of(&place, &origin)
    };

    let landing = land(
        &checkout,
        &git,
        &hub,
        &base,
        &candidate,
        &riding,
        &done.merged,
    );
    assert!(
        matches!(
            &landing,
            Landing::Partly { landed, kept, refused, .. }
                if *landed == base && *kept == [one.clone()] && *refused == two
        ),
        "a rider main already held was reverted rather than kept: {landing:?}"
    );
    assert!(
        !hub.asked
            .take()
            .iter()
            .any(|asked| asked.starts_with("revert")),
        "the train reverted a landing that held the tree it tested"
    );
    fs::remove_dir_all(&place).unwrap();
}

// What GitHub merged before a refusal is still held to the tree the
// candidate had there, as a whole landing is to the candidate's.
#[test]
fn a_merge_before_a_refusal_onto_a_main_that_moved_is_reverted() {
    let landed = landed_by("github-partly-moved", |place, origin| Fake {
        refuses: Some(2),
        moved_before: Some(1),
        ..Fake::of(place, origin)
    });
    assert!(
        matches!(
            &landed.landing,
            Landing::Reverted { reverted, kept, .. }
                if *reverted == [(landed.one.clone(), 101)] && kept.is_empty()
        ),
        "a merge onto a main the train didn't test stayed when the next was refused: {:?}",
        landed.landing
    );
    assert_eq!(
        landed.holds,
        ["elsewhere"],
        "main after the revert doesn't hold just the other change"
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
    assert!(
        landed.unsigned.is_empty(),
        "main gained commits GitHub didn't make, which a follower trusting it refuses: {:?}",
        landed.unsigned
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
        ["merge 1", "merge 2", "merge 2", "merge 2"],
        "the refused merge wasn't asked again until GitHub took it"
    );
}

#[test]
fn a_tree_github_once_couldnt_give_is_asked_again() {
    let landed = landed_by("github-tree-again", |place, origin| Fake {
        tree_fails: Cell::new(1),
        ..Fake::of(place, origin)
    });
    assert_eq!(
        landed.landing,
        Landing::Landed(landed.main.clone()),
        "one failed read of GitHub's tree reverted a right landing"
    );
}

#[test]
fn a_tree_github_cant_give_reverts_the_merges() {
    let landed = landed_by("github-treeless", |place, origin| Fake {
        tree_fails: Cell::new(u32::MAX),
        ..Fake::of(place, origin)
    });
    assert!(
        matches!(
            &landed.landing,
            Landing::Reverted { reverted, kept, why }
                if *reverted == [(landed.two.clone(), 102), (landed.one.clone(), 101)]
                    && kept.is_empty()
                    && why.contains("couldn't say what its merge")
        ),
        "merges whose tree GitHub couldn't give landed unjudged: {:?}",
        landed.landing
    );
}

#[test]
fn a_second_revert_github_wont_merge_keeps_just_the_merge_under_it() {
    let landed = landed_by("github-kept-under", |place, origin| Fake {
        moved_before: Some(2),
        revert_fails: Some(1),
        ..Fake::of(place, origin)
    });
    assert!(
        matches!(
            &landed.landing,
            Landing::Reverted { reverted, kept, why }
                if *reverted == [(landed.two.clone(), 102)]
                    && *kept == [landed.one.clone()]
                    && why.contains("#101")
        ),
        "the merge under a failed revert, and only it, wasn't kept: {:?}",
        landed.landing
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
    // Each failed read asks for the merge again, which merges a pull
    // request once at most, and nothing is reverted under it.
    assert_eq!(
        landed.asked,
        [vec!["merge 1"], vec!["merge 2"; MERGE_TRIES]].concat(),
        "the train didn't ask again for a merge GitHub couldn't say it made, or reverted under it"
    );
}

#[test]
fn a_merge_github_finishes_after_its_server_errors_counts_as_made() {
    // Read unmerged three times past the asks a merge gets, as GitHub
    // read #6398 merged minutes after the train turned it away.
    let landed = landed_by("github-late", |place, origin| Fake {
        merges_late: Cell::new(Some((2, MERGE_TRIES + 3))),
        ..Fake::of(place, origin)
    });
    assert_eq!(
        landed.landing,
        Landing::Landed(landed.main.clone()),
        "a merge GitHub finished after answering with server errors didn't land"
    );
    assert_eq!(
        landed.asked,
        [vec!["merge 1"], vec!["merge 2"; MERGE_TRIES]].concat(),
        "the train asked again for a merge GitHub was still finishing"
    );
}

#[test]
fn a_merge_github_never_finishes_after_its_server_errors_is_refused() {
    let landed = landed_by("github-never", |place, origin| Fake {
        merges_late: Cell::new(Some((2, usize::MAX))),
        ..Fake::of(place, origin)
    });
    assert!(
        matches!(
            &landed.landing,
            Landing::Partly { kept, refused, why, .. }
                if *kept == [landed.one.clone()]
                    && *refused == landed.two
                    && why.contains("HTTP 502")
        ),
        "a merge GitHub never made after its server errors didn't leave the train: {:?}",
        landed.landing
    );
    assert_eq!(
        landed.reads,
        MERGE_TRIES + SETTLE_ASKS,
        "the train didn't wait out a merge GitHub answered only with server errors"
    );
}

#[test]
fn a_merge_github_refuses_isnt_waited_out() {
    let landed = landed_by("github-stated", |place, origin| Fake {
        refuses: Some(2),
        ..Fake::of(place, origin)
    });
    assert!(
        matches!(&landed.landing, Landing::Partly { refused, .. } if *refused == landed.two),
        "a merge GitHub refused didn't leave the train: {:?}",
        landed.landing
    );
    assert_eq!(
        landed.reads, MERGE_TRIES,
        "the train waited five minutes on a merge GitHub said it refused"
    );
}

// GitHub may refuse a merge it is still finishing, so a server error on
// one try leaves it in doubt though every later try reads refused.
#[test]
fn a_merge_refused_after_a_server_error_is_waited_out() {
    let landed = landed_by("github-error-then-refused", |place, origin| Fake {
        merges_late: Cell::new(Some((2, MERGE_TRIES + 3))),
        refuses_after_error: true,
        ..Fake::of(place, origin)
    });
    assert_eq!(
        landed.landing,
        Landing::Landed(landed.main.clone()),
        "a merge GitHub finished after a server error and refusals didn't land"
    );
}

#[test]
fn a_merge_refused_after_a_server_error_that_never_lands_says_both() {
    // The 502 put the merge in doubt, and the refusals after it are what
    // its owner can act on, so the rider falls saying both.
    let landed = landed_by("github-error-then-refused-for-good", |place, origin| Fake {
        merges_late: Cell::new(Some((2, usize::MAX))),
        refuses_after_error: true,
        ..Fake::of(place, origin)
    });
    assert!(
        matches!(
            &landed.landing,
            Landing::Partly { refused, why, .. }
                if *refused == landed.two
                    && why.contains("(HTTP 405)")
                    && why.contains("(HTTP 502)")
        ),
        "a rider refused after a server error didn't fall saying both answers: {:?}",
        landed.landing
    );
}

#[test]
fn a_read_failing_after_a_stated_refusal_stops_the_train() {
    // Nothing put the merge in doubt, but GitHub can't say after whether
    // it merged, so main may hold it.
    let landed = landed_by("github-refused-then-cant-say", |place, origin| Fake {
        refuses: Some(2),
        read_fails: Cell::new(Some((1, 3))),
        ..Fake::of(place, origin)
    });
    assert!(
        matches!(
            &landed.landing,
            Landing::Reverted { reverted, why, .. }
                if reverted.is_empty() && why.contains("#2")
        ),
        "a refused merge GitHub couldn't read after didn't stop the train: {:?}",
        landed.landing
    );
}

#[test]
fn a_read_failing_while_a_merge_is_waited_out_is_asked_past() {
    let landed = landed_by("github-read-fails", |place, origin| Fake {
        merges_late: Cell::new(Some((2, MERGE_TRIES + 3))),
        read_fails: Cell::new(Some((MERGE_TRIES + 1, 3))),
        ..Fake::of(place, origin)
    });
    assert_eq!(
        landed.landing,
        Landing::Landed(landed.main.clone()),
        "one failed read while the train waited turned a late merge away"
    );
}

#[test]
fn a_merge_read_unmerged_last_after_failed_reads_is_refused() {
    // The wait's last read decides: reads failing early in it don't make
    // the merge unknown once a later one reads it unmerged.
    let landed = landed_by("github-fails-then-unmerged", |place, origin| Fake {
        merges_late: Cell::new(Some((2, usize::MAX))),
        read_fails: Cell::new(Some((MERGE_TRIES + 1, 3))),
        ..Fake::of(place, origin)
    });
    assert!(
        matches!(
            &landed.landing,
            Landing::Partly { kept, refused, .. }
                if *kept == [landed.one.clone()] && *refused == landed.two
        ),
        "a merge read unmerged after failed reads didn't leave the train: {:?}",
        landed.landing
    );
}

#[test]
fn a_read_failing_on_a_merge_in_doubt_before_the_wait_is_waited_out() {
    // The read after the first try fails all three times it is asked, so
    // the train used to stop at once on a merge GitHub went on to finish.
    let landed = landed_by("github-early-read-fails", |place, origin| Fake {
        merges_late: Cell::new(Some((2, MERGE_TRIES + 3))),
        read_fails: Cell::new(Some((1, 3))),
        ..Fake::of(place, origin)
    });
    assert_eq!(
        landed.landing,
        Landing::Landed(landed.main.clone()),
        "a failed read on a merge in doubt stopped the train before waiting it out"
    );
}

#[test]
fn a_merge_github_cant_say_after_waiting_stops_the_train() {
    let landed = landed_by("github-never-says", |place, origin| Fake {
        merges_late: Cell::new(Some((2, usize::MAX))),
        read_fails: Cell::new(Some((MERGE_TRIES + 1, usize::MAX))),
        ..Fake::of(place, origin)
    });
    assert!(
        matches!(
            &landed.landing,
            Landing::Reverted { reverted, why, .. }
                if reverted.is_empty() && why.contains("#2")
        ),
        "a merge GitHub couldn't say it made after the wait didn't stop the train: {:?}",
        landed.landing
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

#[test]
fn gh_s_own_answers_split_refusals_from_doubt() {
    // As train.log quoted gh on 25 November, through `gh` here.
    for refused in [
        "gh api: gh: Pull Request is still a draft (HTTP 405)",
        "gh api: gh: Pull Request has merge conflicts (HTTP 405)",
    ] {
        assert!(states_refusal(refused), "{refused} read as no refusal");
    }
    for doubt in [
        "gh api: gh: Server Error (HTTP 502)",
        "gh api: ",
        "running gh: No such file or directory (os error 2)",
        "gh api: gh: You have exceeded a secondary rate limit (HTTP 403)",
        "gh api: gh: Too Many Requests (HTTP 429)",
    ] {
        assert!(!states_refusal(doubt), "{doubt} read as a refusal");
    }
}
