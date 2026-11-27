//! Which merge a train's finding names as its check's owner, read in git.

use std::fs;

use serde_json::json;

use super::super::folder;
use super::super::tests::git_in as git;
use super::{Red, owner_in};

/// A red's owner is read in the train's checkout, which fetched the base,
/// since the watch's clone may not hold it yet: here there is none.
#[test]
fn a_reds_owner_is_read_where_the_train_fetched_its_base() {
    let root = std::env::temp_dir().join(format!("merge-net-owner-{}", std::process::id()));
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
    git(&checkout, &["checkout", "-q", "-b", "side"]).unwrap();
    git(
        &checkout,
        &["commit", "-q", "--allow-empty", "-m", "moves a"],
    )
    .unwrap();
    git(&checkout, &["checkout", "-q", "main"]).unwrap();
    let subject = "Merge pull request #12 from someone/moves-a";
    git(
        &checkout,
        &["merge", "-q", "--no-ff", "-m", subject, "side"],
    )
    .unwrap();
    let base = git(&checkout, &["rev-parse", "HEAD"]).unwrap();
    // Each commit's derivation of `a`, as the watch keeps them, so nothing
    // is evaluated.
    let drv = root.join("drv");
    fs::create_dir_all(&drv).unwrap();
    fs::write(drv.join(format!("{passed}-a")), "/nix/store/was-a.drv").unwrap();
    fs::write(drv.join(format!("{base}-a")), "/nix/store/now-a.drv").unwrap();
    fs::write(
        root.join("state.json"),
        json!({ "passed": passed }).to_string(),
    )
    .unwrap();
    let log = root.join("gate.log");
    let red = Red {
        checkout: &checkout,
        base: &base,
        candidate: &"c".repeat(40),
        log: &log,
    };
    let failed = ["a".to_owned()];
    assert_eq!(
        owner_in(
            &root,
            "https://github.com/someone/core.git",
            &red,
            "a",
            &failed
        ),
        Some(12),
        "the merge that moved the check wasn't named its owner"
    );
    fs::remove_dir_all(&root).unwrap();
}
