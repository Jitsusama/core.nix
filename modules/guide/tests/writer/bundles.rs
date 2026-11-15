//! The rescue bundles a writer keeps and a stick carries on: each machine's
//! newest kept, the writer's own over a stick's, and what a desktop mounted
//! read where it is.

use std::fs::{self, File};

use guide::rescue::Bundle;
use guide::stand_in::StandIn;
use guide::vault::store::Directory;
use guide::writer::GatherBundles;
use guide::writer::carried::{self, Carried};
use guide::{Answer, Answers, Journey, Standing, Step};

use super::{lsblk, repair, stick_file};
use crate::common::Scratch;

/// A bundle as a machine's install seals it, as far as a writer reads one
/// it carries on: which machine's it is, and when it was sealed.
fn bundle(machine: &str, sealed: Option<u64>) -> Vec<u8> {
    let sealed = sealed.map_or_default(|at| format!(r#","sealed":{at}"#));
    format!(
        r#"{{"format":1,"machine":"{machine}","opened_by":["the Nano"],"owner":"22f0175f-c166-49d2-96ab-c61575cddac5","db":"ab","to_keys":"a","to_passphrase":"b","keys":"c"{sealed}}}"#
    )
    .into_bytes()
}

fn carried(machine: &str, sealed: Option<u64>) -> Carried {
    Carried::read(bundle(machine, sealed), None).unwrap()
}

/// The writer's clock, in seconds since 1970, as the tests have it.
const NOW: u64 = 1_000_000;

#[test]
fn a_writer_keeps_each_machines_newest_bundle_and_never_an_older_one() {
    let directory = Scratch::new("writer-keeps-newest");
    let mut store = Directory::new(directory.join("rescue"));
    carried::keep(
        &mut store,
        None,
        vec![carried("laptop", Some(100)), carried("desktop", None)],
        NOW,
    )
    .unwrap();
    let kept = carried::keep(
        &mut store,
        None,
        vec![
            carried("laptop", Some(200)),
            carried("laptop", Some(90)),
            carried("laptop", None),
            carried("desktop", Some(5)),
        ],
        NOW,
    )
    .unwrap();
    assert_eq!(
        kept,
        vec![carried("desktop", Some(5)), carried("laptop", Some(200))]
    );
    // Nothing older replaces it later, either, nor one from before
    // bundles said when they were sealed.
    for older in [Some(150), None] {
        let kept = carried::keep(&mut store, None, vec![carried("laptop", older)], NOW).unwrap();
        assert_eq!(kept[1], carried("laptop", Some(200)));
    }
    // Nor anything but whole bundles is left beside them.
    let mut names: Vec<_> = fs::read_dir(directory.join("rescue"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["desktop-rescue-bundle.json", "laptop-rescue-bundle.json"]
    );
}

#[test]
fn the_writers_own_machines_bundle_is_kept_over_any_a_stick_carries_for_it() {
    let directory = Scratch::new("writer-keeps-own");
    let mut store = Directory::new(directory.join("rescue"));
    carried::keep(&mut store, None, vec![carried("laptop", Some(500))], NOW).unwrap();
    // Its own, from before bundles said when they were sealed, against a
    // stick's that says it's newer.
    let own = carried("laptop", None);
    let kept = carried::keep(
        &mut store,
        Some(own.clone()),
        vec![carried("laptop", Some(900))],
        NOW,
    )
    .unwrap();
    assert_eq!(kept, vec![own.clone()]);
    // And it stays, however new a stick's says it is.
    let kept = carried::keep(&mut store, None, vec![carried("laptop", Some(NOW))], NOW).unwrap();
    assert_eq!(kept, vec![carried("laptop", Some(NOW))]);
    let kept = carried::keep(
        &mut store,
        Some(own.clone()),
        vec![carried("laptop", Some(NOW))],
        NOW,
    )
    .unwrap();
    assert_eq!(kept, vec![own]);
}

#[test]
fn a_bundle_said_to_be_sealed_more_than_a_day_ahead_is_never_kept() {
    let directory = Scratch::new("writer-keeps-ahead");
    let mut store = Directory::new(directory.join("rescue"));
    let kept = carried::keep(
        &mut store,
        None,
        vec![
            carried("laptop", Some(NOW + 60 * 60)),
            carried("desktop", Some(NOW + 2 * 24 * 60 * 60)),
            carried("laptop", Some(u64::MAX)),
        ],
        NOW,
    )
    .unwrap();
    assert_eq!(kept, vec![carried("laptop", Some(NOW + 60 * 60))]);
}

#[test]
fn a_writer_keeps_bundles_for_only_so_many_machines_each_named_plainly() {
    let directory = Scratch::new("writer-keeps-few");
    let mut store = Directory::new(directory.join("rescue"));
    let found: Vec<Carried> = (0..carried::MACHINES + 8)
        .map(|n| carried(&format!("machine-{n:02}"), Some(1)))
        .collect();
    let kept = carried::keep(&mut store, None, found, NOW).unwrap();
    assert_eq!(kept.len(), carried::MACHINES);
    assert_eq!(
        fs::read_dir(directory.join("rescue")).unwrap().count(),
        carried::MACHINES
    );
    // Names FAT would mix up, or couldn't hold, aren't bundles' names.
    for machine in ["Laptop", "lap/top", "-laptop", "", &"a".repeat(64)] {
        assert_eq!(
            Carried::read(bundle(machine, Some(1)), None),
            None,
            "{machine}"
        );
    }
}

#[test]
fn a_stick_for_a_machine_whose_bundle_the_writer_keeps_is_signed_from_it_unless_another_is_given() {
    let kept = vec![carried("desktop", Some(3)), carried("laptop", Some(7))];
    let chosen = |given: Option<&str>, machine: Option<&str>| {
        carried::bundle_for(
            given.map(|machine| Bundle::read(&bundle(machine, Some(1))).unwrap()),
            machine,
            &kept,
        )
        .unwrap()
        .map(|bundle| (bundle.machine().to_string(), bundle.sealed()))
    };
    assert_eq!(chosen(None, Some("laptop")), Some(("laptop".into(), 7)));
    assert_eq!(
        chosen(Some("laptop"), Some("laptop")),
        Some(("laptop".into(), 1))
    );
    assert_eq!(chosen(None, Some("tower")), None);
    // A repair stick is for no machine, so none of the writer's is used.
    assert_eq!(chosen(None, None), None);
}

#[test]
fn a_bundle_kept_under_another_machines_name_or_none_is_left_unread() {
    let directory = Scratch::new("writer-keeps-named");
    let root = directory.join("rescue");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("desktop-rescue-bundle.json"),
        bundle("laptop", Some(1)),
    )
    .unwrap();
    fs::write(root.join("laptop-rescue-bundle.json"), b"not a bundle").unwrap();
    fs::write(root.join("notes.json"), bundle("laptop", Some(1))).unwrap();
    assert_eq!(carried::kept(&mut Directory::new(&root)), vec![]);
}

#[test]
fn a_stick_carries_every_bundle_on_and_the_next_writer_keeps_them_off_it() {
    let directory = Scratch::new("writer-carries");
    let stick = stick_file(&directory);
    let mut stand_in = StandIn::new();
    lsblk(&mut stand_in, fixture!("lsblk-laid-out.json"));
    stand_in.disk("/dev/sda", &stick);
    let bundles = vec![carried("laptop", Some(7)), carried("desktop", Some(3))];
    let step = repair(&directory).carrying(bundles);
    let mut answers = Answers::new();
    answers.give("stick", Answer::Done);
    // Only the stick whose bundles were kept is written over.
    let standing = step.observe(&mut stand_in).unwrap();
    assert!(
        matches!(&standing, Standing::Blocked { why, .. } if why.contains("isn't the stick whose rescue bundles were kept")),
        "{standing:?}"
    );
    stand_in.no_file(guide::rescue::KEPT);
    Journey::attempt(
        &GatherBundles::new(directory.join("first-writer")),
        &mut stand_in,
        &Answers::new(),
    )
    .unwrap();
    Journey::attempt(&step, &mut stand_in, &answers).unwrap();
    let on_it = carried::on_stick(&mut File::open(&stick).unwrap());
    assert_eq!(
        on_it,
        vec![carried("desktop", Some(3)), carried("laptop", Some(7))]
    );

    // Another writer, about to write over it, keeps them first, and its
    // own machine's beside them, the vault mounted as the stick holds it.
    let mounted = directory.join("mounted");
    carried::keep(&mut Directory::new(&mounted), None, on_it, NOW).unwrap();
    let mut writer = StandIn::new();
    lsblk(&mut writer, fixture!("lsblk-laid-out.json"));
    writer
        .mount("/dev/sda4", &mounted)
        .file(guide::rescue::KEPT, &bundle("tower", Some(9)));
    let gather = GatherBundles::new(directory.join("writer"));
    assert!(matches!(
        gather.observe(&mut writer).unwrap(),
        Standing::Needed { .. }
    ));
    Journey::attempt(&gather, &mut writer, &Answers::new()).unwrap();
    // Mounted only for reading, which needs no password, and never the
    // disk opened, which would.
    let mounting: Vec<&String> = writer
        .calls
        .iter()
        .filter(|call| call.contains("mounting") || call.starts_with("opening"))
        .collect();
    assert_eq!(
        mounting,
        ["mounting /dev/sda4 for reading", "unmounting /dev/sda4"],
        "{:#?}",
        writer.calls
    );
    assert_eq!(
        carried::kept(&mut Directory::new(directory.join("writer"))),
        vec![
            carried("desktop", Some(3)),
            carried("laptop", Some(7)),
            carried("tower", Some(9)),
        ]
    );
    assert!(matches!(
        gather.observe(&mut writer).unwrap(),
        Standing::Done { .. }
    ));
}

#[test]
fn a_vault_a_desktop_mounted_is_read_where_it_is_and_the_disk_never_opened() {
    let directory = Scratch::new("writer-gathers-mounted");
    let mounted = directory.join("mounted");
    fs::create_dir_all(&mounted).unwrap();
    fs::write(
        mounted.join("laptop-rescue-bundle.json"),
        bundle("laptop", Some(4)),
    )
    .unwrap();
    fs::write(mounted.join(guide::vault::KEY_PASSPHRASE), b"sealed").unwrap();
    let mut seen: serde_json::Value =
        serde_json::from_str(fixture!("lsblk-laid-out.json")).unwrap();
    for disk in seen["blockdevices"].as_array_mut().unwrap() {
        let partitions = disk
            .get_mut("children")
            .and_then(|found| found.as_array_mut());
        for partition in partitions.into_iter().flatten() {
            if partition["path"] == "/dev/sda4" {
                partition["mountpoints"] = serde_json::json!([mounted.to_str().unwrap()]);
            }
        }
    }
    let mut writer = StandIn::new();
    lsblk(&mut writer, &seen.to_string());
    writer.no_file(guide::rescue::KEPT);
    let gather = GatherBundles::new(directory.join("writer"));
    Journey::attempt(&gather, &mut writer, &Answers::new()).unwrap();
    assert!(
        !writer
            .calls
            .iter()
            .any(|call| call.starts_with("opening") || call.contains("mounting")),
        "{:#?}",
        writer.calls
    );
    assert_eq!(
        carried::kept(&mut Directory::new(directory.join("writer"))),
        vec![carried("laptop", Some(4))]
    );
    // The copy of its key the stick's passphrase opens, kept for the
    // write to prove a passphrase kept against.
    assert_eq!(
        gather.phrase.borrow().as_deref(),
        Some(b"sealed".as_slice())
    );
}

#[test]
fn a_vault_that_wont_mount_carries_nothing_and_stops_nothing() {
    let directory = Scratch::new("writer-gathers-unread");
    let mut writer = StandIn::new();
    lsblk(&mut writer, fixture!("lsblk-laid-out.json"));
    // Nothing scripted for mounting it, so it fails as a vault that
    // isn't one would.
    writer.no_file(guide::rescue::KEPT);
    let gather = GatherBundles::new(directory.join("writer"));
    Journey::attempt(&gather, &mut writer, &Answers::new()).unwrap();
    assert_eq!(
        carried::kept(&mut Directory::new(directory.join("writer"))),
        vec![]
    );
    // Which it says, so the person knows the stick's copies are gone.
    let standing = gather.observe(&mut writer).unwrap();
    assert!(
        matches!(&standing, Standing::Done { evidence } if evidence.contains("its vault couldn't be read")),
        "{standing:?}"
    );
}
