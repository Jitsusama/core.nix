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

/// How many times a machine read down from five to thirty minutes after
/// the last of `ready` is told, each of `ready` read `apart` seconds after
/// the one before from `NOW`, and whether it was held back by its power.
fn told_after(ready: &[Seen], apart: i64, down: &Seen) -> (usize, bool) {
    let mut told = Told::default();
    let mut at = NOW;
    for (n, seen) in ready.iter().enumerate() {
        at = NOW + apart * i64::try_from(n).expect("a few readings");
        step(&mut told, at, std::slice::from_ref(seen));
    }
    let times = (5..=30)
        .step_by(5)
        .map(|minutes| step(&mut told, at + minutes * 60, std::slice::from_ref(down)).len())
        .sum();
    (times, told.machines["yonder"].held_at.is_some())
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
    let desktop = powered(Reading::Ready, Some(false));
    let laptop = powered(Reading::Ready, Some(true));
    let unread = powered(Reading::Ready, None);
    let cases = [
        (
            "a desktop heard, then ticks its power couldn't be read",
            vec![desktop.clone(), unread.clone(), unread.clone()],
            60,
            lost_unread.clone(),
            (1, false),
        ),
        (
            "power unread while ready, a down reading hearing no battery",
            vec![unread.clone()],
            60,
            lost("yonder", 200),
            (1, false),
        ),
        (
            "power never heard",
            vec![unread.clone()],
            60,
            lost_unread.clone(),
            (0, true),
        ),
        (
            "a laptop heard, then a tick reading no battery",
            vec![laptop.clone(), desktop.clone()],
            60,
            lost_unread.clone(),
            (0, true),
        ),
        (
            "a battery heard a day before, then none heard",
            vec![laptop, desktop.clone()],
            BATTERY_FOR,
            Seen {
                heard: Some(NOW + BATTERY_FOR + 200),
                ..lost_unread
            },
            (1, false),
        ),
        (
            "a desktop heard, then a down reading hearing a battery",
            vec![desktop],
            60,
            lost_battery,
            (0, true),
        ),
    ];
    for (case, ready, apart, down, expected) in cases {
        assert_eq!(
            told_after(&ready, apart, &down),
            expected,
            "{case}: told wrongly, or held for another reason"
        );
    }
}

/// What the journal says of holding a machine back over two spells of
/// going down, ready between, each tick as a tick watches.
fn said_over_two_spells(battery: Option<bool>) -> Vec<String> {
    let mut told = Told::default();
    let down = Seen {
        battery,
        ..lost("yonder", 200)
    };
    let mut said = Vec::new();
    for start in [NOW, NOW + 3_600] {
        let ready = powered(Reading::Ready, battery);
        watched(&[ready], &work(), &work(), &mut told, start, true);
        for minutes in (5..=30).step_by(5) {
            let now = start + minutes * 60;
            let down = std::slice::from_ref(&down);
            let (tells, lines) = watched(down, &work(), &work(), &mut told, now, true);
            assert!(tells.is_empty(), "a machine held back was told: {tells:?}");
            said.extend(lines);
        }
    }
    said
}

#[test]
fn a_laptop_shut_busy_past_a_day_is_never_told() {
    let mut told = Told::default();
    step(&mut told, NOW, &[powered(Reading::Ready, Some(true))]);
    let down = [Seen {
        battery: Some(true),
        ..lost("yonder", 300)
    }];
    let told_at: Vec<i64> = (1..=(BATTERY_FOR + 1_800) / 300)
        .map(|n| NOW + n * 300)
        .filter(|&now| !step(&mut told, now, &down).is_empty())
        .collect();
    assert!(
        told_at.is_empty(),
        "a laptop its node still hears a battery of was told down: {told_at:?}"
    );
}

#[test]
fn a_down_realms_repeated_battery_never_passes_as_another_realms_fresh_word() {
    let stale = Seen {
        realm: "personal".into(),
        battery: Some(true),
        ..lost("yonder", 0)
    };
    let (desktop, unread) = (
        powered(Reading::Ready, Some(false)),
        powered(Reading::Ready, None),
    );
    for (case, seen, battery) in [
        (
            "the down realm read first",
            [stale.clone(), desktop.clone()],
            Some(false),
        ),
        (
            "the ready realm read first",
            [desktop, stale.clone()],
            Some(false),
        ),
        ("the ready realm reading no power", [unread, stale], None),
    ] {
        let (kept, _) = &merged(&seen)["yonder"];
        assert_eq!(
            (kept.reading, kept.battery),
            (Reading::Ready, battery),
            "{case}: the down realm's battery rode into the ready reading"
        );
    }
}

#[test]
fn a_busy_machine_held_back_by_its_power_is_said_once_a_spell() {
    for (battery, why) in [
        (None, "its power is unread, which may be a battery"),
        (Some(true), "its node heard a battery"),
    ] {
        let line = format!(
            "yonder has read down 10 min with a session busy but isn't told, as {why}, so a \
             shut lid may have slept it"
        );
        assert_eq!(
            said_over_two_spells(battery),
            [line.clone(), line],
            "a machine held back as {battery:?} wasn't said once each spell"
        );
    }
}

#[test]
fn an_idle_laptop_and_one_not_ready_lately_are_never_said_held() {
    let laptop = |working| Seen {
        battery: Some(true),
        ..one("yonder", Reading::Ready, working)
    };
    let down = Seen {
        battery: Some(true),
        ..lost("yonder", 200)
    };
    for (case, ready, from) in [
        ("an idle laptop", laptop(false), NOW),
        ("one not ready lately", laptop(true), NOW + READY_WITHIN),
    ] {
        let mut told = Told::default();
        watched(&[ready], &work(), &work(), &mut told, NOW, true);
        let mut said = Vec::new();
        for minutes in (5..=30).step_by(5) {
            let now = from + minutes * 60;
            let down = std::slice::from_ref(&down);
            let (tells, lines) = watched(down, &work(), &work(), &mut told, now, true);
            assert!(tells.is_empty(), "{case} was told: {tells:?}");
            said.extend(lines);
        }
        assert!(said.is_empty(), "{case} was said held back: {said:?}");
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
