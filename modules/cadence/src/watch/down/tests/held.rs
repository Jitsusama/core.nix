//! What holds a going down back from being told, and what the journal
//! says of it: a battery, power no realm could read, a realm not
//! answering.

use super::*;

/// A busy machine as the work realm reads it, with power as given.
fn powered(reading: Reading, battery: Option<bool>) -> Seen {
    Seen {
        battery,
        ..one("yonder", reading, true)
    }
}

/// How many times a machine read down from five to thirty minutes on is
/// told, after `ready`, each a minute apart from `NOW`.
fn told_after(ready: Vec<Seen>, down: &Seen) -> usize {
    let mut told = Told::default();
    for (at, seen) in [0, 60].into_iter().zip(ready) {
        step(&mut told, NOW + at, &[seen]);
    }
    (5..=30)
        .step_by(5)
        .map(|minutes| step(&mut told, NOW + minutes * 60, std::slice::from_ref(down)).len())
        .sum()
}

#[test]
fn power_once_heard_stands_and_a_down_readings_word_settles_power_unread() {
    let lost_unread = Seen {
        battery: None,
        ..lost("yonder", 200)
    };
    let lost_battery = Seen {
        battery: Some(true),
        ..lost("yonder", 200)
    };
    let cases = [
        (
            "a desktop heard, then a tick its power couldn't be read",
            vec![
                powered(Reading::Ready, Some(false)),
                powered(Reading::Ready, None),
            ],
            lost("yonder", 200),
            1,
        ),
        (
            "power unread while ready, a down reading hearing no battery",
            vec![powered(Reading::Ready, None)],
            lost("yonder", 200),
            1,
        ),
        (
            "power never heard",
            vec![powered(Reading::Ready, None)],
            lost_unread,
            0,
        ),
        (
            "a laptop heard, then a tick reading no battery",
            vec![
                powered(Reading::Ready, Some(true)),
                powered(Reading::Ready, Some(false)),
            ],
            lost("yonder", 200),
            0,
        ),
        (
            "a desktop heard, then a down reading hearing a battery",
            vec![powered(Reading::Ready, Some(false))],
            lost_battery,
            0,
        ),
    ];
    for (case, ready, down, expected) in cases {
        assert_eq!(told_after(ready, &down), expected, "{case}: told wrongly");
    }
}

#[test]
fn a_busy_machine_held_back_by_its_power_is_said_once_a_spell() {
    for (battery, why) in [
        (None, "its power is unread, which may be a battery"),
        (Some(true), "its node heard a battery"),
    ] {
        let mut told = Told::default();
        step(&mut told, NOW, &[powered(Reading::Ready, battery)]);
        let down = Seen {
            battery,
            ..lost("yonder", 200)
        };
        let mut said = Vec::new();
        for minutes in (5..=30).step_by(5) {
            let before = held_now(&told);
            let now = NOW + minutes * 60;
            step(&mut told, now, std::slice::from_ref(&down));
            said.extend(held_back(&before, &told, now));
        }
        assert_eq!(
            said,
            [format!(
                "yonder has read down 10 min with a session busy but isn't told, as {why}, so \
                 a shut lid may have slept it"
            )],
            "a machine held back as {battery:?} wasn't said once"
        );
    }
}

#[test]
fn a_down_one_realm_reads_waits_for_a_silent_realm_to_answer() {
    let both = ["work".to_owned(), "personal".to_owned()];
    let mut told = Told::default();
    due(
        &[one("yonder", Reading::Ready, true)],
        &both,
        &both,
        &mut told,
        NOW,
    );
    let down = [lost("yonder", 200)];
    for minutes in [5, 10, 15, 20] {
        assert!(
            due(&down, &both, &work(), &mut told, NOW + minutes * 60).is_empty(),
            "a down was told {minutes} min on while a realm that may hold its peers was silent"
        );
    }
    assert_eq!(
        due(&down, &both, &both, &mut told, NOW + 25 * 60).len(),
        1,
        "once every realm answered, the down wasn't told"
    );
}
