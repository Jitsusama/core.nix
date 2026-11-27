//! Which merge a train's finding names as its check's owner, read in git.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use super::super::folder;
use super::super::tests::git_in as git;
use super::Red;
use super::owner::{MOST_OWNER_MERGES, Unowned, owner_in};

/// A merge onto main of one empty commit on `side`, said by `subject`,
/// in `checkout`; its commit.
fn merged(checkout: &Path, side: &str, subject: &str) -> String {
    git(checkout, &["checkout", "-q", "-b", side]).unwrap();
    git(checkout, &["commit", "-q", "--allow-empty", "-m", side]).unwrap();
    git(checkout, &["checkout", "-q", "main"]).unwrap();
    git(checkout, &["merge", "-q", "--no-ff", "-m", subject, side]).unwrap();
    git(checkout, &["rev-parse", "HEAD"]).unwrap()
}

/// A net under `name` in the temp folder that last passed a commit, then
/// merged #12, which moved the check `a`: its state root, its checkout,
/// what it passed and the merge.
fn net_past_twelve(name: &str) -> (PathBuf, PathBuf, String, String) {
    let root = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let checkout = folder(&root).join("checkout");
    fs::create_dir_all(&checkout).unwrap();
    git(&checkout, &["init", "-q", "-b", "main"]).unwrap();
    git(
        &checkout,
        &["commit", "-q", "--allow-empty", "-m", "passed"],
    )
    .unwrap();
    let passed = git(&checkout, &["rev-parse", "HEAD"]).unwrap();
    let twelve = merged(
        &checkout,
        "moves-a",
        "Merge pull request #12 from someone/moves-a",
    );
    // Each commit's derivation of `a`, as the watch keeps them, so nothing
    // is evaluated.
    fs::create_dir_all(root.join("drv")).unwrap();
    keep_drv(&root, &passed, "was");
    keep_drv(&root, &twelve, "now");
    fs::write(
        root.join("state.json"),
        json!({ "passed": passed }).to_string(),
    )
    .unwrap();
    (root, checkout, passed, twelve)
}

/// Keep `a`'s derivation at `commit`, named by `which`, under the net's
/// `root`.
fn keep_drv(root: &Path, commit: &str, which: &str) {
    fs::write(
        root.join("drv").join(format!("{commit}-a")),
        format!("/nix/store/{which}-a.drv"),
    )
    .unwrap();
}

/// The owner of `a`, failed alone, of a red at `base` in `checkout`, by
/// the net under `root`, or why none is named.
fn owner_at(root: &Path, checkout: &Path, base: &str) -> Result<u64, Unowned> {
    let log = root.join("gate.log");
    let red = Red {
        checkout,
        base,
        candidate: &"c".repeat(40),
        log: &log,
    };
    owner_in(
        root,
        "https://github.com/someone/core.git",
        &red,
        "a",
        &["a".to_owned()],
    )
}

/// A red's owner is read in the train's checkout, which fetched the base,
/// since the watch's clone may not hold it yet: here there is none.
#[test]
fn a_reds_owner_is_read_where_the_train_fetched_its_base() {
    let (root, checkout, _, twelve) = net_past_twelve("merge-net-owner");
    assert_eq!(
        owner_at(&root, &checkout, &twelve),
        Ok(12),
        "the merge that moved the check wasn't named its owner"
    );
    fs::remove_dir_all(&root).unwrap();
}

/// Each merge looked at may evaluate the flake twice, inside the ride, so
/// a net that passed nothing for many merges looks back no further than
/// `MOST_OWNER_MERGES`, and names no owner it would find past them.
#[test]
fn an_owner_is_looked_for_among_the_newest_merges_alone() {
    let (root, checkout, _, _) = net_past_twelve("merge-net-owner-far");
    let mut base = String::new();
    for later in 0..MOST_OWNER_MERGES {
        let side = format!("later-{later}");
        base = merged(
            &checkout,
            &side,
            &format!("Merge pull request #{} from x/{side}", 20 + later),
        );
        keep_drv(&root, &base, "now");
    }
    assert_eq!(
        owner_at(&root, &checkout, &base),
        Err(Unowned::NoMerge {
            looked: MOST_OWNER_MERGES,
            since: MOST_OWNER_MERGES + 1,
        }),
        "an owner was looked for past the newest merges"
    );
    fs::remove_dir_all(&root).unwrap();
}

/// A finding names no owner for one of three reasons, and the train's log
/// says which: a net that has passed nothing, and a base git can't count
/// from, read no merges at all, so they aren't said as merges read in vain.
#[test]
fn why_no_owner_is_named_is_told_apart() {
    let (root, checkout, _, twelve) = net_past_twelve("merge-net-owner-unread");
    assert_eq!(
        owner_at(&root, &checkout, &"0".repeat(40)),
        Err(Unowned::Uncounted),
        "a base the checkout lacks wasn't said uncounted"
    );
    fs::write(root.join("state.json"), json!({}).to_string()).unwrap();
    assert_eq!(
        owner_at(&root, &checkout, &twelve),
        Err(Unowned::NothingPassed),
        "a net that passed nothing wasn't said to have passed nothing"
    );
    let said = [
        Unowned::NothingPassed,
        Unowned::Uncounted,
        Unowned::NoMerge {
            looked: 1,
            since: 1,
        },
    ]
    .map(|unowned| unowned.said("a"));
    assert!(
        said[0].contains("passed no head") && said[1].contains("couldn't count"),
        "a range never read was said as read: {said:?}"
    );
    assert!(
        said[2].contains("no merge of the 1 since") && !said[0].contains("no merge"),
        "merges read in vain weren't told from none read: {said:?}"
    );
    fs::remove_dir_all(&root).unwrap();
}
