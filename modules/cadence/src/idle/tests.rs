use super::*;

const NOW: i64 = 100_000;

/// One naming no step it can be woken to, as one waiting on an answer
/// or holding no quest is.
fn quiet(idle: i64) -> Quiet {
    Quiet {
        address: "8C7D8C31".into(),
        session: "s-1".into(),
        quest: Some("QEST-1".into()),
        orchestrated_by: Some("VP50PC".into()),
        listening: true,
        running: true,
        turned: Some(NOW - idle),
        since: Some(NOW - idle),
        started: None,
        unstarted: false,
        next: None,
        stepless: false,
        working: false,
        routes: false,
        starts: None,
    }
}

/// SEYNT2 on 18 November, in seconds: its last turn ended at 20:25, its
/// pi started again at 22:10, and it named no step while two alarm loops
/// of its own ran, one since 12:51.
fn seynt2() -> Quiet {
    Quiet {
        address: "SEYNT2".into(),
        session: "01a1d752".into(),
        quest: Some("QEST-20261113-SEYNT2".into()),
        orchestrated_by: Some("WCJQSG".into()),
        turned: Some(1_795_051_546),
        since: Some(1_795_051_546),
        started: Some(1_795_057_800),
        unstarted: true,
        stepless: true,
        working: true,
        ..quiet(0)
    }
}

#[test]
fn a_session_naming_no_step_on_its_quest_is_woken_to_it() {
    let mut told = Told::default();
    let s = Quiet {
        stepless: true,
        ..quiet(WAKE_AFTER - 60)
    };
    assert_eq!(
        due(std::slice::from_ref(&s), &mut told, NOW, clock),
        Vec::<Tell>::new(),
        "not before twenty minutes"
    );
    let tells = due(std::slice::from_ref(&s), &mut told, NOW + 60, clock);
    assert_eq!(tells.len(), 1, "woken at twenty minutes: {tells:?}");
    assert!(tells[0].wakes, "it is woken itself");
    assert_eq!(tells[0].to, "8C7D8C31", "to its own address");
    assert!(
        tells[0].text.starts_with(
            "8C7D8C31: this session has sat idle 0 h 20 min on QEST-1, naming no next step"
        ),
        "{}",
        tells[0].text
    );
    assert!(
        tells[0].text.contains(
            "Answer this with re, saying which. Send it to CADENCE: the mesh says that reaches \
             no peer, but cadence reads it from what you send."
        ),
        "its answer goes to cadence, though no peer is reached: {}",
        tells[0].text
    );
    assert!(
        tells[0]
            .text
            .contains("focus a plan whose next item says it"),
        "it is told how to be woken to a step: {}",
        tells[0].text
    );
    assert_eq!(
        due(std::slice::from_ref(&s), &mut told, NOW + 1800, clock),
        Vec::<Tell>::new(),
        "asked once"
    );
    let later = NOW - (WAKE_AFTER - 60) + IDLE_FOR;
    let tells = due(&[s], &mut told, later, clock);
    assert_eq!(
        tells.len(),
        1,
        "its orchestrator hears at two hours: {tells:?}"
    );
    assert!(!tells[0].wakes, "{tells:?}");
    assert!(
        tells[0]
            .text
            .contains("asked at t100060 to take up its quest"),
        "{}",
        tells[0].text
    );
}

#[test]
fn background_work_holds_back_a_wake_an_hour_at_most() {
    let mut told = Told::default();
    let s = Quiet {
        working: true,
        ..with_next(WORKS_AT_MOST - 60)
    };
    assert_eq!(
        due(std::slice::from_ref(&s), &mut told, NOW, clock),
        Vec::<Tell>::new(),
        "its work's end will wake it within the hour"
    );
    let tells = due(&[s], &mut told, NOW + 60, clock);
    assert_eq!(tells.len(), 1, "woken past the hour: {tells:?}");
    assert!(tells[0].wakes, "{tells:?}");
    assert!(
        tells[0]
            .text
            .contains("background work of its own has run all that while"),
        "it isn't told its work still runs: {}",
        tells[0].text
    );
}

#[test]
fn one_woken_to_its_quest_is_asked_again_only_after_two_hours() {
    let mut told = Told::default();
    // Idle from `at`, its last turn's end.
    let from = |at| Quiet {
        turned: Some(at),
        since: Some(at),
        ..quiet(0)
    };
    let stepless = |at| Quiet {
        stepless: true,
        ..from(at)
    };
    assert_eq!(
        due(&[stepless(NOW - WAKE_AFTER)], &mut told, NOW, clock).len(),
        1,
        "woken to its quest"
    );
    // It answers, naming a step, and is woken to that step.
    let stepped = Quiet {
        next: Some("Merge #12".into()),
        ..from(NOW + 600)
    };
    assert_eq!(
        due(&[stepped], &mut told, NOW + 600 + WAKE_AFTER, clock).len(),
        1,
        "woken to a real step"
    );
    // It answers again, naming none, in a spell of its own.
    let again = NOW + 900 + WAKE_AFTER;
    assert_eq!(
        due(&[stepless(again)], &mut told, again + WAKE_AFTER, clock),
        Vec::<Tell>::new(),
        "going back and forth between a step and none woke it to its quest again"
    );
    let tells = due(&[stepless(again)], &mut told, NOW + IDLE_FOR, clock);
    assert_eq!(tells.len(), 1, "two hours on, it is asked again: {tells:?}");
    assert!(tells[0].wakes, "{tells:?}");
}

#[test]
fn seynt2_resumed_with_alarms_running_is_woken() {
    let mut told = Told::default();
    // 22:15, five minutes after its pi started again.
    let tells = due(&[seynt2()], &mut told, 1_795_058_100, clock);
    assert_eq!(tells.len(), 1, "{tells:?}");
    assert!(tells[0].wakes, "woken itself, not told of: {tells:?}");
    assert!(
        tells[0]
            .text
            .starts_with("SEYNT2: this session has sat idle 1 h 49 min on QEST-20261113-SEYNT2"),
        "{}",
        tells[0].text
    );
    let mut told = Told::default();
    let soon = Quiet {
        turned: Some(1_795_058_100 - 40 * 60),
        since: Some(1_795_058_100 - 40 * 60),
        ..seynt2()
    };
    assert_eq!(
        due(&[soon], &mut told, 1_795_058_100, clock),
        Vec::<Tell>::new(),
        "forty minutes into its alarms, their end still wakes it"
    );
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
fn a_cell_of_a_quest_that_routes_is_never_woken_or_told_of() {
    let mut told = Told::default();
    // As 8GYNJ6, the void, on 19 November: idle by design on its quest.
    let void = |idle| Quiet {
        routes: true,
        stepless: true,
        ..quiet(idle)
    };
    assert_eq!(
        due(&[void(WAKE_AFTER)], &mut told, NOW, clock),
        Vec::<Tell>::new(),
        "woken at twenty minutes"
    );
    assert_eq!(
        due(&[void(IDLE_FOR + 3600)], &mut told, NOW, clock),
        Vec::<Tell>::new(),
        "told of at two hours"
    );
    let readme = |front: &str| format!("---\nid: QEST-1\n{front}status: active\n---\n# Q\n");
    assert!(
        routes(&readme("hears: addressed\n")),
        "the harness's mark unread"
    );
    assert!(
        routes(&readme("hears: \"addressed\"\n")),
        "the mark quoted unread"
    );
    assert!(!routes(&readme("")), "a quest with no mark read as routing");
    assert!(
        !routes("# Q\n\nhears: addressed\n"),
        "a body's line read as its frontmatter"
    );
}

#[test]
fn a_wake_before_a_steps_start_still_leaves_it_woken_once_it_starts() {
    // As F8TT8O on 19 November: woken at 05:15 to a step starting 09:00,
    // before its start was read.
    let mut told = Told::default();
    let s = Quiet {
        starts: Some(NOW + 3600),
        ..with_next(3 * 3600)
    };
    told.woken.insert(
        "s-1".into(),
        Woken {
            since: NOW - 3 * 3600,
            at: NOW - 600,
            next: "Merge #12 once its gate passes".into(),
            to_quest: None,
        },
    );
    let tells = due(
        std::slice::from_ref(&s),
        &mut told,
        NOW + 3600 + WAKE_AFTER,
        clock,
    );
    assert!(
        tells.len() == 1 && tells[0].wakes,
        "not woken past its start for an earlier wake: {tells:?}"
    );
}

#[test]
fn a_step_dated_ahead_counts_no_idle_time_until_it_starts() {
    let mut told = Told::default();
    // Idle three hours, its step starting an hour from now.
    let s = Quiet {
        starts: Some(NOW + 3600),
        ..with_next(3 * 3600)
    };
    let sessions = std::slice::from_ref(&s);
    assert_eq!(
        due(sessions, &mut told, NOW, clock),
        Vec::<Tell>::new(),
        "woken, or told of, before its step starts"
    );
    assert_eq!(
        due(sessions, &mut told, NOW + 3600 + WAKE_AFTER - 60, clock),
        Vec::<Tell>::new(),
        "woken under twenty minutes past its start"
    );
    let tells = due(sessions, &mut told, NOW + 3600 + WAKE_AFTER, clock);
    assert!(
        tells.len() == 1 && tells[0].wakes,
        "woken twenty minutes past its start: {tells:?}"
    );
    assert!(
        tells[0].text.contains(&format!(
            "has sat idle 0 h 20 min with a next step of its own, \
             \"Merge #12 once its gate passes\": its next step said it starts at t{}.",
            NOW + 3600
        )),
        "its spell said as a turn's end: {}",
        tells[0].text
    );
    assert!(
        told_to_orchestrator(&due(sessions, &mut told, NOW + 3600 + IDLE_FOR - 60, clock))
            .is_empty(),
        "its orchestrator told under two hours past its start"
    );
    assert_eq!(
        told_to_orchestrator(&due(sessions, &mut told, NOW + 3600 + IDLE_FOR, clock)).len(),
        1,
        "its orchestrator told two hours past its start"
    );
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
    // As `Quiet::from` reads one that never turned: its spell from its
    // pi's start.
    let s = Quiet {
        turned: None,
        started: Some(NOW - IDLE_FOR),
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
         If it waits on someone or on its work, park saying on what; if it is another's, \
         hand it to them by name. Answer this with re, saying which. Send it to CADENCE: the \
         mesh says that reaches no peer, but cadence reads it from what you send. VP50PC hears \
         of it at two hours idle."
    );
    assert!(
        due(&[with_next(WAKE_AFTER)], &mut told, NOW + 600, clock).is_empty(),
        "a spell is woken once"
    );
}

#[test]
fn a_step_is_asked_of_once_and_its_orchestrator_hears_when_turns_dont_move_it() {
    let mut told = Told::default();
    assert_eq!(
        due(&[with_next(WAKE_AFTER)], &mut told, NOW, clock).len(),
        1,
        "asked first"
    );
    // It answers, a turn of its own, and sits with the same step.
    let answered = NOW + 60;
    let later = || Quiet {
        turned: Some(answered),
        since: Some(answered),
        ..with_next(0)
    };
    assert!(
        due(&[later()], &mut told, answered + WAKE_AFTER + 60, clock).is_empty(),
        "the same step asked of again"
    );
    let tells = due(&[later()], &mut told, answered + IDLE_FOR, clock);
    assert_eq!(tells.len(), 1, "{tells:?}");
    assert!(!tells[0].wakes, "woken again rather than told of");
    assert!(
        tells[0].text.contains(
            " It was asked at t100000 to take up its next step, and the turns since \
             haven't moved it."
        ),
        "{}",
        tells[0].text
    );
    let moved = Quiet {
        next: Some("Rebase #12 onto main".into()),
        ..later()
    };
    let tells = due(&[moved], &mut told, answered + IDLE_FOR + 300, clock);
    assert!(
        tells.len() == 1 && tells[0].wakes,
        "a new step wasn't asked of: {tells:?}"
    );
}

#[test]
fn one_first_seen_past_two_hours_is_asked_before_its_orchestrator_hears() {
    let mut told = Told::default();
    let s = with_next(IDLE_FOR * 2);
    let tells = due(std::slice::from_ref(&s), &mut told, NOW, clock);
    assert!(
        tells.len() == 1 && tells[0].wakes,
        "not asked alone first: {tells:?}"
    );
    assert!(
        due(
            std::slice::from_ref(&s),
            &mut told,
            NOW + WAKE_AFTER - 1,
            clock
        )
        .is_empty(),
        "its orchestrator heard before the question had time to take"
    );
    let s = Quiet {
        since: Some(NOW - IDLE_FOR * 2),
        ..s
    };
    let tells = due(&[s], &mut told, NOW + WAKE_AFTER, clock);
    assert!(
        tells.len() == 1 && !tells[0].wakes && tells[0].text.contains("has taken no turn since"),
        "its orchestrator didn't hear once the question went unanswered: {tells:?}"
    );
}

#[test]
fn a_spell_across_midnight_or_a_resumed_log_is_asked_and_told_once() {
    let mut told = Told::default();
    let s = with_next(IDLE_FOR);
    let first = due(std::slice::from_ref(&s), &mut told, NOW, clock);
    assert_eq!(first.len(), 1, "asked: {first:?}");
    let after = NOW + WAKE_AFTER;
    assert_eq!(
        due(std::slice::from_ref(&s), &mut told, after, clock).len(),
        1,
        "told of"
    );
    // Past midnight its presence names no turn, and its log was written
    // later, at the turn's end or by a host resuming it.
    let day_began = Quiet {
        turned: None,
        since: Some(NOW - IDLE_FOR + 5),
        ..s
    };
    assert!(
        due(&[day_began], &mut told, after + 300, clock).is_empty(),
        "the same spell was asked or told of again"
    );
}

#[test]
fn a_line_that_could_not_be_sent_is_said_again() {
    let mut told = Told::default();
    let tells = due(&[with_next(IDLE_FOR)], &mut told, NOW, clock);
    unsay(&mut told, &tells[0]);
    assert_eq!(
        due(&[with_next(IDLE_FOR)], &mut told, NOW + 300, clock).len(),
        1,
        "a wake not sent wasn't sent again"
    );
    let tells = due(&[quiet(IDLE_FOR)], &mut told, NOW, clock);
    assert!(!tells[0].wakes, "{tells:?}");
    unsay(&mut told, &tells[0]);
    assert_eq!(
        due(&[quiet(IDLE_FOR)], &mut told, NOW + 300, clock).len(),
        1,
        "a line for an orchestrator not sent wasn't sent again"
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
    assert_eq!(tells.len(), 1, "asked again, or not told of: {tells:?}");
    let told_of = told_to_orchestrator(&tells);
    assert_eq!(told_of.len(), 1, "{tells:?}");
    assert!(
        told_of[0].text.contains(
            " It was asked at t100000 to take up its next step, and has taken no turn since."
        ),
        "{}",
        told_of[0].text
    );
}

#[test]
fn one_cut_by_a_host_stop_is_woken_once_its_host_is_back() {
    // 5HJZ8Z on 18 November: its turn cut at 07:40 by the host's stop, its
    // host back at 09:12, and nothing said to it until 10:50.
    let cut = NOW - 100 * 60;
    let resumed = NOW - 8 * 60;
    let s = Quiet {
        turned: Some(cut),
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
    assert!(
        q.working,
        "its background work is kept, to hold back a wake"
    );
    assert_eq!(
        q.next.as_deref(),
        Some("Merge #12"),
        "its step is kept while it works"
    );
    assert!(!q.stepless, "it names a step");

    let other_boot =
        Quiet::from(&started_later, None, Some("c"), (false, false), |_| true).expect("one");
    assert!(!other_boot.running, "another boot's pi doesn't run");
    let parked =
        Quiet::from(&started_later, None, Some("b"), (true, false), |_| true).expect("one");
    assert!(!parked.listening, "a parked one isn't listening");
}

#[test]
fn a_step_waiting_on_joel_or_on_an_answer_is_none_to_wake_to() {
    let mut joel = presence(Some("1970-01-12T13:46:40Z"), 900_000_000);
    joel.doing = serde_json::json!({"next": "Drill with Joel at his work MacBook"});
    let q = Quiet::from(&joel, None, Some("b"), (false, false), |_| true).expect("one");
    assert_eq!(q.next, None, "a step only Joel can move");
    let mut asked = presence(Some("1970-01-12T13:46:40Z"), 900_000_000);
    asked.wanting = serde_json::json!({"id": "2f95756a"});
    let q = Quiet::from(&asked, None, Some("b"), (false, false), |_| true).expect("one");
    assert_eq!(q.next, None, "a step waiting on the answer it asked for");
    asked.doing = serde_json::Value::Null;
    asked.quest = Some("QEST-1".into());
    let q = Quiet::from(&asked, None, Some("b"), (false, false), |_| true).expect("one");
    assert!(
        !q.stepless,
        "one waiting on an answer isn't woken to its quest"
    );
}

#[test]
fn one_naming_no_step_is_stepless_only_on_a_quest() {
    let mut none = presence(Some("1970-01-12T13:46:40Z"), 900_000_000);
    none.doing = serde_json::Value::Null;
    none.quest = Some("QEST-1".into());
    let q = Quiet::from(&none, None, Some("b"), (false, false), |_| true).expect("one");
    assert!(
        q.stepless && q.next.is_none(),
        "on a quest, naming no step: {q:?}"
    );
    for quest in [None, Some(String::new())] {
        none.quest = quest;
        let q = Quiet::from(&none, None, Some("b"), (false, false), |_| true).expect("one");
        assert!(!q.stepless, "with no quest there's nothing to wake it to");
    }
    let mut joel = presence(Some("1970-01-12T13:46:40Z"), 900_000_000);
    joel.quest = Some("QEST-1".into());
    joel.doing = serde_json::json!({"next": "Drill with Joel at his work MacBook"});
    let q = Quiet::from(&joel, None, Some("b"), (false, false), |_| true).expect("one");
    assert!(!q.stepless, "a step only Joel can move isn't no step");
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
