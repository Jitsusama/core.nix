//! A machine that can't begin a run in the folder pineapple runs in here,
//! as a cloud room whose account can't enter Joel's home: it's asked once
//! more to begin it in the scratch folder, for a stop given there and for
//! Joel's stop on everything alike, and nothing else is asked twice.

use pineapple::pause::carry::{Carried, Carrier, Gift};
use pineapple::pause::{self, Plan, Took};

use super::carry::{heard, keep};
use super::{Machine, asked_with, machine, plan, scratch};

/// What an older node says when its fence can't start the shell in the
/// folder asked.
const NO_SHELL: &str = "pi-fence: bash didn't start: spawn bash EACCES (exit status: 126)";

/// What a newer node says when a machine can't begin a run in the folder
/// asked.
const CANT_BEGIN: &str = "mycelium: room can't begin this in /home/joel, since its launcher \
                          runs it as account 1001 (exit status: 1)";

/// The stop these cases give.
const NAME: &str = "slack-C1-1.000000";

/// A plan naming the room beside the Mac its sessions name.
fn with_room() -> Plan {
    Plan {
        machines: vec!["room".to_owned()],
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
