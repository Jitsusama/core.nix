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
fn a_pain_given_up_on_stays_a_trouble_through_its_day() {
    let place = Place::new("tries-later");
    place.routes();
    a_day(&place);
    for _ in 0..3 {
        hear(&place.ears(), CELLS, &mut |_, _| false).expect("a run");
    }
    let mut ears = place.ears();
    // Past the two hours its owner had, but within its day.
    ears.at = AT + 3 * 3_600_000;
    let hour = hear(&ears, CELLS, &mut |_, _| true).expect("a run");
    assert!(
        hour.troubles.iter().any(|t| t.contains("gave up")),
        "{hour:?}"
    );
}

#[test]
fn a_pain_too_late_for_its_owner_waits_on_no_mycelium() {
    let place = Place::new("late-no-sessions");
    place.routes();
    a_day(&place);
    let mut ears = place.ears();
    ears.at = AT + 3 * 3_600_000;
    let hour = hear(&ears, "", &mut |_, _| true).expect("a run");
    assert!(
        !hour
            .troubles
            .iter()
            .any(|t| t.contains("named no sessions")),
        "a pain that goes to no one waits on no one: {hour:?}"
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
fn a_mycelium_that_names_no_sessions_is_a_trouble() {
    let place = Place::new("no-sessions");
    place.routes();
    a_day(&place);
    let hour = hear(&place.ears(), "", &mut |_, _| true).expect("a run");
    assert!(
        hour.troubles
            .iter()
            .any(|t| t.contains("named no sessions")),
        "{hour:?}"
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
