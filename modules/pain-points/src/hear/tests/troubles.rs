//! Runs that couldn't route as they should, each said as a trouble, so a
//! broken listener never reads as a quiet hour; and those that look broken
//! but aren't, which say nothing.

use super::*;

#[test]
fn a_run_that_could_not_send_says_so() {
    let place = Place::new("failing");
    place.routes();
    a_day(&place);
    let hour = hear(&place.ears(), CELLS, &mut |_, _| false).expect("a run");
    assert!(
        hour.troubles.iter().any(|t| t.contains("failed")),
        "{hour:?}"
    );
    fs::write(place.root.join("state/routes.json"), "{not json").expect("routes");
    let hour = hear(&place.ears(), CELLS, &mut |_, _| true).expect("a run");
    assert!(
        hour.troubles.iter().any(|t| t.contains("routes.json")),
        "a broken routes file isn't a quiet hour: {hour:?}"
    );
}

#[test]
fn a_finding_the_node_never_takes_is_tried_three_times_then_said_as_a_trouble() {
    let place = Place::new("tries");
    place.routes();
    a_day(&place);
    let mut to_owner = 0;
    let mut last = None;
    for _ in 0..5 {
        last = Some(
            hear(&place.ears(), CELLS, &mut |to, _| {
                to_owner += usize::from(to == "ABC123");
                false
            })
            .expect("a run"),
        );
    }
    assert_eq!(to_owner, 3, "a cell is asked again every five minutes");
    let hour = last.expect("runs");
    assert!(
        hour.troubles.iter().any(|t| t.contains("gave up")),
        "{hour:?}"
    );
}

#[test]
fn a_mycelium_with_no_sessions_and_nothing_waiting_is_no_trouble() {
    let place = Place::new("quiet-node");
    place.routes();
    let hour = hear(&place.ears(), "", &mut |_, _| true).expect("a run");
    assert!(hour.troubles.is_empty(), "{hour:?}");
}

#[test]
fn a_mycelium_that_names_no_sessions_for_a_quarter_hour_is_a_trouble() {
    let place = Place::new("no-sessions");
    place.routes();
    a_day(&place);
    let minute = 60_000;
    let run = |at: i64, cells: &str| {
        let ears = Ears { at, ..place.ears() };
        let hour = hear(&ears, cells, &mut |_, _| false).expect("a run");
        let trouble = hour
            .troubles
            .iter()
            .any(|t| t.contains("named no sessions"));
        (trouble, said(&hour))
    };
    // Runs on the five-minute beat, each started a few seconds late, as
    // a busy machine starts them: the limits sit off the beat.
    let late = 4_000;
    // A node that blinks as the machine goes down is asked again next
    // run, so a run or two without it fails nothing, though it's said.
    let (trouble, line) = run(AT, "");
    assert!(!trouble, "a first run without cells fails nothing");
    assert!(
        line.contains("named no sessions for 0 minutes"),
        "the grace is said: {line}"
    );
    assert!(
        !run(AT + 5 * minute + late, "").0,
        "five minutes without cells fails nothing"
    );
    let (trouble, line) = run(AT + 10 * minute + 2 * late, "");
    assert!(!trouble, "ten minutes without cells fails nothing");
    assert!(line.contains("for 10 minutes, so pains wait"), "{line}");
    let (trouble, line) = run(AT + 15 * minute + late, "");
    assert!(trouble, "a quarter hour without cells is a trouble: {line}");
    assert!(
        !line.contains("so pains wait"),
        "the trouble isn't said twice: {line}"
    );
    assert!(
        run(AT + 20 * minute, "").0,
        "an outage that goes on fails each run"
    );
    // A run that reads cells starts the count again.
    assert!(
        !run(AT + 25 * minute, CELLS).0,
        "a run with cells has no such trouble"
    );
    assert!(
        !run(AT + 30 * minute, "").0,
        "the count starts again after cells"
    );
    // A machine that was off isn't a node that said nothing all along,
    // and the count it starts again still ends in a trouble.
    let back = AT + 5 * 60 * minute;
    assert!(
        !run(back, "").0,
        "a run after hours off starts the count again"
    );
    assert!(
        !run(back + 5 * minute, "").0,
        "five minutes after hours off fails nothing"
    );
    assert!(
        !run(back + 10 * minute, "").0,
        "ten minutes after hours off fails nothing"
    );
    assert!(
        run(back + 15 * minute, "").0,
        "a quarter hour after hours off is a trouble"
    );
}

#[test]
fn a_mycelium_that_could_not_be_started_is_a_trouble_at_once_by_its_error() {
    let place = Place::new("unstarted-node");
    place.routes();
    let ears = Ears {
        unnamed: Some("entity not found".into()),
        ..place.ears()
    };
    let hour = hear(&ears, "", &mut |_, _| false).expect("a run");
    assert!(
        hour.troubles
            .iter()
            .any(|t| t.contains("couldn't be started") && t.contains("entity not found")),
        "the first run says why no session was named, with nothing waiting: {hour:?}"
    );
    a_day(&place);
    let hour = hear(&ears, "", &mut |_, _| false).expect("a run");
    assert!(
        !said(&hour).contains("so pains wait"),
        "a known cause isn't said as a node silent: {}",
        said(&hour)
    );
    let later = Ears {
        at: AT + 20 * 60_000,
        ..ears
    };
    let hour = hear(&later, "", &mut |_, _| false).expect("a run");
    assert_eq!(
        hour.troubles
            .iter()
            .filter(|t| t.contains("named no sessions") || t.contains("couldn't be started to"))
            .count(),
        1,
        "the cause is said once, not as a silence too: {hour:?}"
    );
    fs::remove_file(place.root.join("state/routes.json")).expect("routes gone");
    let unset = Ears {
        unnamed: Some("entity not found".into()),
        ..place.ears()
    };
    let hour = hear(&unset, "", &mut |_, _| false).expect("a run");
    assert!(
        hour.troubles
            .iter()
            .all(|t| !t.contains("couldn't be started to")),
        "a listener nobody set up has no such trouble: {hour:?}"
    );
}

#[test]
fn an_earlier_run_dated_ahead_of_this_one_holds_no_outage_back() {
    let place = Place::new("future-hour");
    place.routes();
    a_day(&place);
    let minute = 60_000;
    let since = |before: &Hour, at: i64| {
        fs::write(
            place.root.join("state/hour.json"),
            serde_json::to_string(before).expect("an hour"),
        )
        .expect("hour.json");
        let ears = Ears { at, ..place.ears() };
        hear(&ears, "", &mut |_, _| false)
            .expect("a run")
            .cellless_since
    };
    let ahead = Hour {
        at: AT + 60 * minute,
        cellless_since: Some(AT),
        ..Hour::default()
    };
    assert_eq!(
        since(&ahead, AT + minute),
        Some(AT + minute),
        "a run dated ahead carries nothing"
    );
    let begun_ahead = Hour {
        at: AT,
        cellless_since: Some(AT + 60 * minute),
        ..Hour::default()
    };
    assert_eq!(
        since(&begun_ahead, AT + 5 * minute),
        Some(AT + 5 * minute),
        "a count begun ahead starts again"
    );
    let before = Hour {
        at: AT,
        cellless_since: Some(AT - 5 * minute),
        ..Hour::default()
    };
    assert_eq!(
        since(&before, AT + 5 * minute),
        Some(AT - 5 * minute),
        "a count carries on"
    );
}

#[test]
fn routes_that_cant_be_read_or_name_no_sender_are_troubles() {
    for (name, routes) in [
        ("routes-dir", None),
        ("routes-no-from", Some(r#"{"never":["V"]}"#)),
    ] {
        let place = Place::new(name);
        let at = place.root.join("state/routes.json");
        match routes {
            Some(text) => fs::write(&at, text).expect("routes"),
            None => fs::create_dir_all(&at).expect("a folder where routes.json goes"),
        }
        a_day(&place);
        let hour = hear(&place.ears(), CELLS, &mut |to, text| {
            panic!("sent {to} without routes: {text}")
        })
        .expect("a run");
        assert!(
            hour.troubles.iter().any(|t| t.contains("routes.json")),
            "{name}: {hour:?}"
        );
    }
}

#[test]
fn routes_that_never_name_a_cell_never_to_send_nothing() {
    let place = Place::new("no-never");
    fs::write(
        place.root.join("state/routes.json"),
        r#"{"from":"EAR001","fallback":"ORCH01","ledgers":"ORCH01"}"#,
    )
    .expect("routes");
    a_day(&place);
    let hour = hear(&place.ears(), CELLS, &mut |to, text| {
        panic!("sent {to} with no void guarded: {text}")
    })
    .expect("a run");
    assert!(
        hour.troubles.iter().any(|t| t.contains("never")),
        "{hour:?}"
    );
}
