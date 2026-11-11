use disk_guard::activity::Use;
use disk_guard::config::Config;
use disk_guard::holders::{
    Kind, LastMeasure, MOVED, Offered, Offers, Place, SPARED, due, holders, mark, measure, places,
};
use disk_guard::keeps::{Keep, Keeps};
use disk_guard::ledger::{Grown, Growth};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A home with a checkout, a worktree, a quest's workspace and a cache,
/// each with a file of `kib` KiB in it, and a config naming them as this
/// machine's sources do: the workspaces under a cargo root as well.
fn home(root: &Path) -> Config {
    let write = |path: &str, kib: usize| {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![7u8; kib * 1024]).unwrap();
    };
    write("src/app/main.rs", 8);
    write("trees/fix-1/target/debug/app", 64);
    write("trees/fix-1/.git", 1);
    write("cache/quests/QEST-20261026-ABCDEF/build.log", 32);
    write("cache/sccache/blob", 16);
    Config::parse(&format!(
        r#"{{ "version": 1, "scope": "user",
             "nix": {{ "bin": "{0}/no-nix/nix", "store": "{0}/no-nix/store" }},
             "sources": {{
               "src": {{ "kind": "cargo", "roots": ["{0}/src", "{0}/trees", "{0}/cache/quests"] }},
               "targets": {{ "kind": "cargo", "roots": ["{0}/cache"] }},
               "sccache": {{ "kind": "cache", "path": "{0}/cache/sccache" }},
               "workspaces": {{ "kind": "workspaces", "path": "{0}/cache/quests",
                                "quests": "{0}/quests" }} }} }}"#,
        root.display()
    ))
    .unwrap()
}

fn found(config: &Config) -> Vec<(String, Kind, String)> {
    let mut found: Vec<_> = places(config)
        .into_iter()
        .map(|place| {
            let path = place.path.to_string_lossy().into_owned();
            (path, place.owner.kind, place.owner.name)
        })
        .collect();
    found.sort_by(|one, other| one.0.cmp(&other.0));
    found
}

#[test]
fn each_place_is_the_folder_under_a_root_named_by_whose_it_is() {
    let dir = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    let config = home(&root);
    let at = |path: &str| root.join(path).to_string_lossy().into_owned();
    assert_eq!(
        found(&config),
        [
            (
                at("cache/quests/QEST-20261026-ABCDEF"),
                Kind::Quest,
                "QEST-20261026-ABCDEF".to_owned()
            ),
            (at("cache/sccache"), Kind::Cache, "sccache".to_owned()),
            (at("src/app"), Kind::Other, "app".to_owned()),
            (at("trees/fix-1"), Kind::Worktree, "fix-1".to_owned()),
        ],
        "the cache root's own quests folder holds the quests, so it isn't one"
    );
}

#[test]
fn a_place_is_sized_as_it_takes_the_disk() {
    let dir = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    let config = home(&root);
    let measured = measure(places(&config));
    let size = |name: &str| {
        measured
            .iter()
            .find(|(place, _)| place.owner.name == name)
            .map(|(_, bytes)| *bytes)
            .unwrap()
    };
    assert!(size("fix-1") >= 65 * 1024, "{measured:?}");
    assert!(size("QEST-20261026-ABCDEF") >= 32 * 1024, "{measured:?}");
    assert!(size("fix-1") > size("QEST-20261026-ABCDEF"));
}

fn place(path: &str, kind: Kind, name: &str) -> Place {
    Place {
        path: PathBuf::from(path),
        owner: disk_guard::holders::Owner {
            kind,
            name: name.to_owned(),
        },
    }
}

const GIB: u64 = 1 << 30;

#[test]
fn each_holder_says_whose_it_is_and_why_the_guard_may_clear_none_of_it() {
    let home = Path::new("/home/u");
    let measured = [
        (
            place("/home/u/trees/fix-1", Kind::Worktree, "fix-1"),
            40 * GIB,
        ),
        (
            place(
                "/home/u/quests/QEST-20261026-ABCDEF",
                Kind::Quest,
                "QEST-20261026-ABCDEF",
            ),
            30 * GIB,
        ),
        (
            place("/home/u/trees/fix-2", Kind::Worktree, "fix-2"),
            20 * GIB,
        ),
        (place("/home/u/src/app", Kind::Other, "app"), GIB),
        (place("/home/u/src/empty", Kind::Other, "empty"), 0),
    ];
    let offered = [offered("/home/u/trees/fix-2/target", 12 * GIB, true)];
    let kept = [(PathBuf::from("/home/u/trees/fix-1/target"), Use::Busy(4242))];
    let said = holders(
        &measured,
        Some(&Offers::new(offered.to_vec())),
        &kept,
        &Grown::new(),
        Some(home),
    );
    let says: Vec<_> = said.iter().map(|holder| holder.says.as_str()).collect();
    assert_eq!(
        says,
        [
            "The worktree fix-1 holds 40.0 GiB; a build is using it (process 4242).",
            "QEST-20261026-ABCDEF's workspace holds 30.0 GiB; its quest is still open, or \
             only just finished.",
            "The worktree fix-2 holds 20.0 GiB, of which 12.0 GiB can be cleared now.",
            "~/src/app holds 1.0 GiB; nothing in it is a build or cache the guard clears.",
        ],
        "biggest first, and an empty one says nothing"
    );
    assert!(!said[0].clearable);
    assert_eq!(
        said[0].why.as_deref(),
        Some("a build is using it (process 4242)")
    );
    assert!(said[2].clearable && said[2].why.is_none());
    let json = serde_json::to_value(&said[1]).unwrap();
    assert_eq!(json["owner"]["kind"], "quest");
    assert_eq!(json["owner"]["name"], "QEST-20261026-ABCDEF");
    assert_eq!(json["bytes"], 30 * GIB);
}

// Measured before the guard's first survey, which may be six hours off
// on a disk well under its budget, a holder said nothing in it was a
// build or cache the guard clears, of a tree full of them.
#[test]
fn a_holder_measured_before_any_survey_says_the_guard_hasnt_looked_yet() {
    let measured = [(
        place("/home/u/trees/fix-1", Kind::Worktree, "fix-1"),
        40 * GIB,
    )];
    let said = holders(&measured, None, &[], &Grown::new(), None);
    assert!(!said[0].clearable);
    assert_eq!(
        said[0].says,
        "The worktree fix-1 holds 40.0 GiB; the guard hasn't yet looked at what of it \
         it could clear."
    );
}

#[test]
fn only_the_ten_biggest_are_listed() {
    let measured: Vec<_> = (1..=12)
        .map(|n| (place(&format!("/x/{n}"), Kind::Other, "x"), n * GIB))
        .collect();
    let said = holders(
        &measured,
        Some(&Offers::default()),
        &[],
        &Grown::new(),
        None,
    );
    assert_eq!(said.len(), 10);
    assert_eq!(said[0].bytes, 12 * GIB);
    assert_eq!(said[9].bytes, 3 * GIB);
}

#[test]
fn a_kept_holder_says_his_reason_and_says_the_guard_s_own_once_let_go() {
    let home = Path::new("/home/u");
    let measured = [(
        place("/home/u/trees/fix-2", Kind::Worktree, "fix-2"),
        20 * GIB,
    )];
    let offered = [offered("/home/u/trees/fix-2/target", 12 * GIB, true)];
    let offered = Offers::new(offered.to_vec());
    let mut said = holders(&measured, Some(&offered), &[], &Grown::new(), Some(home));
    let judged = said.clone();
    assert_eq!(said[0].kept, None);
    assert_eq!(said[0].clears, 12 * GIB);
    assert_eq!(said[0].name, "fix-2");
    let mut keeps = Keeps::new();
    keeps.insert(
        PathBuf::from("/home/u/trees"),
        Keep {
            reason: "the release builds".to_owned(),
            at: 5.0,
            declared: false,
        },
    );
    mark(&mut said, &keeps, &[], Some(home));
    assert_eq!(
        said[0].says,
        "The worktree fix-2 holds 20.0 GiB; you keep it: the release builds."
    );
    assert_eq!(
        said[0].kept.as_ref().map(|keep| keep.reason.as_str()),
        Some("the release builds")
    );
    assert!(
        said[0].clearable,
        "what the guard judged of its contents stands beside the keep"
    );
    mark(&mut said, &Keeps::new(), &[], Some(home));
    assert_eq!(said, judged, "marking again with no keep undoes it");
}

fn offered(path: &str, size: u64, now: bool) -> Offered {
    Offered {
        path: PathBuf::from(path),
        size,
        now,
    }
}

#[test]
fn offers_in_any_order_count_only_toward_the_place_they_lie_beneath() {
    let measured = [
        (
            place("/home/u/trees/fix-1", Kind::Worktree, "fix-1"),
            40 * GIB,
        ),
        (
            place("/home/u/trees/fix-10", Kind::Worktree, "fix-10"),
            30 * GIB,
        ),
        (
            place("/home/u/trees/fix-2", Kind::Worktree, "fix-2"),
            20 * GIB,
        ),
    ];
    let offered = Offers::new(vec![
        offered("/home/u/trees/fix-2/target", 5 * GIB, true),
        offered("/home/u/trees/fix-10/target", 7 * GIB, true),
        offered("/home/u/trees/fix-1/target", 3 * GIB, true),
        offered("/home/u/trees/fix-1-old", GIB, true),
        offered("/home/u/trees/fix-1/scratch", 2 * GIB, true),
    ]);
    let said = holders(&measured, Some(&offered), &[], &Grown::new(), None);
    let clears: Vec<_> = said
        .iter()
        .map(|holder| (holder.path.to_str().unwrap(), holder.clears / GIB))
        .collect();
    assert_eq!(
        clears,
        [
            ("/home/u/trees/fix-1", 5),
            ("/home/u/trees/fix-10", 7),
            ("/home/u/trees/fix-2", 5),
        ]
    );
}

#[test]
fn a_holder_says_how_much_a_clear_takes_now_and_whether_the_rest_waits() {
    let home = Path::new("/home/u");
    let measured = [
        (
            place("/home/u/trees/fix-1", Kind::Worktree, "fix-1"),
            20 * GIB,
        ),
        (
            place("/home/u/trees/fix-2", Kind::Worktree, "fix-2"),
            10 * GIB,
        ),
    ];
    let offered = [
        offered("/home/u/trees/fix-1/target", 6 * GIB, true),
        offered("/home/u/trees/fix-1/scratch", 3 * GIB, false),
        offered("/home/u/trees/fix-2/scratch", 4 * GIB, false),
    ];
    let offered = Offers::new(offered.to_vec());
    let said = holders(&measured, Some(&offered), &[], &Grown::new(), Some(home));
    assert_eq!(
        said[0].clears,
        6 * GIB,
        "only what a clear takes now counts"
    );
    assert!(said[1].clearable, "the guard still clears it in time");
    assert_eq!(said[1].clears, 0);
    assert_eq!(
        said[1].says,
        "The worktree fix-2 holds 10.0 GiB; the guard clears some of it once it has sat idle \
         long enough."
    );
}

#[test]
fn a_holder_being_cleared_says_so_until_its_clear_ends() {
    let measured = [(
        place("/home/u/trees/fix-2", Kind::Worktree, "fix-2"),
        20 * GIB,
    )];
    let offered = [offered("/home/u/trees/fix-2/target", 12 * GIB, true)];
    let offered = Offers::new(offered.to_vec());
    let mut said = holders(&measured, Some(&offered), &[], &Grown::new(), None);
    assert!(!said[0].clearing);
    let asked = [PathBuf::from("/home/u/trees/fix-2/target")];
    mark(&mut said, &Keeps::new(), &asked, None);
    assert!(said[0].clearing, "a clear of part of it is a clear of it");
    mark(&mut said, &Keeps::new(), &[], None);
    assert!(!said[0].clearing);
}

#[test]
fn a_holder_says_how_much_it_grew_in_a_day_from_a_gibibyte() {
    let home = Path::new("/home/u");
    let measured = [
        (place("/home/u/src/app", Kind::Other, "app"), 10 * GIB),
        (place("/home/u/src/lib", Kind::Other, "lib"), 4 * GIB),
    ];
    let mut grew = Grown::new();
    let over = 20.0 * 3600.0;
    grew.insert(
        PathBuf::from("/home/u/src/app"),
        Growth {
            bytes: 6 * GIB,
            over,
        },
    );
    grew.insert(
        PathBuf::from("/home/u/src/lib"),
        Growth {
            bytes: GIB / 2,
            over,
        },
    );
    let said = holders(&measured, Some(&Offers::default()), &[], &grew, Some(home));
    assert_eq!(
        said[0].says,
        "~/src/app holds 10.0 GiB; nothing in it is a build or cache the guard clears. It \
         grew 6.0 GiB in 20.0 h."
    );
    assert_eq!(
        said[0].grew,
        Some(Growth {
            bytes: 6 * GIB,
            over
        })
    );
    assert_eq!(
        said[1].says,
        "~/src/lib holds 4.0 GiB; nothing in it is a build or cache the guard clears.",
        "growing less than a gibibyte a day isn't news"
    );
}

#[test]
fn a_holder_growing_fast_is_listed_beyond_the_ten_biggest() {
    let measured: Vec<_> = (10..=21)
        .map(|n| (place(&format!("/x/{n}"), Kind::Other, "x"), n * GIB))
        .collect();
    let mut grew = Grown::new();
    let over = 3600.0;
    grew.insert(
        PathBuf::from("/x/10"),
        Growth {
            bytes: 5 * GIB,
            over,
        },
    );
    grew.insert(
        PathBuf::from("/x/11"),
        Growth {
            bytes: 4 * GIB,
            over,
        },
    );
    let said = holders(&measured, Some(&Offers::default()), &[], &grew, None);
    let listed: Vec<u64> = said.iter().map(|holder| holder.bytes / GIB).collect();
    assert_eq!(listed, [21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 10]);
}

#[test]
fn each_folder_in_a_holders_root_is_a_holder_unless_a_source_says_more() {
    let dir = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    for folder in [
        "share/versions/1",
        "share/containers/storage",
        "share/Trash/files",
    ] {
        fs::create_dir_all(root.join(folder)).unwrap();
    }
    fs::write(root.join("share/loose"), "a file is no holder").unwrap();
    let config = Config::parse(&format!(
        r#"{{ "version": 1, "scope": "user",
             "nix": {{ "bin": "{0}/no-nix/nix", "store": "{0}/no-nix/store" }},
             "holders": {{ "roots": ["{0}/share"] }},
             "sources": {{
               "trash": {{ "kind": "trash", "path": "{0}/share/Trash" }},
               "podman": {{ "kind": "podman", "command": "/bin/podman",
                            "path": "{0}/share/containers/storage" }} }} }}"#,
        root.display()
    ))
    .unwrap();
    let at = |path: &str| root.join(path).to_string_lossy().into_owned();
    assert_eq!(
        found(&config),
        [
            (at("share/Trash"), Kind::Other, "trash".to_owned()),
            (
                at("share/containers/storage"),
                Kind::Containers,
                "podman".to_owned()
            ),
            (at("share/versions"), Kind::Other, "versions".to_owned()),
        ],
        "the containers folder holds podman's storage, so it isn't one"
    );
}

const HOUR: f64 = 3600.0;

/// Bytes used on each disk, by device.
fn used(disks: &[(u64, u64)]) -> BTreeMap<u64, u64> {
    disks.iter().copied().collect()
}

/// A measure taken at hour zero, with 500 GiB used on disk 1.
fn last() -> LastMeasure {
    LastMeasure {
        at: 0.0,
        used: used(&[(1, 500 * GIB)]),
        took: 60.0,
    }
}

#[test]
fn a_first_measure_is_due_at_once() {
    assert!(due(None, 0.0, &used(&[(1, 500 * GIB)])));
}

#[test]
fn no_measure_is_taken_within_half_an_hour_of_the_last_however_the_disk_moves() {
    assert!(!due(Some(&last()), 0.4 * HOUR, &used(&[(1, 600 * GIB)])));
}

#[test]
fn after_half_an_hour_a_disk_moved_by_a_gibibyte_is_measured_again() {
    assert!(due(
        Some(&last()),
        0.5 * HOUR,
        &used(&[(1, 500 * GIB + MOVED)])
    ));
    assert!(due(
        Some(&last()),
        0.5 * HOUR,
        &used(&[(1, 500 * GIB - MOVED)])
    ));
}

#[test]
fn a_disk_that_held_still_is_measured_again_only_after_six_hours() {
    let still = used(&[(1, 500 * GIB + MOVED - 1)]);
    assert!(!due(Some(&last()), 5.9 * HOUR, &still));
    assert!(due(Some(&last()), 6.0 * HOUR, &still));
}

/// A walk of every holder took 6 to 12 minutes of a core at a time on 9
/// November, every half hour on a filling disk, so the guard spent a fifth
/// of a core sizing what it couldn't clear.
#[test]
fn a_measure_that_took_long_waits_that_many_times_its_length_before_the_next() {
    let slow = LastMeasure {
        took: 600.0,
        ..last()
    };
    let moved = used(&[(1, 600 * GIB)]);
    let wait = 600.0 * SPARED;
    assert!(
        !due(Some(&slow), wait - 1.0, &moved),
        "a ten minute walk waits {wait} s however the disk moved"
    );
    assert!(due(Some(&slow), wait, &moved), "and is due once it has");
    let slower = LastMeasure {
        took: HOUR / 2.0,
        ..last()
    };
    let still = used(&[(1, 500 * GIB)]);
    let longer = HOUR / 2.0 * SPARED;
    assert!(
        !due(Some(&slower), longer - 1.0, &still),
        "a still disk after a half hour walk waits {longer} s, past six hours"
    );
    assert!(due(Some(&slower), longer, &still), "and is due once it has");
}

#[test]
fn what_moved_on_each_disk_counts_together_and_a_new_disk_counts_whole() {
    let half = MOVED / 2;
    let both = LastMeasure {
        at: 0.0,
        used: used(&[(1, 500 * GIB), (2, 100 * GIB)]),
        took: 60.0,
    };
    let opposite = used(&[(1, 500 * GIB + half), (2, 100 * GIB - half)]);
    assert!(
        due(Some(&both), HOUR, &opposite),
        "one filling as another empties"
    );
    let added = used(&[(1, 500 * GIB), (2, 100 * GIB), (3, MOVED)]);
    assert!(
        due(Some(&both), HOUR, &added),
        "a disk the last measure didn't see"
    );
}
