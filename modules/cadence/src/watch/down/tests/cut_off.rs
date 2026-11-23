//! How the watch tells this machine cut off from another machine down.

use super::*;

#[test]
fn machines_last_heard_together_with_none_ready_is_this_one_cut_off() {
    let up = [
        one("yonder", Reading::Ready, true),
        one("near", Reading::Ready, true),
    ];
    let down = [lost("yonder", 200), lost("near", 230)];
    // However long it lasts, past the hour they last read ready.
    let mut told = Told::default();
    step(&mut told, NOW, &up);
    for minutes in (5..=120).step_by(5) {
        assert!(
            step(&mut told, NOW + minutes * 60, &down).is_empty(),
            "every machine dropping at once was told at {minutes} min"
        );
    }
    // A first tick that reads nothing, or nothing but unread, then the drop.
    for between in [vec![], vec![one("yonder", Reading::Unread, true)]] {
        let mut told = Told::default();
        step(&mut told, NOW, &up);
        step(&mut told, NOW + 5 * 60, &between);
        for minutes in (10..=40).step_by(5) {
            assert!(
                step(&mut told, NOW + minutes * 60, &down).is_empty(),
                "a drop after a tick reading {between:?} was told at {minutes} min"
            );
        }
    }
    // Peers lost either side of a tick.
    let mut told = Told::default();
    step(&mut told, NOW, &up);
    let straddled = [lost("yonder", 290), one("near", Reading::Ready, true)];
    step(&mut told, NOW + 5 * 60, &straddled);
    let down = [lost("yonder", 290), lost("near", 320)];
    step(&mut told, NOW + 10 * 60, &down);
    // And back either side of a tick, the count the cut-off held starts anew.
    let back = [lost("yonder", 290), one("near", Reading::Ready, true)];
    for minutes in [15, 20] {
        assert!(
            step(&mut told, NOW + minutes * 60, &back).is_empty(),
            "a peer back a tick after another was told at {minutes} min"
        );
    }
}

#[test]
fn a_machine_unsure_of_its_own_reach_tells_a_drop_only_beside_one_ready() {
    let unsure = |seen: &[Seen], told: &mut Told, now| {
        super::super::due(seen, &work(), &work(), told, now, false)
    };
    // A lone peer, which this machine losing its link would drop the same.
    let mut told = Told::default();
    unsure(&[one("yonder", Reading::Ready, true)], &mut told, NOW);
    for minutes in (5..=30).step_by(5) {
        assert!(
            unsure(&[lost("yonder", 200)], &mut told, NOW + minutes * 60).is_empty(),
            "a lone peer's drop was told unsure at {minutes} min"
        );
    }
    // Beside one still ready, which shows this machine's link holds.
    let mut told = Told::default();
    let up = [
        one("yonder", Reading::Ready, true),
        one("near", Reading::Ready, true),
    ];
    unsure(&up, &mut told, NOW);
    let mixed = [lost("yonder", 200), one("near", Reading::Ready, true)];
    unsure(&mixed, &mut told, NOW + 5 * 60);
    assert_eq!(
        unsure(&mixed, &mut told, NOW + 15 * 60).len(),
        1,
        "a drop beside a ready machine wasn't told unsure"
    );
}

#[test]
fn what_the_cut_off_cant_tell_is_never_told() {
    let up = [
        one("yonder", Reading::Ready, true),
        one("near", Reading::Ready, true),
    ];
    // A machine down with no time heard.
    let mut told = Told::default();
    step(&mut told, NOW, &up);
    let unheard = Seen {
        heard: None,
        ..one("yonder", Reading::Down, true)
    };
    for minutes in (5..=30).step_by(5) {
        let down = [unheard.clone(), lost("near", 100)];
        assert!(
            step(&mut told, NOW + minutes * 60, &down).is_empty(),
            "a drop beside a machine with no time heard was told at {minutes} min"
        );
    }
    // Read down by two realms, one of which lost it long before.
    let mut told = Told::default();
    step(&mut told, NOW, &up);
    let both = ["work".to_owned(), "personal".to_owned()];
    let personal = |seen: Seen| Seen {
        realm: "personal".into(),
        ..seen
    };
    for minutes in (5..=30).step_by(5) {
        let down = [
            lost("yonder", -7200),
            personal(lost("yonder", 200)),
            personal(lost("near", 230)),
        ];
        assert!(
            due(&down, &both, &both, &mut told, NOW + minutes * 60).is_empty(),
            "a realm's stale hearing parted a cut-off, told at {minutes} min"
        );
    }
    // A realm not answering.
    let mut told = Told::default();
    step(&mut told, NOW, &up);
    for minutes in (5..=30).step_by(5) {
        let down = [lost("yonder", 200)];
        assert!(
            due(&down, &both, &work(), &mut told, NOW + minutes * 60).is_empty(),
            "a drop while a realm didn't answer was told at {minutes} min"
        );
    }
}

#[test]
fn a_machine_gone_past_the_hour_with_no_time_heard_hides_no_crash() {
    let mut told = Told::default();
    step(&mut told, NOW, &[one("yonder", Reading::Ready, true)]);
    let long_gone = Seen {
        heard: None,
        ..one("old", Reading::Down, false)
    };
    let crash = [long_gone, lost("yonder", 200)];
    step(&mut told, NOW + 5 * 60, &crash);
    assert_eq!(
        step(&mut told, NOW + 15 * 60, &crash).len(),
        1,
        "a machine never ready, with no time heard, hid a crash"
    );
}

#[test]
fn three_machines_tell_apart_a_cut_off_from_a_crash() {
    let three = [
        one("yonder", Reading::Ready, true),
        one("near", Reading::Ready, true),
        one("far", Reading::Ready, true),
    ];
    // Two dropping together beside one still ready is no cut-off.
    let mut told = Told::default();
    step(&mut told, NOW, &three);
    let pair = [
        lost("yonder", 200),
        lost("near", 210),
        one("far", Reading::Ready, true),
    ];
    step(&mut told, NOW + 5 * 60, &pair);
    assert_eq!(
        step(&mut told, NOW + 15 * 60, &pair).len(),
        2,
        "two machines down beside one ready weren't told"
    );
    // Two idle, asleep together long before, don't hide a third's crash.
    let mut told = Told::default();
    let idle_pair = [
        one("yonder", Reading::Ready, false),
        one("near", Reading::Ready, false),
        one("far", Reading::Ready, true),
    ];
    step(&mut told, NOW, &idle_pair);
    let crash = [lost("yonder", -9000), lost("near", -8990), lost("far", 200)];
    step(&mut told, NOW + 5 * 60, &crash);
    assert_eq!(
        step(&mut told, NOW + 15 * 60, &crash).len(),
        1,
        "a crash beside two machines asleep since long before wasn't told"
    );
}

#[test]
fn a_machine_down_apart_from_the_others_is_told() {
    let up = [
        one("yonder", Reading::Ready, true),
        one("near", Reading::Ready, true),
    ];
    // One down while another stays up.
    let mut told = Told::default();
    step(&mut told, NOW, &up);
    let mixed = [lost("yonder", 200), one("near", Reading::Ready, true)];
    step(&mut told, NOW + 5 * 60, &mixed);
    assert_eq!(
        step(&mut told, NOW + 15 * 60, &mixed).len(),
        1,
        "one machine down while another stayed up wasn't told"
    );
    // One asleep since earlier, then another failing on its own.
    let mut told = Told::default();
    step(&mut told, NOW, &up);
    let asleep = [one("yonder", Reading::Ready, true), lost("near", 100)];
    step(&mut told, NOW + 5 * 60, &asleep);
    let down = [lost("yonder", 500), lost("near", 100)];
    step(&mut told, NOW + 10 * 60, &down);
    step(&mut told, NOW + 15 * 60, &down);
    assert_eq!(
        step(&mut told, NOW + 20 * 60, &down).len(),
        1,
        "a working machine's crash after another slept was taken for a cut-off"
    );
}

#[test]
fn a_peer_whose_clock_runs_ahead_is_heard_as_now() {
    let ahead = Seen {
        heard: Some(NOW + 36_000),
        ..lost("near", 0)
    };
    let by = BTreeMap::from([
        ("yonder", (lost("yonder", 200), BTreeSet::new())),
        ("near", (ahead, BTreeSet::new())),
    ]);
    assert!(
        cut_off(&by, NOW + 300, true, &BTreeSet::new()),
        "a peer lost beside another, its clock ten hours ahead, was heard apart from it"
    );
}

#[test]
fn a_machine_with_no_time_heard_hides_a_drop_only_within_the_hour_it_read_ready() {
    let silent = Seen {
        heard: None,
        ..one("old", Reading::Down, true)
    };
    for (ready_ago, expected) in [(30, 0), (90, 1)] {
        let mut told = Told::default();
        let up = [
            one("old", Reading::Ready, true),
            one("yonder", Reading::Ready, true),
        ];
        step(&mut told, NOW - ready_ago * 60, &up);
        step(
            &mut told,
            NOW,
            &[silent.clone(), one("yonder", Reading::Ready, true)],
        );
        let crash = [silent.clone(), lost("yonder", 200)];
        step(&mut told, NOW + 5 * 60, &crash);
        assert_eq!(
            step(&mut told, NOW + 15 * 60, &crash).len(),
            expected,
            "a crash beside a machine with no time heard, ready {ready_ago} min before, was \
             judged wrongly"
        );
    }
}
