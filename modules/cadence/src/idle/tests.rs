use super::*;

const NOW: i64 = 100_000;

fn quiet(idle: i64) -> Quiet {
    Quiet {
        address: "8C7D8C31".into(),
        session: "s-1".into(),
        quest: Some("QEST-1".into()),
        orchestrated_by: Some("VP50PC".into()),
        listening: true,
        running: true,
        since: Some(NOW - idle),
        started: None,
        unstarted: false,
        next: None,
    }
}

fn with_next(idle: i64) -> Quiet {
    Quiet {
        next: Some("Merge #12 once its gate passes".into()),
        ..quiet(idle)
    }
}

fn clock(at: i64) -> String {
    format!("t{at}")
}

fn told_to_orchestrator(tells: &[Tell]) -> Vec<&Tell> {
    tells.iter().filter(|t| !t.wakes).collect()
}

#[test]
fn a_session_idle_two_hours_is_told_to_its_orchestrator_once() {
    let mut told = Told::default();
    let sessions = [quiet(IDLE_FOR + 5 * 60)];
    let tells = due(&sessions, &mut told, NOW, clock);
    assert_eq!(tells.len(), 1, "{tells:?}");
    assert_eq!(tells[0].to, "VP50PC", "told to its orchestrator");
    assert!(!tells[0].wakes, "one with no next step isn't woken");
    assert!(
        tells[0]
            .text
            .starts_with("8C7D8C31 on QEST-1 has sat idle 2 h 05 min: its last turn ended at"),
        "{}",
        tells[0].text
    );
    assert!(tells[0].text.contains("(session s-1)"), "{}", tells[0].text);
    assert_eq!(
        due(&sessions, &mut told, NOW + 3600, clock),
        Vec::<Tell>::new(),
        "a spell is told once"
    );
}

#[test]
fn a_new_turn_starts_a_new_spell() {
    let mut told = Told::default();
    assert_eq!(
        due(&[quiet(IDLE_FOR)], &mut told, NOW, clock).len(),
        1,
        "the first spell is told"
    );
    let again = quiet(IDLE_FOR - 60);
    assert_eq!(
        due(std::slice::from_ref(&again), &mut told, NOW, clock),
        Vec::<Tell>::new(),
        "a spell under two hours isn't"
    );
    assert_eq!(
        due(&[again], &mut told, NOW + 60, clock).len(),
        1,
        "the new spell is told at two hours"
    );
}

#[test]
fn one_that_never_turned_since_its_start_says_so() {
    let mut told = Told::default();
    let s = Quiet {
        unstarted: true,
        ..quiet(IDLE_FOR)
    };
    let tells = due(&[s], &mut told, NOW, clock);
    assert!(
        tells[0]
            .text
            .contains("its pi started at t92800 and has taken no turn since"),
        "{}",
        tells[0].text
    );
}

#[test]
fn one_resumed_by_a_restart_is_idle_from_its_last_turn() {
    let mut told = Told::default();
    let s = Quiet {
        started: Some(NOW - 600),
        unstarted: true,
        ..quiet(IDLE_FOR)
    };
    let tells = due(&[s], &mut told, NOW, clock);
    assert_eq!(tells.len(), 1, "told though its pi started ten minutes ago");
    assert!(
        tells[0].text.contains(
            "its last turn ended at t92800, before its pi started again at t99400, with none since"
        ),
        "{}",
        tells[0].text
    );
}

#[test]
fn busy_parked_dead_unled_and_recent_sessions_are_not_told() {
    let long = IDLE_FOR * 3;
    let cases = [
        quiet(IDLE_FOR - 1),
        Quiet {
            listening: false,
            ..with_next(long)
        },
        Quiet {
            running: false,
            ..with_next(long)
        },
        Quiet {
            orchestrated_by: None,
            ..with_next(long)
        },
        Quiet {
            orchestrated_by: Some("8c7d8c31".into()),
            ..with_next(long)
        },
        Quiet {
            since: None,
            ..with_next(long)
        },
    ];
    for case in cases {
        let mut told = Told::default();
        assert!(
            due(std::slice::from_ref(&case), &mut told, NOW, clock).is_empty(),
            "{case:?}"
        );
    }
}

#[test]
fn one_with_a_next_step_is_woken_itself_at_twenty_minutes_once() {
    let mut told = Told::default();
    assert!(
        due(&[with_next(WAKE_AFTER - 1)], &mut told, NOW, clock).is_empty(),
        "not before twenty minutes"
    );
    let tells = due(&[with_next(WAKE_AFTER)], &mut told, NOW, clock);
    assert_eq!(tells.len(), 1, "{tells:?}");
    assert!(tells[0].wakes, "a wake, not a line for its orchestrator");
    assert_eq!(tells[0].to, "8C7D8C31", "sent to the session itself");
    assert_eq!(
        tells[0].text,
        "8C7D8C31: this session has sat idle 0 h 20 min with a next step of its own, \
         \"Merge #12 once its gate passes\": its last turn ended at t98800. Take it up now. \
         If it waits on someone, park saying on whom; if it is another's, hand it to them \
         by name. VP50PC hears of it at two hours idle."
    );
    assert!(
        due(&[with_next(WAKE_AFTER)], &mut told, NOW + 600, clock).is_empty(),
        "a spell is woken once"
    );
}

#[test]
fn one_woken_and_still_idle_at_two_hours_is_told_so() {
    let mut told = Told::default();
    assert_eq!(
        due(&[with_next(WAKE_AFTER)], &mut told, NOW, clock).len(),
        1,
        "woken first"
    );
    let later = NOW + IDLE_FOR;
    let s = Quiet {
        since: Some(NOW - WAKE_AFTER),
        ..with_next(0)
    };
    let tells = due(&[s], &mut told, later, clock);
    let told_of = told_to_orchestrator(&tells);
    assert_eq!(told_of.len(), 1, "{tells:?}");
    assert!(
        told_of[0]
            .text
            .contains(" It was woken at t100000 and has taken no turn since."),
        "{}",
        told_of[0].text
    );
}

#[test]
fn one_cut_by_a_host_stop_is_woken_at_the_first_tick_after_its_host_is_back() {
    // 5HJZ8Z on 18 November: its turn cut at 07:40 by the host's stop, its
    // host back at 09:12, and nothing said to it until 10:50.
    let cut = NOW - 100 * 60;
    let resumed = NOW - 8 * 60;
    let s = Quiet {
        since: Some(cut),
        started: Some(resumed),
        unstarted: true,
        ..with_next(0)
    };
    let mut told = Told::default();
    let tells = due(&[s], &mut told, NOW, clock);
    assert_eq!(tells.len(), 1, "{tells:?}");
    assert!(tells[0].wakes, "woken itself, not only told of");
    assert!(
        tells[0].text.contains("idle 1 h 40 min")
            && tells[0]
                .text
                .contains("before its pi started again at t99520"),
        "{}",
        tells[0].text
    );
}

#[test]
fn a_restart_full_of_idle_sessions_wakes_the_longest_idle_first() {
    let sessions: Vec<Quiet> = (0..6_i64)
        .map(|i| Quiet {
            address: format!("A{i}"),
            session: format!("s-{i}"),
            ..with_next(WAKE_AFTER + i * 60)
        })
        .collect();
    let mut told = Told::default();
    let woken: Vec<String> = due(&sessions, &mut told, NOW, clock)
        .into_iter()
        .map(|t| t.to)
        .collect();
    assert_eq!(
        woken,
        ["A5", "A4", "A3", "A2"],
        "at most four, longest idle first"
    );
    let rest: Vec<String> = due(&sessions, &mut told, NOW + 300, clock)
        .into_iter()
        .map(|t| t.to)
        .collect();
    assert_eq!(rest, ["A1", "A0"], "the rest a tick later");
}

fn presence(turned: Option<&str>, started_ms: i64) -> Presence {
    let usage = turned.map_or_else(
        || serde_json::json!({"day": "1970-01-13", "turns": {"today": 0}}),
        |at| serde_json::json!({"turns": {"last": {"at": at, "duration_ms": 60_000}}}),
    );
    serde_json::from_value(serde_json::json!({
        "session": "s-1",
        "addresses": ["8C7D8C31"],
        "state": "listening",
        "orchestratedBy": "VP50PC",
        "owner": {"pid": 7, "startedAt": started_ms, "bootId": "b"},
        "usage": usage,
        "doing": {"next": "  Merge #12  "},
    }))
    .expect("a presence")
}

#[test]
fn a_presence_counts_from_its_last_turn_across_a_restart() {
    // 1_000_000 s is 1970-01-12T13:46:40Z.
    let turned_later = presence(Some("1970-01-12T13:46:40Z"), 900_000_000);
    let seven = |p: &Presence| p.owner.as_ref().and_then(|o| o.pid) == Some(7);
    let q = Quiet::from(&turned_later, None, Some("b"), (false, false), seven).expect("one");
    assert_eq!(q.since, Some(1_000_060), "from its last turn's end");
    assert!(!q.unstarted, "it turned after its start");
    assert!(q.running && q.listening, "{q:?}");
    assert_eq!(
        q.next.as_deref(),
        Some("Merge #12"),
        "its next step, trimmed"
    );

    let started_later = presence(Some("1970-01-12T13:46:40Z"), 1_100_000_000);
    // A host writes the log of each conversation it resumes.
    let q = Quiet::from(
        &started_later,
        Some(1_100_005),
        Some("b"),
        (false, true),
        |_| true,
    )
    .expect("one");
    assert_eq!(q.since, Some(1_000_060), "a restart begins no spell");
    assert_eq!(q.started, Some(1_100_000), "its start is kept to say");
    assert!(q.unstarted, "no turn since its start");
    assert_eq!(
        q.next, None,
        "its background work's end wakes it, not cadence"
    );

    let other_boot =
        Quiet::from(&started_later, None, Some("c"), (false, false), |_| true).expect("one");
    assert!(!other_boot.running, "another boot's pi doesn't run");
    let parked =
        Quiet::from(&started_later, None, Some("b"), (true, false), |_| true).expect("one");
    assert!(!parked.listening, "a parked one isn't listening");
}

#[test]
fn a_presence_whose_day_began_again_counts_from_its_log() {
    // Past midnight a presence's usage holds no last turn.
    let p = presence(None, 1_000_000_000);
    let q = Quiet::from(&p, Some(1_005_100), Some("b"), (false, false), |_| true).expect("one");
    assert_eq!(q.since, Some(1_005_100), "from its log");
    assert!(!q.unstarted, "it took turns after its start");
    let q = Quiet::from(&p, None, Some("b"), (false, false), |_| true).expect("one");
    assert_eq!(q.since, Some(1_000_000), "from its start, with no log");
    assert!(q.unstarted, "no turn since its start");
}
