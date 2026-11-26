//! The tests of the timers as they're kept: their spans, pauses and rings,
//! the stopwatch and its laps, the asks read from a line, and what's kept.

use super::*;

const MINUTE: u64 = 60_000;

fn words() -> Words {
    Words {
        app: "Timers".to_owned(),
        unnamed: "Timer".to_owned(),
        rang: "{span} timer".to_owned(),
        late: "{span} timer, rang late".to_owned(),
        again: "Again".to_owned(),
        hours: "h".to_owned(),
        minutes: "min".to_owned(),
        seconds: "s".to_owned(),
    }
}

/// The reason an ask was refused as unfit, or nothing where it wasn't, so
/// a failing test shows what came back.
fn unfit<T>(answer: Result<T, Refusal>) -> Option<&'static str> {
    match answer.err() {
        Some(Refusal::Unfit { reason, .. }) => Some(reason),
        _ => None,
    }
}

#[test]
fn a_timer_started_counts_its_span_from_now_and_ends_then() {
    let mut held = Held::default();
    let timer = held.start(" tea ", 10 * MINUTE, 1_000).unwrap().clone();
    assert_eq!(timer.key, "timer:1");
    assert_eq!(timer.name, "tea");
    assert_eq!(timer.standing, Standing::Running { ends: 601_000 });
    assert_eq!(held.next_end(), Some(601_000));
    assert_eq!(held.start("eggs", MINUTE, 2_000).unwrap().key, "timer:2");
    assert_eq!(held.next_end(), Some(62_000));
}

#[test]
fn a_span_of_nothing_or_past_a_week_is_refused() {
    let mut held = Held::default();
    assert_eq!(unfit(held.start("", 0, 0)), Some("nothing"));
    assert_eq!(unfit(held.start("", LONGEST + 1, 0)), Some("pastAWeek"));
    assert_eq!(held.timers, Vec::<Timer>::new());
}

#[test]
fn a_long_name_is_cut_to_what_it_keeps() {
    let mut held = Held::default();
    let name = "é".repeat(NAMED + 10);
    assert_eq!(
        held.start(&name, MINUTE, 0).unwrap().name.chars().count(),
        NAMED
    );
}

#[test]
fn a_paused_timer_holds_what_it_had_left_and_resumes_from_there() {
    let mut held = Held::default();
    held.start("tea", 10 * MINUTE, 0).unwrap();
    let paused = held.pause("timer:1", 4 * MINUTE).unwrap().clone();
    assert_eq!(paused.standing, Standing::Paused { left: 6 * MINUTE });
    assert_eq!(paused.left(100 * MINUTE), 6 * MINUTE);
    assert_eq!(held.next_end(), None);
    assert_eq!(held.due(100 * MINUTE), Vec::<String>::new());
    let resumed = held.resume("timer:1", 20 * MINUTE).unwrap();
    assert_eq!(resumed.standing, Standing::Running { ends: 26 * MINUTE });
}

#[test]
fn pausing_or_resuming_what_doesnt_fit_is_refused() {
    let mut held = Held::default();
    held.start("tea", MINUTE, 0).unwrap();
    assert_eq!(unfit(held.resume("timer:1", 0)), Some("resumesPaused"));
    assert_eq!(
        held.pause("timer:9", 0),
        Err(Refusal::Unknown("timer:9".to_owned()))
    );
}

#[test]
fn a_timer_coming_due_is_heard_only_within_a_minute_of_its_end() {
    let mut held = Held::default();
    assert!(!held.heard(MINUTE), "no timer, yet one was heard");
    held.start("tea", MINUTE, 0).unwrap();
    assert!(!held.heard(MINUTE - 1), "heard before its end");
    assert!(held.heard(MINUTE), "not heard at its end");
    assert!(
        held.heard(MINUTE + HEARD),
        "not heard at the last of its minute"
    );
    assert!(
        !held.heard(MINUTE + HEARD + 1),
        "heard past the minute after its end"
    );
    // One heard among those due is enough to sound the ring once.
    held.start("eggs", MINUTE + HEARD, 0).unwrap();
    assert!(
        held.heard(MINUTE + HEARD + 1),
        "one of two due within its minute, yet neither heard"
    );
    held.due(MINUTE + HEARD + 1);
    assert!(
        !held.heard(MINUTE + HEARD + 1),
        "heard again once both had rung"
    );
}

#[test]
fn a_timer_rings_once_its_end_passes_saying_whether_it_was_late() {
    let mut held = Held::default();
    held.start("tea", MINUTE, 0).unwrap();
    held.start("eggs", 2 * MINUTE, 0).unwrap();
    assert_eq!(held.due(MINUTE - 1), Vec::<String>::new());
    assert_eq!(held.due(MINUTE), vec!["timer:1".to_owned()]);
    assert_eq!(
        held.get("timer:1").unwrap().standing,
        Standing::Rung {
            rang: MINUTE,
            late: false
        }
    );
    assert_eq!(held.due(MINUTE + 1), Vec::<String>::new());
    assert_eq!(held.due(10 * MINUTE), vec!["timer:2".to_owned()]);
    assert_eq!(
        held.get("timer:2").unwrap().standing,
        Standing::Rung {
            rang: 10 * MINUTE,
            late: true
        }
    );
    let here = on_bus("b1", ":1.4");
    assert_eq!(held.unsaid(&here), vec!["timer:1", "timer:2"]);
    held.timers[0].notice = Some(Notice {
        id: 7,
        server: here.clone(),
    });
    assert_eq!(held.unsaid(&here), vec!["timer:2"]);
    // Said to a server that's gone, it's unsaid to the one after it.
    assert_eq!(
        held.unsaid(&on_bus("b1", ":1.9")),
        vec!["timer:1", "timer:2"]
    );
    // And to one on a bus begun again, though it has the same name.
    let reborn = on_bus("b2", ":1.4");
    assert_eq!(held.unsaid(&reborn), vec!["timer:1", "timer:2"]);
    assert_eq!(held.noticed(7, &here).unwrap().key, "timer:1");
    assert_eq!(held.noticed(7, &on_bus("b1", ":1.9")), None);
    assert_eq!(
        held.noticed(7, &reborn),
        None,
        "a reborn bus's name isn't the server's"
    );
    assert_eq!(held.next_end(), None);
}

#[test]
fn again_sets_it_going_for_its_whole_span_however_it_stands() {
    let mut held = Held::default();
    held.start("tea", MINUTE, 0).unwrap();
    held.due(MINUTE);
    held.timers[0].notice = Some(Notice {
        id: 7,
        server: on_bus("b1", ":1.4"),
    });
    let again = held.again("timer:1", 5 * MINUTE).unwrap();
    assert_eq!(again.standing, Standing::Running { ends: 6 * MINUTE });
    assert_eq!(again.set, 5 * MINUTE);
    assert_eq!(again.notice, None);
}

#[test]
fn adding_lengthens_one_counting_and_sets_a_rung_one_going() {
    let mut held = Held::default();
    held.start("tea", MINUTE, 0).unwrap();
    let added = held.add("timer:1", MINUTE, 30_000).unwrap().clone();
    assert_eq!(added.standing, Standing::Running { ends: 2 * MINUTE });
    assert_eq!(added.span, 2 * MINUTE);
    held.pause("timer:1", MINUTE).unwrap();
    let paused = held.add("timer:1", MINUTE, MINUTE).unwrap().clone();
    assert_eq!(paused.standing, Standing::Paused { left: 2 * MINUTE });
    held.resume("timer:1", MINUTE).unwrap();
    held.due(3 * MINUTE);
    let again = held.add("timer:1", MINUTE, 4 * MINUTE).unwrap();
    assert_eq!(again.standing, Standing::Running { ends: 5 * MINUTE });
    assert_eq!(again.span, MINUTE);
}

#[test]
fn ending_takes_it_away_and_an_unknown_one_is_refused() {
    let mut held = Held::default();
    held.start("tea", MINUTE, 0).unwrap();
    assert_eq!(held.end("timer:1").unwrap().name, "tea");
    assert_eq!(held.timers, Vec::<Timer>::new());
    assert_eq!(
        held.end("timer:1"),
        Err(Refusal::Unknown("timer:1".to_owned()))
    );
    // A key isn't given twice, even once the one holding it has gone.
    assert_eq!(held.start("", MINUTE, 0).unwrap().key, "timer:2");
}

#[test]
fn what_is_kept_reads_back_as_it_was() {
    let folder = std::env::temp_dir().join(format!("timers-kept-{}", std::process::id()));
    let path = folder.join("held.json");
    let mut held = Held::default();
    held.start("tea", MINUTE, 0).unwrap();
    held.start("eggs", MINUTE, 0).unwrap();
    held.pause("timer:2", 1_000).unwrap();
    held.stopwatch.run(0).unwrap();
    held.stopwatch.lap(500).unwrap();
    held.write(&path).unwrap();
    assert_eq!(Held::read(&path), held);
    assert_eq!(Held::read(&folder.join("none.json")), Held::default());
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn a_record_says_how_it_stands_in_the_interfaces_fields() {
    let mut held = Held::default();
    held.start("tea", MINUTE, 0).unwrap();
    assert_eq!(
        held.timers[0].json(),
        json!({"key": "timer:1", "kind": "timer", "name": "tea", "span": MINUTE,
            "set": 0, "state": "running", "ends": MINUTE})
    );
    held.due(MINUTE);
    assert_eq!(held.timers[0].json()["state"], "rung");
    assert_eq!(held.timers[0].json()["late"], false);
}

#[test]
fn a_notice_names_the_timer_and_says_its_span_in_its_largest_units() {
    let words = words();
    let mut held = Held::default();
    held.start("", 90 * MINUTE, 0).unwrap();
    held.start("tea", 10 * MINUTE, 0).unwrap();
    held.due(10 * MINUTE);
    assert_eq!(
        words.notice(&held.timers[1]),
        ("Tea".to_owned(), "10 min timer".to_owned())
    );
    held.due(100 * MINUTE);
    assert_eq!(
        words.notice(&held.timers[0]),
        ("Timer".to_owned(), "1 h 30 min timer, rang late".to_owned())
    );
    assert_eq!(words.span(90_000), "1 min 30 s");
    assert_eq!(words.span(1), "1 s");
}

#[test]
fn an_ask_is_read_with_what_it_needs_or_refused() {
    assert_eq!(
        ask(r#"{"id":1,"ask":"start","span":600000,"name":"tea"}"#),
        Ask::Start {
            id: json!(1),
            name: "tea".to_owned(),
            span: 600_000
        }
    );
    assert_eq!(
        ask(r#"{"id":2,"ask":"pause","key":"timer:1"}"#),
        Ask::Pause {
            id: json!(2),
            key: "timer:1".to_owned()
        }
    );
    assert!(
        matches!(ask(r#"{"id":3,"ask":"pause"}"#), Ask::Malformed { .. }),
        "a pause with no key wasn't read as malformed"
    );
    assert!(
        matches!(ask(r#"{"id":4,"ask":"start"}"#), Ask::Malformed { .. }),
        "a start with no span wasn't read as malformed"
    );
    assert_eq!(
        ask(r#"{"id":"all","ask":"watch","kind":["timer","stopwatch"]}"#),
        Ask::Watch {
            id: json!("all"),
            kinds: vec!["timer".to_owned(), "stopwatch".to_owned()]
        }
    );
    assert_eq!(ask(r#"{"id":5,"ask":"lap"}"#), Ask::Lap { id: json!(5) });
    assert_eq!(
        ask(r#"{"id":"all","ask":"watch","kind":"person"}"#),
        Ask::UnknownKind {
            id: json!("all"),
            kind: "person".to_owned()
        }
    );
    assert!(
        matches!(ask("not json"), Ask::Unknown { .. }),
        "a line that isn't JSON wasn't read as unknown"
    );
}

#[test]
fn a_watch_hears_only_the_kinds_it_asked_for_or_every_kind() {
    let timers = vec!["timer".to_owned()];
    assert!(watched(&timers, KIND), "a watch of timers didn't hear one");
    assert!(
        !watched(&timers, STOPWATCH),
        "a watch of timers heard the stopwatch"
    );
    assert!(
        watched(&[], STOPWATCH),
        "a watch of every kind didn't hear the stopwatch"
    );
}

#[test]
fn the_stopwatch_counts_while_it_runs_and_holds_its_count_stopped() {
    let mut held = Held::default();
    assert_eq!(held.stopwatch.json()["state"], "unstarted");
    held.stopwatch.run(1_000).unwrap();
    assert_eq!(held.stopwatch.count(4_000), 3_000);
    assert_eq!(unfit(held.stopwatch.run(4_000)), Some("runningAlready"));
    held.stopwatch.stop(5_000).unwrap();
    assert_eq!(held.stopwatch.count(60_000), 4_000);
    assert_eq!(held.stopwatch.json()["state"], "stopped");
    assert_eq!(unfit(held.stopwatch.stop(6_000)), Some("pausesRunning"));
    held.stopwatch.run(10_000).unwrap();
    assert_eq!(
        held.stopwatch.json(),
        json!({"key": "stopwatch", "kind": "stopwatch", "state": "running",
            "counted": 4_000, "since": 10_000, "laps": []})
    );
    assert_eq!(held.stopwatch.count(11_000), 5_000);
}

#[test]
fn a_lap_marks_its_count_only_while_it_runs_and_reset_clears_them() {
    let mut held = Held::default();
    assert_eq!(unfit(held.stopwatch.lap(0)), Some("lapsRunning"));
    held.stopwatch.run(0).unwrap();
    held.stopwatch.lap(2_000).unwrap();
    held.stopwatch.stop(3_000).unwrap();
    held.stopwatch.run(10_000).unwrap();
    held.stopwatch.lap(12_000).unwrap();
    assert_eq!(held.stopwatch.laps, vec![2_000, 5_000]);
    held.stopwatch.laps = vec![0; LAPS];
    assert_eq!(unfit(held.stopwatch.lap(13_000)), Some("lapsFull"));
    held.stopwatch.reset();
    assert_eq!(held.stopwatch, Stopwatch::default());
}

#[test]
fn what_was_kept_before_the_stopwatch_came_reads_with_one_unstarted() {
    let folder = std::env::temp_dir().join(format!("timers-older-{}", std::process::id()));
    let path = folder.join("held.json");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(&path, r#"{"timers":[],"next":3}"#).unwrap();
    let held = Held::read(&path);
    assert_eq!(held.next, 3);
    assert_eq!(held.stopwatch, Stopwatch::default());
    std::fs::remove_dir_all(folder).unwrap();
}
