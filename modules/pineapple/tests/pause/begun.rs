//! A machine that can't begin a run in the folder pineapple runs in here,
//! as a cloud room whose account can't enter Joel's home: it's asked once
//! more to begin it in the scratch folder, for a stop given there and for
//! Joel's stop on everything alike, and nothing else is asked twice. A
//! machine keeping the node's own stops folder, which its guards read,
//! is given a stop in its account's folder and then the node's, and the
//! give fails where the node's can't be written.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use pineapple::pause::carry::{Carried, Carrier, Gift, NODE_UNWRITTEN, script, taken_away};
use pineapple::pause::{self, Plan, Took};

use super::carry::{heard, keep};
use super::{
    FORKS, Machine, asked_with, at_home, json, machine, node_stops, plan, runs_as_root, scratch,
};

/// What an older node says when its fence can't start the shell in the
/// folder asked.
const NO_SHELL: &str = "pi-fence: bash didn't start: spawn bash EACCES (exit status: 126)";

/// What a newer node says when a machine can't begin a run in the folder
/// asked.
const CANT_BEGIN: &str = "mycelium: room can't begin this in /home/joel, since its launcher \
                          runs it as account 1001 (exit status: 1)";

/// The stop these cases give.
const NAME: &str = "slack-C1-1.000000";

/// A plan naming the room beside the Mac its sessions name, after it, so
/// a room asked again is put back in its own place, not the first.
fn with_room() -> Plan {
    Plan {
        machines: vec!["mac".to_owned(), "room".to_owned()],
        ..plan()
    }
}

/// The Mac's and the room's gives when `here` is asked to carry the
/// stop kept under [`NAME`] once.
fn carried(name: &str, here: &mut Machine) -> (Carried, Carried) {
    let state = scratch(name);
    keep(&state, NAME, &heard("slack:C1:1.000000"));
    let (gives, _) = Carrier::default().carry(&with_room(), &state, "here", 0, here);
    let _ = std::fs::remove_dir_all(&state);
    let of = |machine: &str| {
        gives
            .iter()
            .find(|one| one.machine == machine)
            .cloned()
            .unwrap_or_else(|| panic!("{machine} is given the stop: {gives:?}"))
    };
    (of("mac"), of("room"))
}

#[test]
fn a_room_that_cant_begin_in_joels_home_is_given_the_stop_in_the_scratch_folder() {
    for words in [NO_SHELL, CANT_BEGIN] {
        let mut here = Machine {
            homeless: Some(("machine=room", words)),
            ..machine()
        };

        let (mac, room) = carried("begun-carry", &mut here);

        assert_eq!(
            (mac.gift, mac.failed, room.gift, room.failed),
            (Gift::Stop, None, Gift::Stop, None),
            "both take the stop when the room says {words:?}"
        );
        let asked = asked_with(&here, "--need machine=room");
        assert!(
            asked.len() == 2
                && !asked[0].contains("--folder")
                && asked[1].contains("--need machine=room --folder /tmp -- sh -c"),
            "the room is asked where pineapple runs, then in the scratch folder: {asked:?}"
        );
        let mac = asked_with(&here, "--need machine=mac");
        assert!(
            mac.len() == 1 && !mac[0].contains("--folder"),
            "the Mac is asked once, where pineapple runs: {mac:?}"
        );
    }
}

#[test]
fn a_room_failing_otherwise_or_twice_is_asked_at_most_once_more() {
    let otherwise = "sh: 1: pineapple: Permission denied (exit status: 126)";
    for (words, refusing, asks) in [(otherwise, None, 1), (NO_SHELL, Some("--folder /tmp"), 2)] {
        let mut here = Machine {
            homeless: Some(("machine=room", words)),
            refusing,
            ..machine()
        };

        let (mac, room) = carried("begun-failed", &mut here);

        assert_eq!(mac.failed, None, "the Mac takes it: {mac:?}");
        assert!(
            room.failed.is_some(),
            "the room couldn't take it, refused with {words:?} then {refusing:?}: {room:?}"
        );
        assert_eq!(
            asked_with(&here, "--need machine=room").len(),
            asks,
            "the room is asked {asks} times: {:?}",
            here.asked
        );
    }
}

#[test]
fn joels_stop_on_everything_reaches_a_room_in_the_scratch_folder() {
    let state = scratch("begun-stop");
    let mut here = Machine {
        homeless: Some(("machine=room", NO_SHELL)),
        ..machine()
    };
    let plan = Plan {
        machines: vec!["mac".to_owned(), "room".to_owned()],
        ..plan()
    };

    let took = pause::stop(&plan, &state, "stop-all-1", "here", 1, &mut here).expect("stops");

    let room = took.iter().find(|one| one.machine == "room");
    assert_eq!(
        room,
        Some(&Took {
            machine: "room".to_owned(),
            failed: vec![],
            recorded: true,
            reached: true,
            realm: None,
        }),
        "the room takes it, begun in the scratch folder: {took:?}"
    );
    let asked = asked_with(&here, "--need machine=room");
    assert!(
        asked.len() == 2 && asked[1].contains("--need machine=room --folder /tmp -- sh -c"),
        "asked once where pineapple runs, once in the scratch folder: {asked:?}"
    );
    assert_eq!(
        asked_with(&here, "--need machine=mac").len(),
        1,
        "the Mac is asked once: {:?}",
        here.asked
    );
    let _ = std::fs::remove_dir_all(&state);
}

/// How a script ended and what it said on its error output.
#[derive(Debug)]
struct Ended {
    code: Option<i32>,
    said: String,
}

/// How `script` ends when run as a machine whose home is `home` runs it.
fn ended_at(home: &Path, script: &str) -> Ended {
    let _held = FORKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let ran = std::process::Command::new("sh")
        .arg("-c")
        .arg(at_home(home, script))
        .env("HOME", home)
        .env_remove("XDG_STATE_HOME")
        .output()
        .expect("sh runs");
    Ended {
        code: ran.status.code(),
        said: String::from_utf8_lossy(&ran.stderr).into_owned(),
    }
}

/// Whether `ended` is a script failing for the node's stops folder
/// `node`, naming it, so the give is told as untaken and tried again.
fn failed_for(ended: &Ended, node: &Path) -> bool {
    ended.code.is_some_and(|code| code != 0)
        && ended
            .said
            .contains(&format!("{NODE_UNWRITTEN} {}", node.display()))
}

/// The copy of the stop given under [`NAME`] in the account's stops
/// folder of `home`.
fn account_copy(home: &Path) -> std::path::PathBuf {
    home.join(".local/state/pineapple/stops")
        .join(format!("{NAME}.json"))
}

#[test]
fn a_give_holds_the_stop_in_both_folders_or_fails_keeping_the_accounts() {
    let case = "a_give_holds_the_stop_in_both_folders_or_fails_keeping_the_accounts";
    let home = scratch("begun-node-stops");
    let node = node_stops(&home);
    fs::create_dir_all(&node).expect("the node's stops folder");
    let record = heard("slack:C1:1.000000");
    let copy = node.join(format!("{NAME}.json"));

    let given = ended_at(&home, &script(NAME, &record, "listener"));
    assert_eq!(given.code, Some(0), "the stop is written: {given:?}");
    assert_eq!(
        (
            json(&account_copy(&home))["carried_from"].clone(),
            json(&copy)["carried_from"].clone()
        ),
        (serde_json::json!("listener"), serde_json::json!("listener")),
        "it's held in the account's folder, and in the node's, which the guards read"
    );

    fs::remove_dir_all(home.join(".local")).expect("the account's copy dropped");
    fs::remove_file(&copy).expect("the node's copy dropped");
    if runs_as_root(&home, case) {
        let _ = fs::remove_dir_all(&home);
        return;
    }
    fs::set_permissions(&node, fs::Permissions::from_mode(0o555)).expect("closed");
    let refused = ended_at(&home, &script(NAME, &record, "listener"));
    fs::set_permissions(&node, fs::Permissions::from_mode(0o755)).expect("opened");
    assert!(
        failed_for(&refused, &node),
        "a node's folder this account can't write fails the give, naming it: {refused:?}"
    );
    assert_eq!(
        json(&account_copy(&home))["carried_from"],
        "listener",
        "while the account's copy is kept"
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn taking_a_give_away_drops_both_copies_or_fails() {
    let case = "taking_a_give_away_drops_both_copies_or_fails";
    let home = scratch("begun-node-taken");
    let node = node_stops(&home);
    fs::create_dir_all(&node).expect("the node's stops folder");
    let record = heard("slack:C1:1.000000");
    let copy = node.join(format!("{NAME}.json"));
    let give = script(NAME, &record, "listener");

    assert_eq!(ended_at(&home, &give).code, Some(0), "the stop is given");
    let taken = ended_at(&home, &taken_away(NAME));
    assert_eq!(taken.code, Some(0), "its end runs: {taken:?}");
    assert!(
        !account_copy(&home).exists() && !copy.exists(),
        "and drops the copy from the account's folder and the node's"
    );

    if runs_as_root(&home, case) {
        let _ = fs::remove_dir_all(&home);
        return;
    }
    assert_eq!(
        ended_at(&home, &give).code,
        Some(0),
        "the stop is given again"
    );
    fs::set_permissions(&node, fs::Permissions::from_mode(0o555)).expect("closed");
    let kept = ended_at(&home, &taken_away(NAME));
    fs::set_permissions(&node, fs::Permissions::from_mode(0o755)).expect("opened");
    assert!(
        failed_for(&kept, &node),
        "an end that can't drop the node's copy fails, naming its folder: {kept:?}"
    );
    assert!(
        copy.exists(),
        "so the guards still read the stop till it's tried again"
    );
    let _ = fs::remove_dir_all(&home);
}
