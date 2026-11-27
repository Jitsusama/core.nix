//! What a filling disk's reach skips after one was cut short: the files
//! it already judged, kept across the spell, and when that set lets go.

use disk_guard::band::Band;
use disk_guard::decide::Decision;
use disk_guard::pass::{End, judged_all, left_to_judge, skipped_after};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn paths(names: &[&str]) -> BTreeSet<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

/// An offer's path, for `left_to_judge`, named since a closure can't say the borrow it returns.
fn path(offer: &PathBuf) -> &Path {
    offer
}

const FILLING: Decision = Decision::Reach(Band::Filling);

#[test]
fn a_cut_reach_keeps_the_files_it_tried_and_never_a_named_offer() {
    // The oldest garbage batch is named garbage-0 every round, whatever
    // paths it holds, so keeping its name would skip garbage nobody judged.
    let tried = paths(&["/d/cache", "/nix/store#garbage-0", "/d#podman-prune"]);
    let first = skipped_after(None, FILLING, &End::Unfinished, &tried);
    assert_eq!(first, Some(paths(&["/d/cache"])), "only the file is kept");
    let second = skipped_after(first, FILLING, &End::Unfinished, &paths(&["/d/logs"]));
    assert_eq!(
        second,
        Some(paths(&["/d/cache", "/d/logs"])),
        "a second cut adds to the first"
    );
}

#[test]
fn a_reach_that_waits_stalls_or_stops_and_a_collection_keep_what_was_skipped() {
    let skipped = Some(paths(&["/d/cache"]));
    let tried = paths(&["/d/logs"]);
    for end in [End::Waiting, End::Stalled, End::Stopped] {
        assert_eq!(
            skipped_after(skipped.clone(), FILLING, &end, &tried),
            skipped,
            "a filling reach ending {end:?} keeps it as it was"
        );
    }
    // Nix's garbage is collected in a reach's rest, which the spell
    // outlasts.
    assert_eq!(
        skipped_after(skipped.clone(), Decision::Collect, &End::Reached, &tried),
        skipped,
        "a collection keeps it"
    );
}

#[test]
fn a_reach_that_gets_through_runs_at_a_worse_band_or_upkeep_skips_nothing_more() {
    let skipped = Some(paths(&["/d/cache"]));
    let tried = paths(&["/d/logs"]);
    for end in [End::Reached, End::Exhausted, End::Failed("no".into())] {
        assert_eq!(
            skipped_after(skipped.clone(), FILLING, &end, &tried),
            None,
            "a filling reach ending {end:?} lets it go"
        );
    }
    let nearly = Decision::Reach(Band::NearlyFull);
    assert_eq!(
        skipped_after(skipped.clone(), nearly, &End::Unfinished, &tried),
        None,
        "no length cuts a nearly full reach, so it keeps nothing"
    );
    assert_eq!(
        skipped_after(skipped, Decision::Upkeep, &End::Unfinished, &tried),
        None,
        "upkeep comes only to a healthy disk, past the spell"
    );
}

#[test]
fn a_reach_skips_what_was_judged_unless_that_leaves_it_nothing() {
    let offers: Vec<PathBuf> = ["/d/cache", "/d/logs"].map(PathBuf::from).to_vec();
    assert_eq!(
        left_to_judge(&offers, Some(&paths(&["/d/cache"])), path),
        vec![PathBuf::from("/d/logs")],
        "what was judged is skipped"
    );
    assert_eq!(
        left_to_judge(&offers, Some(&paths(&["/d/cache", "/d/logs"])), path),
        offers,
        "given nothing, the reach would call the disk exhausted over offers it never saw"
    );
    assert_eq!(
        left_to_judge(&offers, None, path),
        offers,
        "nothing skipped"
    );
}

#[test]
fn a_reach_left_only_named_offers_is_given_everything() {
    // A garbage batch or a command is never skipped, so on the store's
    // disk it was always left, and a reach shown only it ended exhausted,
    // telling Joel to delete by hand over files it never judged again.
    let offers: Vec<PathBuf> = ["/d/cache", "/nix/store#garbage-0", "/d#podman-prune"]
        .map(PathBuf::from)
        .to_vec();
    assert_eq!(
        left_to_judge(&offers, Some(&paths(&["/d/cache"])), path),
        offers,
        "a reach left only named offers was kept from the files"
    );
    let more: Vec<PathBuf> = ["/d/cache", "/d/logs", "/nix/store#garbage-0"]
        .map(PathBuf::from)
        .to_vec();
    assert_eq!(
        left_to_judge(&more, Some(&paths(&["/d/cache"])), path),
        ["/d/logs", "/nix/store#garbage-0"]
            .map(PathBuf::from)
            .to_vec(),
        "a file left to judge keeps the named offers beside it"
    );
}

#[test]
fn a_reach_that_had_judged_everything_starts_afresh() {
    // Kept, its skip set would cover the same offers next time, and every
    // reach after would judge the same two hours.
    let offers: Vec<PathBuf> = ["/d/cache", "/d/logs"].map(PathBuf::from).to_vec();
    assert!(
        judged_all(
            &offers,
            Some(&paths(&["/d/cache", "/d/logs", "/d/gone"])),
            path
        ),
        "every offer was judged"
    );
    assert!(
        !judged_all(&offers, Some(&paths(&["/d/cache"])), path),
        "one is left to judge"
    );
    assert!(!judged_all(&offers, None, path), "nothing was skipped");
    assert!(
        !judged_all(&[], Some(&paths(&["/d/cache"])), path),
        "a reach offered nothing has nothing to start afresh on"
    );
}

#[test]
fn a_reach_that_had_judged_every_file_starts_afresh_beside_named_offers() {
    let offers: Vec<PathBuf> = ["/d/cache", "/nix/store#garbage-0"]
        .map(PathBuf::from)
        .to_vec();
    assert!(
        judged_all(&offers, Some(&paths(&["/d/cache"])), path),
        "a named offer, never skipped, kept a reach from starting afresh"
    );
    let named: Vec<PathBuf> = ["/nix/store#garbage-0"].map(PathBuf::from).to_vec();
    assert!(
        !judged_all(&named, Some(&paths(&["/d/cache"])), path),
        "a reach offered no file has no file to start afresh on"
    );
}
