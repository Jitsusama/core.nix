//! The hour a survey gives a budgeted build's replaced builds, judged by
//! the opens this guard heard. A Mac hears nothing, so none goes by the
//! hour there, and these run on Linux alone.

use crate::discover::file;
use crate::replaced::{artifact, fingerprinted, runnable};
use crate::survey::{find, now, offers_using, written_ago};
use crate::survey_trees::{budgeted, heard};
use disk_guard::plan::{self, Group};
use disk_guard::replaced::{Uses, watched_dirs};
use disk_guard::survey::Offer;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Dates everything in `fingerprint` `hours` ago.
fn aged_fingerprint(fingerprint: &Path, hours: f64) {
    for entry in fs::read_dir(fingerprint).unwrap() {
        written_ago(&entry.unwrap().path(), hours * 3600.0);
    }
}

/// A build of `package`'s integration test in `deps`, written `hours` ago,
/// with its dep-info and the fingerprint naming its unit, built as
/// `variant` changes the plain build.
fn test_binary(
    deps: &Path,
    package: &str,
    digest: &str,
    hours: f64,
    variant: &[(&str, serde_json::Value)],
) -> PathBuf {
    let unit = "test-integration-test-integration";
    let binary = runnable(deps, &format!("integration-{digest}"), 0);
    file(&binary, &"x".repeat(4096));
    let info = artifact(deps, &format!("integration-{digest}.d"), 0);
    for path in [&binary, &info] {
        written_ago(path, hours * 3600.0);
    }
    aged_fingerprint(&fingerprinted(deps, package, digest, unit, variant), hours);
    binary
}

#[test]
fn a_budgeted_builds_replaced_artifacts_go_an_hour_unused_and_one_opened_within_it_stays() {
    // On 18 November a gate's build held 28 GiB of test binaries a newer
    // build of the same test had replaced, nearly all one to six hours
    // old, so the day a replaced build waited took none of them.
    let (_home, made, config) = budgeted(&[("app", 2.0)], 100);
    let target = made[0].join("target");
    let deps = target.join("debug/deps");
    // budgeted() builds the newer library, and an older one of the same
    // variant sits beside it.
    aged_fingerprint(
        &fingerprinted(&deps, "app", "0123456789abcdef", "lib-app", &[]),
        2.0,
    );
    let library = deps.join("libapp-5555555555555555.rlib");
    file(&library, "old");
    written_ago(&library, 3.0 * 3600.0);
    aged_fingerprint(
        &fingerprinted(&deps, "app", "5555555555555555", "lib-app", &[]),
        3.0,
    );
    let unused = test_binary(&deps, "server", "1111111111111111", 3.0, &[]);
    let newest = test_binary(&deps, "server", "2222222222222222", 2.0, &[]);
    // The same test built with a feature, never opened: the newest of its
    // own way of being built, so nothing has replaced it within the hour.
    let featured = serde_json::json!("[\"x\"]");
    let other_way = test_binary(
        &deps,
        "server",
        "6666666666666666",
        3.0,
        &[("features", featured)],
    );
    // Another package's test of the same name, the older opened since.
    let opened = test_binary(&deps, "client", "3333333333333333", 3.0, &[]);
    test_binary(&deps, "client", "4444444444444444", 2.0, &[]);
    let mut uses = heard(&made, 3.0);
    // One opened a minute inside the hour stays; one a minute past it goes.
    // Written out rather than read from the rule, so a changed hour fails.
    uses.opened(&opened, now() - 3540.0);
    uses.opened(&unused, now() - 3660.0);
    let found = offers_using(&config, &[], &|_| false, &BTreeMap::new(), &uses);
    let spent = |found: &[Offer], path: &Path| {
        let offer = find(found, path);
        offer.candidate.spent && plan::windows(&offer.candidate).0 == Group::Waste
    };
    // A library's live builds can differ only in how a dependency was
    // built, so it waits its day.
    assert!(
        !spent(&found, &deps.join("app-5555555555555555")),
        "a replaced library went within its day"
    );
    assert!(
        spent(&found, &unused),
        "a replaced test binary last opened a minute past the hour stayed"
    );
    assert!(
        !spent(&found, &opened),
        "a replaced test binary opened a minute inside the hour went"
    );
    assert!(
        !spent(&found, &other_way),
        "a test binary went by the hour though no newer build of its features replaced it"
    );
    assert!(
        found.iter().all(|offer| offer.candidate.path != newest),
        "the newest build of a test was offered"
    );
    // Watched three hours but heard only half an hour, as after the guard
    // restarts, or watched only by an earlier guard, an open in the hour
    // may have gone unheard.
    let mut earlier = Uses::default();
    let mut restarted = Uses::default();
    for dir in watched_dirs(&target) {
        earlier.watching(&dir, now() - 3.0 * 3600.0);
        restarted.watching(&dir, now() - 3.0 * 3600.0);
        let handle = disk_guard::opens::handle(&dir).unwrap();
        restarted.marked(&dir, now() - 1800.0, handle);
    }
    for (why, uses) in [("heard half an hour", restarted), ("unheard", earlier)] {
        let found = offers_using(&config, &[], &|_| false, &BTreeMap::new(), &uses);
        assert!(
            !spent(&found, &unused),
            "a replaced test binary went within its day, {why}"
        );
    }
    // Outside a budget, nobody asked for builds to be held small.
    let unbudgeted = crate::survey::config(&format!(
        r#""cargo": {{ "kind": "cargo", "roots": ["{}"] }}"#,
        made[0].parent().unwrap().display()
    ));
    let found = offers_using(&unbudgeted, &[], &|_| false, &BTreeMap::new(), &uses);
    assert!(
        !spent(&found, &unused),
        "a replaced test binary went within its day outside a budget"
    );
}
