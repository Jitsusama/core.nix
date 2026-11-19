//! The faults health sees itself, rather than hears: the lid reading
//! wrong, a disk guard gone quiet and a machine whose calls go unseen,
//! each counted once until it clears, through a start and across
//! midnight.

use super::*;

/// A look on which `unseen` machines went unseen, `answered` nodes here
/// answered and those `unanswered` didn't.
fn look(unseen: &[&str], answered: usize, unanswered: &[&str]) -> calls::elsewhere::Heard {
    let owned = |names: &[&str]| names.iter().map(|name| (*name).to_owned()).collect();
    calls::elsewhere::Heard {
        unseen: owned(unseen),
        unanswered: owned(unanswered),
        answered,
        ..calls::elsewhere::Heard::default()
    }
}

/// A look on which `look` heard, with `well` machines heard on no call.
fn heard_well(look: calls::elsewhere::Heard, well: &[&str]) -> calls::elsewhere::Heard {
    calls::elsewhere::Heard {
        no_call: well.iter().map(|name| (*name).to_owned()).collect(),
        ..look
    }
}

#[test]
fn a_lid_gone_wrong_is_a_devices_fault_once() {
    let into = tempfile::tempdir().expect("a folder");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
    let used = |keys| Looked {
        shut: true,
        screen_on: true,
        keys,
    };
    for keys in 1..5 {
        follower.look_at_lid(used(keys), NOON);
    }
    let open = Looked {
        shut: false,
        ..used(5)
    };
    follower.look_at_lid(open, NOON);
    follower.look_at_lid(used(6), NOON);
    follower.look_at_lid(used(7), NOON);
    follower.write().expect("the day is written");
    let lid = &read(into.path(), "2026-11-12").things["lid"];
    assert_eq!(lid.kind, Some(Kind::Devices), "a lid is a device");
    assert_eq!(lid.errors, 2, "once a time it went wrong: {lid:?}");
    let said: Vec<&str> = lid
        .signatures
        .values()
        .map(|said| said.example.as_str())
        .collect();
    assert_eq!(said, [LID_WRONG], "said the way Joel reads it");
}

#[test]
fn a_restart_keeps_the_faults_health_saw_and_counts_one_still_seen_no_more() {
    let into = tempfile::tempdir().expect("a folder");
    let status = into.path().join("status.json");
    let beat = |seconds: i64| {
        fs::write(
            &status,
            format!(r#"{{"heartbeat": {seconds}, "filesystems": []}}"#),
        )
        .expect("a status written");
    };
    let used = |keys| Looked {
        shut: true,
        screen_on: true,
        keys,
    };
    let noon = NOON / 1_000_000;
    let mut follower =
        Follower::new("2026-11-12", into.path(), TimeZone::UTC).guarding(vec![status.clone()]);
    for keys in 1..5 {
        follower.look_at_lid(used(keys), NOON);
    }
    beat(noon);
    follower.guard(NOON + 200_000_000);
    follower.write().expect("the day is written");
    let day = read(into.path(), "2026-11-12");
    assert_eq!(
        (day.things["lid"].errors, day.things["disk-guard"].errors),
        (1, 1),
        "health saw the lid wrong and the guard stopped, once each"
    );

    // A restart keeps both before health looks again.
    let mut again = Follower::new("2026-11-12", into.path(), TimeZone::UTC)
        .guarding(vec![status.clone()])
        .resuming(day);
    let errors = |day: &Day, name: &str| day.things.get(name).map(|thing| thing.errors);
    assert_eq!(
        (errors(&again.day, "lid"), errors(&again.day, "disk-guard")),
        (Some(1), Some(1)),
        "a restart lost what health saw itself"
    );
    assert_eq!(
        again.day.things["lid"].kind,
        Some(Kind::Devices),
        "a kept fault is still a device's"
    );

    // With the lid still wrong and the guard still quiet, neither counts again.
    for keys in 5..9 {
        again.look_at_lid(used(keys), NOON + 300_000_000);
    }
    again.guard(NOON + 300_000_000);
    assert_eq!(
        (errors(&again.day, "lid"), errors(&again.day, "disk-guard")),
        (Some(1), Some(1)),
        "a restart counted again what health still saw"
    );

    // The lid reads open, which alone is written; started again, it
    // counts again once it goes wrong again.
    again.look_at_lid(
        Looked {
            shut: false,
            ..used(9)
        },
        NOON + 330_000_000,
    );
    again.write().expect("the lid's clearing is written");
    let mut later = Follower::new("2026-11-12", into.path(), TimeZone::UTC)
        .guarding(vec![status.clone()])
        .resuming(read(into.path(), "2026-11-12"));
    for keys in 10..12 {
        later.look_at_lid(used(keys), NOON + 360_000_000);
    }
    assert_eq!(
        errors(&later.day, "lid"),
        Some(2),
        "a lid that cleared before a start wasn't counted once wrong again"
    );
    later.write().expect("the lid's fault is written");

    // The guard beats, which alone is written; started again, it counts
    // again once it stops again.
    beat(noon + 400);
    later.guard(NOON + 400_000_000);
    later.write().expect("the guard's clearing is written");
    let mut last = Follower::new("2026-11-12", into.path(), TimeZone::UTC)
        .guarding(vec![status.clone()])
        .resuming(read(into.path(), "2026-11-12"));
    last.guard(NOON + 600_000_000);
    assert_eq!(
        (errors(&last.day, "lid"), errors(&last.day, "disk-guard")),
        (Some(2), Some(2)),
        "a guard that beat before a start wasn't counted once stopped again, or a kept fault \
         was lost"
    );
}

#[test]
fn a_fault_still_seen_at_midnight_counts_once_in_each_day() {
    let into = tempfile::tempdir().expect("a folder");
    let status = into.path().join("status.json");
    fs::write(
        &status,
        format!(
            r#"{{"heartbeat": {}, "filesystems": []}}"#,
            NOON / 1_000_000
        ),
    )
    .expect("a status written");
    let used = |keys| Looked {
        shut: true,
        screen_on: true,
        keys,
    };
    let mut follower =
        Follower::new("2026-11-12", into.path(), TimeZone::UTC).guarding(vec![status.clone()]);
    for keys in 1..3 {
        follower.look_at_lid(used(keys), NOON);
    }
    follower.guard(NOON + 200_000_000);
    follower
        .take(&warning(NOON + DAY))
        .expect("the day rolls over");
    for keys in 3..5 {
        follower.look_at_lid(used(keys), NOON + DAY);
    }
    follower.guard(NOON + DAY);
    let errors = |day: &Day, name: &str| day.things.get(name).map(|thing| thing.errors);
    assert_eq!(
        (
            errors(&follower.day, "lid"),
            errors(&follower.day, "disk-guard")
        ),
        (Some(1), Some(1)),
        "a new day didn't count what health still saw wrong"
    );
    assert_eq!(
        (
            errors(&read(into.path(), "2026-11-12"), "lid"),
            errors(&read(into.path(), "2026-11-12"), "disk-guard")
        ),
        (Some(1), Some(1)),
        "the day before lost what health saw"
    );
}

#[test]
fn a_disk_guard_that_stopped_is_a_services_fault_once() {
    let into = tempfile::tempdir().expect("a folder");
    let status = into.path().join("status.json");
    let said = |seconds: i64, free: u64| {
        fs::write(
            &status,
            format!(
                r#"{{"heartbeat": {seconds}, "filesystems": [
                        {{"path": "/", "band": "filling", "free": {free}, "size": 100}}]}}"#
            ),
        )
        .expect("a status written");
    };
    let beat = |seconds: i64| said(seconds, 10);
    let mut follower =
        Follower::new("2026-11-12", into.path(), TimeZone::UTC).guarding(vec![status.clone()]);
    let noon = NOON / 1_000_000;
    beat(noon);
    follower.guard(NOON);
    // Quiet past the two minutes a guard may be, for two looks.
    follower.guard(NOON + 200_000_000);
    follower.guard(NOON + 230_000_000);
    beat(noon + 260);
    follower.guard(NOON + 260_000_000);
    follower.guard(NOON + 500_000_000);
    follower.write().expect("the day is written");
    let day = read(into.path(), "2026-11-12");
    assert_eq!(
        day.things["disk-guard"].errors, 2,
        "once each time it stopped: {:?}",
        day.things["disk-guard"]
    );
    assert_eq!(
        day.disks.filesystems["/"].band, "filling",
        "the guard's word on the disks wasn't kept"
    );

    // A restart, after which the disk has more free than the day's least.
    let mut again = Follower::new("2026-11-12", into.path(), TimeZone::UTC)
        .guarding(vec![status.clone()])
        .resuming(day);
    assert_eq!(
        again.day.things["disk-guard"].errors, 2,
        "a restart kept fewer of the guard's stops than the day had"
    );
    said(noon + 600, 60);
    again.guard(NOON + 600_000_000);
    again.write().expect("the day is written again");
    assert_eq!(
        read(into.path(), "2026-11-12").disks.filesystems["/"].least_free,
        10,
        "a restart lost the day's least free space"
    );
}

#[test]
fn a_machine_whose_calls_go_unseen_is_a_standing_fault_each() {
    let into = tempfile::tempdir().expect("a folder");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
    let unheard = |names: &[&str], well: &[&str]| heard_well(look(names, 1, &[]), well);
    let second = 1_000_000;
    let errors = |day: &Day, name: &str| {
        day.things
            .get(&format!("calls on {name}"))
            .map(|thing| thing.errors)
    };
    // A blip, mended within the three minutes, counts nothing.
    follower.see_heard(&unheard(&["skiff", "dinghy", "skerry"], &[]), NOON);
    follower.see_heard(
        &unheard(&["skiff", "dinghy"], &["skerry"]),
        NOON + 30 * second,
    );
    follower.see_heard(
        &unheard(&["skiff", "dinghy"], &["skerry"]),
        NOON + 179 * second,
    );
    assert_eq!(
        (
            errors(&follower.day, "skiff"),
            errors(&follower.day, "dinghy")
        ),
        (None, None),
        "a machine unseen under three minutes was counted"
    );
    follower.see_heard(
        &unheard(&["skiff", "dinghy", "skerry"], &[]),
        NOON + 180 * second,
    );
    follower.see_heard(
        &unheard(&["skiff"], &["dinghy", "skerry"]),
        NOON + 210 * second,
    );
    follower.write().expect("the day is written");
    let day = read(into.path(), "2026-11-12");
    assert_eq!(
        (
            errors(&day, "skiff"),
            errors(&day, "dinghy"),
            errors(&day, "skerry")
        ),
        (Some(1), Some(1), None),
        "each machine unseen for three minutes wasn't one fault, or one heard between was"
    );
    assert_eq!(
        day.things["calls on skiff"].kind,
        Some(Kind::Services),
        "an unseen machine isn't a service's fault"
    );
    let said: Vec<&str> = day.things["calls on skiff"]
        .signatures
        .values()
        .map(|said| said.example.as_str())
        .collect();
    assert_eq!(said, [peers::CALL_UNSEEN], "said the way Joel reads it");

    // A restart keeps what's still unseen, so it isn't counted again,
    // even once the restarted follower has seen it unseen as long, or its
    // first look finds no node yet, as when health and the node start
    // together.
    let mut again = Follower::new("2026-11-12", into.path(), TimeZone::UTC).resuming(day);
    again.see_heard(&look(&[], 0, &[]), NOON + 230 * second);
    again.see_heard(&unheard(&["skiff"], &[]), NOON + 240 * second);
    again.see_heard(&unheard(&["skiff"], &[]), NOON + 420 * second);
    assert_eq!(
        errors(&again.day, "skiff"),
        Some(1),
        "a restart counted again a machine still unseen"
    );

    // dinghy, seen again since, counts again once it's unseen again for
    // as long.
    again.see_heard(&unheard(&["skiff", "dinghy"], &[]), NOON + 450 * second);
    assert_eq!(
        errors(&again.day, "dinghy"),
        Some(1),
        "a machine unseen again was counted at once"
    );
    again.see_heard(&unheard(&["skiff", "dinghy"], &[]), NOON + 630 * second);
    assert_eq!(
        errors(&again.day, "dinghy"),
        Some(2),
        "a machine unseen again wasn't counted again"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn a_node_here_nobody_answers_on_is_a_fault_once_its_time_is_up() {
    let into = tempfile::tempdir().expect("a folder");
    let runtimes = tempfile::tempdir().expect("runtimes");
    let held = runtimes.path().join("1000").join("mycelium");
    std::fs::create_dir_all(&held).expect("a node's folder");
    // A socket its node has let go of: connecting is refused at once.
    drop(std::os::unix::net::UnixListener::bind(held.join("work.sock")).expect("a socket"));
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC)
        .hearing_peers(runtimes.path().to_path_buf());
    let second = 1_000_000;
    follower.hear_peers(NOON);
    follower.hear_peers(NOON + 180 * second);
    follower.write().expect("the day is written");
    let day = read(into.path(), "2026-11-12");
    assert_eq!(
        day.things.get(peers::NODES).map(|thing| thing.errors),
        Some(1),
        "a node nobody answers on wasn't counted once: {:?}",
        day.things.keys().collect::<Vec<_>>()
    );
}

#[test]
fn a_look_a_node_didnt_answer_on_mends_nothing_and_counts_nothing_again() {
    let into = tempfile::tempdir().expect("a folder");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
    let second = 1_000_000;
    let errors = |day: &Day, name: &str| day.things.get(name).map(|thing| thing.errors);
    let unheard = |names: &[&str]| look(names, 1, &[]);
    follower.see_heard(&unheard(&["skiff"]), NOON);
    follower.see_heard(&unheard(&["skiff", "dinghy"]), NOON + 120 * second);
    follower.see_heard(&unheard(&["skiff", "dinghy"]), NOON + 180 * second);
    // A node restarting answers nothing for a look while another answers,
    // so neither machine heard through it is said then; nor on a look
    // that found no node.
    follower.see_heard(&look(&[], 1, &["work.sock"]), NOON + 210 * second);
    follower.see_heard(&look(&[], 0, &[]), NOON + 225 * second);
    follower.see_heard(&unheard(&["skiff", "dinghy"]), NOON + 240 * second);
    follower.see_heard(&unheard(&["skiff", "dinghy"]), NOON + 300 * second);
    assert_eq!(
        errors(&follower.day, "calls on dinghy"),
        Some(1),
        "a machine unseen across a look a node didn't answer on lost its time"
    );
    follower.see_heard(&unheard(&["skiff", "dinghy"]), NOON + 420 * second);
    assert_eq!(
        errors(&follower.day, "calls on skiff"),
        Some(1),
        "a machine counted before a look a node didn't answer on was counted again after"
    );
    assert_eq!(
        errors(&follower.day, "calls on dinghy"),
        Some(1),
        "a machine unseen across a look a node didn't answer on was counted twice"
    );
    assert_eq!(
        errors(&follower.day, peers::NODES),
        None,
        "a node answering again within the three minutes was counted"
    );
}

#[test]
fn a_node_that_stays_mute_keeps_no_machines_time_for_ever() {
    let into = tempfile::tempdir().expect("a folder");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
    let second = 1_000_000;
    let errors = |day: &Day| day.things.get("calls on skiff").map(|thing| thing.errors);
    // One node of two stays mute all along, while skiff, unseen once,
    // goes unsaid for ten minutes: its record is the mute node's, or
    // asleep.
    let mute = |unseen: &[&str]| look(unseen, 1, &["work.sock"]);
    follower.see_heard(&mute(&["skiff"]), NOON);
    for minute in 1..=10 {
        follower.see_heard(&mute(&[]), NOON + minute * 60 * second);
    }
    follower.see_heard(&mute(&["skiff"]), NOON + 660 * second);
    assert_eq!(
        errors(&follower.day),
        None,
        "a machine unseen ten minutes before was counted at once, beside a mute node"
    );
    follower.see_heard(&mute(&["skiff"]), NOON + 840 * second);
    assert_eq!(
        errors(&follower.day),
        Some(1),
        "a machine unseen three minutes beside a mute node wasn't counted"
    );
}

#[test]
fn only_a_machine_heard_well_mends() {
    let into = tempfile::tempdir().expect("a folder");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
    let second = 1_000_000;
    let errors = |day: &Day, name: &str| {
        day.things
            .get(&format!("calls on {name}"))
            .map(|thing| thing.errors)
    };
    let mute = |unseen: &[&str], well: &[&str]| heard_well(look(unseen, 1, &["work.sock"]), well);
    follower.see_heard(&look(&["skiff"], 1, &[]), NOON);
    follower.see_heard(&look(&["skiff"], 1, &[]), NOON + 180 * second);
    // Its node restarts and answers before it's heard its peers again,
    // then goes mute for longer than three minutes.
    follower.see_heard(&look(&[], 1, &[]), NOON + 210 * second);
    for minute in 4..=10 {
        follower.see_heard(&mute(&[], &[]), NOON + minute * 60 * second);
    }
    follower.see_heard(&look(&["skiff"], 1, &[]), NOON + 630 * second);
    follower.see_heard(&look(&["skiff"], 1, &[]), NOON + 840 * second);
    assert_eq!(
        errors(&follower.day, "skiff"),
        Some(1),
        "a machine never heard well was counted again once its node said it"
    );

    // Beside a mute node, blips heard well between them add up to nothing.
    follower.see_heard(&mute(&["dinghy"], &[]), NOON + 900 * second);
    follower.see_heard(&mute(&[], &["dinghy"]), NOON + 1000 * second);
    follower.see_heard(&mute(&["dinghy"], &[]), NOON + 1050 * second);
    follower.see_heard(&mute(&["dinghy"], &[]), NOON + 1200 * second);
    assert_eq!(
        errors(&follower.day, "dinghy"),
        None,
        "blips a mute node's neighbour heard well between were counted"
    );
    follower.see_heard(&mute(&["dinghy"], &[]), NOON + 1230 * second);
    assert_eq!(
        errors(&follower.day, "dinghy"),
        Some(1),
        "a machine unseen three minutes since it was heard well wasn't counted"
    );

    // Heard well, skiff counts again once it's unseen again as long.
    follower.see_heard(
        &heard_well(look(&[], 1, &[]), &["skiff"]),
        NOON + 1260 * second,
    );
    follower.see_heard(&look(&["skiff"], 1, &[]), NOON + 1290 * second);
    follower.see_heard(&look(&["skiff"], 1, &[]), NOON + 1470 * second);
    assert_eq!(
        errors(&follower.day, "skiff"),
        Some(2),
        "a machine heard well and unseen again for three minutes wasn't counted again"
    );
}

#[test]
fn each_machine_and_the_nodes_whose_calls_go_unheard_are_a_thing_apart() {
    let heard = calls::elsewhere::Heard {
        unseen: vec!["skiff".to_owned()],
        untold: vec!["dinghy".to_owned()],
        undated: vec!["punt".to_owned()],
        unanswered: vec!["one".to_owned(), "two".to_owned()],
        unnamed: 40_000,
        ..calls::elsewhere::Heard::default()
    };
    assert_eq!(
        peers::unheard(&heard),
        [
            ("calls on skiff".to_owned(), peers::CALL_UNSEEN),
            ("calls on dinghy".to_owned(), peers::CALL_UNTOLD),
            ("calls on punt".to_owned(), peers::CALL_UNDATED),
            (peers::NODES.to_owned(), peers::NODE_SILENT),
            (peers::UNNAMED.to_owned(), peers::MACHINES_UNNAMED),
        ],
        "whose calls go unheard wasn't each a thing of its own"
    );
    assert!(
        peers::unheard(&calls::elsewhere::Heard::default()).is_empty(),
        "everything heard was still a fault"
    );
}
