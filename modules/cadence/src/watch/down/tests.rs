use std::cell::RefCell;

use super::*;

mod cut_off;

const NOW: i64 = 1_795_200_000;

fn one(machine: &str, reading: Reading, working: bool) -> Seen {
    Seen {
        machine: machine.into(),
        realm: "work".into(),
        reading,
        working: Some(working),
        battery: false,
        heard: Some(NOW),
    }
}

/// A machine read unreachable, last heard from so many seconds past `NOW`.
fn lost(machine: &str, heard: i64) -> Seen {
    Seen {
        heard: Some(NOW + heard),
        ..one(machine, Reading::Down, false)
    }
}

fn work() -> Vec<String> {
    vec!["work".into()]
}

/// What's due at `now` on a machine sure of its own reach.
fn due(
    seen: &[Seen],
    asked: &[String],
    answered: &[String],
    told: &mut Told,
    now: i64,
) -> Vec<Tell> {
    super::due(seen, asked, answered, told, now, true)
}

/// What's due at `now`, told to everyone without a fault, as a tick does.
fn step(told: &mut Told, now: i64, seen: &[Seen]) -> Vec<Tell> {
    let tells = due(seen, &work(), &work(), told, now);
    let sent: Result<(), ()> = tell(&tells, told, "ORCH", |_| None, |_| Some(7), |_, _| Ok(()));
    assert_eq!(sent, Ok(()), "a send that couldn't fail failed");
    tells
}

#[test]
fn the_nodes_answers_are_read_for_each_other_machine() {
    let machines = concat!(
        r#"{"Ok":{"Machines":[{"name":"here","state":"ready"},"#,
        r#"{"name":"yonder","state":"unreachable","measured_at":"2026-11-21T08:32:49Z"},"#,
        r#"{"name":"far","state":"ready","power":{"on_battery":true}},"#,
        r#"{"name":"near","state":"ready"},{"name":"odd","state":"starting"}]}}"#,
        "\n",
        r#"{"Ok":{"Builders":[]}}"#,
    );
    let sessions = r#"{"Ok":{"Sessions":[{"machine":"yonder","state":"busy"},
        {"machine":"near","state":"idle"},{"machine":"here","state":"busy"}]}}"#;
    assert_eq!(
        seen("work", machines, sessions, "here"),
        [
            Seen {
                heard: Some(1_795_249_969),
                ..one("yonder", Reading::Down, true)
            },
            Seen {
                battery: true,
                heard: None,
                ..one("far", Reading::Ready, false)
            },
            Seen {
                heard: None,
                ..one("near", Reading::Ready, false)
            },
            Seen {
                heard: None,
                ..one("odd", Reading::Unread, false)
            },
        ],
        "the machines weren't read as the node says, or this one wasn't left out"
    );
    assert!(
        seen("work", machines, r#"{"Err":"timed out"}"#, "here")
            .iter()
            .all(|one| one.working.is_none()),
        "a node not saying which sessions are busy read every machine idle"
    );
    assert!(
        seen("work", r#"{"Err":"no such realm"}"#, "", "here").is_empty(),
        "a node answering with an error read machines"
    );
}

#[test]
fn a_working_machine_down_ten_minutes_is_told_once_and_again_once_back() {
    let mut told = Told::default();
    let down = [one("yonder", Reading::Down, false)];
    assert!(
        step(&mut told, NOW, &[one("yonder", Reading::Ready, true)]).is_empty(),
        "a ready machine was told"
    );
    let mut now = NOW + 5 * 60;
    assert!(
        step(&mut told, now, &down).is_empty(),
        "a machine down a moment was told"
    );
    now += 5 * 60;
    assert!(
        step(&mut told, now, &down).is_empty(),
        "a machine down five minutes was told"
    );
    // Ten minutes from when it first read down.
    now += 5 * 60;
    assert_eq!(
        step(&mut told, now, &down),
        [Tell::Down {
            machine: "yonder".into(),
            realm: "work".into(),
            minutes: 10,
        }],
        "a working machine down ten minutes wasn't told"
    );
    now += 25 * 60;
    assert!(
        step(&mut told, now, &down).is_empty(),
        "a machine still down was told again"
    );
    now += 5 * 60;
    assert_eq!(
        step(&mut told, now, &[one("yonder", Reading::Ready, false)]),
        [Tell::Back {
            machine: "yonder".into(),
            minutes: 40,
            at_least: false,
            to: BTreeSet::from(["ORCH".to_owned()]),
            notice: Some(7),
        }],
        "a machine back wasn't told to whoever had its going down"
    );
    now += 5 * 60;
    assert!(
        step(&mut told, now, &[one("yonder", Reading::Ready, false)]).is_empty(),
        "a machine back was told twice"
    );
}

/// A reading at so many minutes past `NOW`, and whether a session on the
/// machine was busy then.
type At = (i64, Reading, bool);

#[test]
fn a_machine_idle_never_ready_slept_away_from_or_back_in_moments_is_never_told() {
    let cases: [(&str, Vec<At>); 4] = [
        // Asleep with no work, as its node lets it.
        (
            "idle",
            vec![(0, Reading::Ready, false), (5, Reading::Down, false)],
        ),
        ("never ready", vec![(0, Reading::Down, true)]),
        // Ready, then this machine slept eight hours.
        (
            "slept away",
            vec![(0, Reading::Ready, true), (480, Reading::Down, true)],
        ),
        // A link that drops for a tick.
        (
            "a blip",
            vec![(0, Reading::Ready, true), (5, Reading::Down, true)],
        ),
    ];
    for (case, readings) in cases {
        let mut told = Told::default();
        for (minutes, reading, working) in readings {
            step(
                &mut told,
                NOW + minutes * 60,
                &[one("yonder", reading, working)],
            );
        }
        let last = told.machines["yonder"].read_at.unwrap_or(NOW);
        let blip = case == "a blip";
        let then: &[i64] = if blip { &[5] } else { &[5, 15, 60] };
        for &minutes in then {
            let reading = if blip { Reading::Ready } else { Reading::Down };
            assert!(
                step(
                    &mut told,
                    last + minutes * 60,
                    &[one("yonder", reading, false)]
                )
                .is_empty(),
                "{case}: a machine was told {minutes} min on"
            );
        }
    }
}

#[test]
fn a_laptop_on_battery_shut_while_busy_is_never_told() {
    let mut told = Told::default();
    let on_battery = Seen {
        battery: true,
        ..one("yonder", Reading::Ready, true)
    };
    step(&mut told, NOW, &[on_battery]);
    for minutes in (5..=30).step_by(5) {
        assert!(
            step(
                &mut told,
                NOW + minutes * 60,
                &[one("yonder", Reading::Down, true)]
            )
            .is_empty(),
            "a laptop on battery asleep was told at {minutes} min"
        );
    }
}

#[test]
fn only_readings_in_a_row_count_as_down() {
    let mut told = Told::default();
    let down = [one("yonder", Reading::Down, false)];
    step(&mut told, NOW, &[one("yonder", Reading::Ready, true)]);
    step(&mut told, NOW + 5 * 60, &down);
    // This machine slept twenty minutes, and its first reading starts anew.
    for minutes in [25, 30] {
        assert!(
            step(&mut told, NOW + minutes * 60, &down).is_empty(),
            "time this machine didn't read was counted as down, at {minutes} min"
        );
    }
    assert_eq!(
        step(&mut told, NOW + 35 * 60, &down).len(),
        1,
        "ten minutes of readings after a gap weren't told"
    );
    // One tick missed is absorbed.
    let mut told = Told::default();
    step(&mut told, NOW, &[one("near", Reading::Ready, true)]);
    step(&mut told, NOW + 5 * 60, &[one("near", Reading::Down, true)]);
    assert_eq!(
        step(
            &mut told,
            NOW + 15 * 60,
            &[one("near", Reading::Down, true)]
        )
        .len(),
        1,
        "a tick missed restarted the count"
    );
}

#[test]
fn a_laptop_unplugged_just_before_its_lid_shut_is_never_told() {
    let mut told = Told::default();
    step(&mut told, NOW, &[one("yonder", Reading::Ready, true)]);
    let unplugged = Seen {
        battery: true,
        ..lost("yonder", 120)
    };
    for minutes in (5..=30).step_by(5) {
        assert!(
            step(
                &mut told,
                NOW + minutes * 60,
                std::slice::from_ref(&unplugged)
            )
            .is_empty(),
            "a laptop its node last heard on battery was told at {minutes} min"
        );
    }
}

#[test]
fn a_realm_not_saying_who_is_busy_never_reads_another_realms_idle_as_its_own() {
    let mut told = Told::default();
    step(&mut told, NOW, &[one("yonder", Reading::Ready, true)]);
    let personal = Seen {
        realm: "personal".into(),
        working: None,
        ..one("yonder", Reading::Ready, false)
    };
    let both = ["work".to_owned(), "personal".to_owned()];
    due(
        &[one("yonder", Reading::Ready, false), personal],
        &both,
        &both,
        &mut told,
        NOW + 5 * 60,
    );
    assert!(
        told.machines["yonder"].working,
        "one realm's idle and another's unsaid read the machine idle"
    );
    // A word not said again past its time doesn't keep it busy for good.
    for minutes in [10, 15, 20, 25] {
        due(
            &[one("yonder", Reading::Ready, false), personal_unsaid()],
            &both,
            &both,
            &mut told,
            NOW + minutes * 60,
        );
    }
    assert!(
        !told.machines["yonder"].working,
        "a busy word never said again kept the machine busy"
    );
}

/// The machine as the personal realm reads it, ready, not saying who's busy.
fn personal_unsaid() -> Seen {
    Seen {
        realm: "personal".into(),
        working: None,
        ..one("yonder", Reading::Ready, false)
    }
}

#[test]
fn a_realm_not_answering_leaves_its_machines_unread_and_one_let_go_doesnt() {
    let mut told = Told::default();
    let personal = Seen {
        realm: "personal".into(),
        ..one("yonder", Reading::Ready, true)
    };
    let both = ["work".to_owned(), "personal".to_owned()];
    due(
        &[one("yonder", Reading::Down, false), personal],
        &both,
        &both,
        &mut told,
        NOW,
    );
    let read = [one("yonder", Reading::Down, false)];
    for minutes in (5..=30).step_by(5) {
        assert!(
            due(&read, &both, &work(), &mut told, NOW + minutes * 60).is_empty(),
            "a machine the realm reaching it didn't read was told at {minutes} min"
        );
    }
    // A realm no longer asked is let go, and the machine read again.
    let mut told = Told::default();
    due(
        &[one("yonder", Reading::Down, false), personal_ready()],
        &both,
        &both,
        &mut told,
        NOW,
    );
    due(&read, &work(), &work(), &mut told, NOW + 5 * 60);
    assert_eq!(
        due(&read, &work(), &work(), &mut told, NOW + 15 * 60).len(),
        1,
        "a realm no longer asked left its machine unread"
    );
    due(&[], &work(), &work(), &mut told, NOW + FORGET + 45 * 60);
    assert!(
        told.machines.is_empty(),
        "a machine no realm lists was kept"
    );
}

/// The machine as the personal realm reads it, ready and busy.
fn personal_ready() -> Seen {
    Seen {
        realm: "personal".into(),
        ..one("yonder", Reading::Ready, true)
    }
}

#[test]
fn a_tick_not_saying_who_is_busy_keeps_what_the_last_said() {
    let mut told = Told::default();
    step(&mut told, NOW, &[one("yonder", Reading::Ready, true)]);
    let unsaid = Seen {
        working: None,
        ..one("yonder", Reading::Ready, false)
    };
    step(&mut told, NOW + 5 * 60, &[unsaid]);
    let down = [one("yonder", Reading::Down, false)];
    step(&mut told, NOW + 10 * 60, &down);
    step(&mut told, NOW + 15 * 60, &down);
    assert_eq!(
        step(&mut told, NOW + 20 * 60, &down).len(),
        1,
        "a tick whose sessions went unread made a working machine idle"
    );
}

/// A machine working at `NOW` and unreachable since five minutes on.
fn gone() -> (Told, Vec<Tell>) {
    let mut told = Told::default();
    let down = [one("yonder", Reading::Down, false)];
    due(
        &[one("yonder", Reading::Ready, true)],
        &work(),
        &work(),
        &mut told,
        NOW,
    );
    due(&down, &work(), &work(), &mut told, NOW + 5 * 60);
    let tells = due(&down, &work(), &work(), &mut told, NOW + 15 * 60);
    (told, tells)
}

#[test]
fn a_going_down_is_told_to_joel_the_orchestrator_and_its_owner_each_once() {
    let (mut told, tells) = gone();
    let notices = RefCell::new(Vec::new());
    let findings = RefCell::new(Vec::new());
    let owner = |key: &str| (key == "down:yonder").then(|| "OWNER".to_owned());
    let notify = |one: &Tell| {
        notices.borrow_mut().push(format!(
            "{} {}: {}",
            one.urgency(),
            one.machine(),
            one.notice()
        ));
        Some(42)
    };
    let send = |to: &str, text: String| {
        findings.borrow_mut().push((to.to_owned(), text));
        if to == "OWNER" { Err("gone") } else { Ok(()) }
    };
    let result = tell(&tells, &mut told, "ORCH", owner, notify, send);
    assert_eq!(result, Err("gone"), "a send that failed wasn't said");
    assert_eq!(
        notices.take(),
        [
            "critical yonder: Can't be reached for 10 min, and its work waits until it's back. \
             Check it's awake and online."
        ],
        "Joel's notice wasn't as written"
    );
    let text = "Cadence alarm: yonder has read unreachable in the work realm for 10 min, after \
                reading ready within the hour before, on mains power with a session on it \
                busy. Joel has a notice of it.";
    assert_eq!(
        findings.take(),
        [
            ("ORCH".to_owned(), text.to_owned()),
            ("OWNER".to_owned(), text.to_owned())
        ],
        "the finding didn't go to both as written"
    );
    assert!(
        !told.machines["yonder"].told,
        "a going down not sent to all was kept as told"
    );
    let down = [one("yonder", Reading::Down, false)];
    let again = due(&down, &work(), &work(), &mut told, NOW + 20 * 60);
    let send = |to: &str, text: String| {
        findings.borrow_mut().push((to.to_owned(), text));
        Ok::<(), &str>(())
    };
    assert_eq!(
        tell(&again, &mut told, "ORCH", owner, notify, send),
        Ok(()),
        "a send failed"
    );
    let to: Vec<String> = findings.take().into_iter().map(|(to, _)| to).collect();
    assert_eq!(
        (to, notices.take().len()),
        (vec!["OWNER".to_owned()], 0),
        "a going down told again went beyond the one who missed it, or showed Joel it twice"
    );
    assert!(
        due(&down, &work(), &work(), &mut told, NOW + 25 * 60).is_empty(),
        "a going down told to all was told again"
    );
}

#[test]
fn a_coming_back_replaces_joels_notice_and_goes_to_each_who_had_it() {
    let back = Tell::Back {
        machine: "yonder".into(),
        minutes: 25,
        at_least: false,
        to: BTreeSet::from(["ORCH".to_owned(), "OWNER".to_owned()]),
        notice: Some(42),
    };
    let notices = RefCell::new(Vec::new());
    let findings = RefCell::new(Vec::new());
    let result: Result<(), ()> = tell(
        std::slice::from_ref(&back),
        &mut Told::default(),
        "ORCH",
        |_| None,
        |one| {
            notices
                .borrow_mut()
                .push(format!("{} {}", one.urgency(), one.notice()));
            None
        },
        |to, text| {
            findings.borrow_mut().push(format!("{to}: {text}"));
            Ok(())
        },
    );
    assert_eq!(result, Ok(()), "a send failed");
    assert_eq!(
        notices.take(),
        ["normal Back after 25 min, and its work carries on."],
        "Joel's notice of a machine back wasn't as written"
    );
    assert_eq!(
        findings.take(),
        [
            "ORCH: yonder reads ready again, after 25 min down.",
            "OWNER: yonder reads ready again, after 25 min down.",
        ],
        "a machine back wasn't told to each who had its going down"
    );
    let replaced = notice_args(&back);
    assert!(
        replaced.windows(2).any(|pair| pair == ["-r", "42"]) && replaced.contains(&"-p".into()),
        "a coming back wouldn't replace its going down's notice: {replaced:?}"
    );
    let (_, tells) = gone();
    assert!(
        !notice_args(&tells[0]).contains(&"-r".into()),
        "a going down would replace some other notice"
    );
}

#[test]
fn a_coming_back_after_a_gap_says_how_long_at_least() {
    let (mut told, tells) = gone();
    let fine = |_: &str, _: String| Ok::<(), ()>(());
    let sent = tell(&tells, &mut told, "ORCH", |_| None, |_| Some(7), fine);
    assert_eq!(sent, Ok(()), "a send that couldn't fail failed");
    // This machine slept from twenty minutes on, and wakes to it back.
    step(
        &mut told,
        NOW + 20 * 60,
        &[one("yonder", Reading::Down, false)],
    );
    let back = step(
        &mut told,
        NOW + 14 * 3600,
        &[one("yonder", Reading::Ready, false)],
    );
    assert_eq!(
        back.iter().map(Tell::notice).collect::<Vec<_>>(),
        ["Back after at least 15 min, and its work carries on."],
        "a coming back counted this machine's own sleep as time down"
    );
}

#[test]
fn a_notice_not_shown_is_tried_again_and_the_finding_says_so() {
    let (mut told, tells) = gone();
    let findings = RefCell::new(Vec::new());
    let send = |_: &str, text: String| {
        findings.borrow_mut().push(text);
        Ok::<(), ()>(())
    };
    assert_eq!(
        tell(&tells, &mut told, "ORCH", |_| None, |_| None, send),
        Ok(()),
        "a send failed"
    );
    assert!(
        findings.take()[0].ends_with("Joel's notice of it couldn't be shown."),
        "a finding said Joel had a notice that wasn't shown"
    );
    assert!(
        !told.machines["yonder"].told,
        "a going down Joel never saw was kept as told"
    );
    let down = [one("yonder", Reading::Down, false)];
    let again = due(&down, &work(), &work(), &mut told, NOW + 20 * 60);
    let shown = RefCell::new(0);
    let result = tell(
        &again,
        &mut told,
        "ORCH",
        |_| None,
        |_| {
            *shown.borrow_mut() += 1;
            Some(3)
        },
        send,
    );
    assert_eq!(
        (result, shown.take(), findings.take().len()),
        (Ok(()), 1, 0),
        "a notice not shown wasn't tried again, or its finding was sent twice"
    );
}

#[test]
fn a_notices_body_is_kept_from_markup_and_a_slow_node_is_stopped() {
    assert_eq!(
        escaped("<a href=\"x\">&</a>"),
        "&lt;a href=\"x\"&gt;&amp;&lt;/a&gt;",
        "a notice's body could be read as markup"
    );
    let shell = std::env::var("BASH").unwrap_or_else(|_| "bash".into());
    let slow = answer(
        &shell,
        &["-c", "while :; do :; done"],
        Duration::from_millis(300),
    );
    assert_eq!(
        slow.map_err(|err| err.kind()),
        Err(io::ErrorKind::TimedOut),
        "a node that wouldn't answer held the tick"
    );
    assert_eq!(
        answer(&shell, &["-c", "echo said"], Duration::from_secs(5)).ok(),
        Some("said\n".to_owned()),
        "a node's answer wasn't read"
    );
}
