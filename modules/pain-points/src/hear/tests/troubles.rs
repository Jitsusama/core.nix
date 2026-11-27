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
fn the_nodes_words_in_the_hours_troubles_are_quoted_as_its_own() {
    let trouble = super::super::troubles::Hubless {
        at: AT,
        sent: 2,
        why: "refused\n\"all clear\"".into(),
    }
    .trouble();
    assert!(
        trouble.ends_with("to the fallback: \"refused 'all clear'\""),
        "the node's words weren't quoted as its own: {trouble}"
    );
}

#[test]
fn a_pain_a_hub_would_take_waits_out_a_blink_then_goes_to_the_fallback_said() {
    let place = Place::new("hubless-node");
    place.routes();
    a_day(&place);
    let minute = 60_000;
    let void = "why is the launcher so slow";
    // Whom the void's pain went to this run, and whether the card says
    // pains went to the fallback for want of hubs, naming the error.
    let run = |at: i64, hubless: Option<&str>| {
        let ears = Ears {
            at,
            hubless: hubless.map(String::from),
            ..place.ears()
        };
        let mut to = Vec::new();
        let hour = hear(&ears, CELLS, &mut |owner, text| {
            if text.contains(void) && !text.contains(route::FOR_THE_KEEPER) {
                to.push(owner.to_owned());
            }
            true
        })
        .expect("a run");
        let said = hour
            .troubles
            .iter()
            .any(|t| t.contains("named no hubs") && t.contains("connection refused"));
        (to, said, hour)
    };
    let refused = Some("connection refused");
    let (to, said, hour) = run(AT, refused);
    assert!(
        to.is_empty() && !said,
        "a node that blinked for one run sent the void's pain on, or failed the run: {to:?} \
         {hour:?}"
    );
    assert_eq!(hour.hubless_since, Some(AT), "the outage wasn't counted");
    let (to, said, hour) = run(AT + 10 * minute, refused);
    assert!(to.is_empty() && !said, "within the grace: {to:?}");
    // The card says the grace, as it does a node naming no sessions
    // (#7022's round, F8).
    let line = super::super::said(&hour);
    assert!(
        line.contains("named no hubs for 10 minutes, so pains a hub would take wait"),
        "the grace isn't said: {line}"
    );
    let (to, said, hour) = run(AT + 15 * minute, refused);
    assert_eq!(to, ["ORCH01"], "past a blink it didn't go to the fallback");
    assert!(
        said,
        "a pain going to the fallback past a blink wasn't said on the card: {hour:?}"
    );
    let (to, said, hour) = run(AT + 20 * minute, None);
    assert!(
        to.is_empty() && said && hour.hubless_since.is_none(),
        "with hubs named again, the outage still counted, or the hour's fallback wasn't \
         said: {to:?} {hour:?}"
    );
    assert!(
        !run(AT + 76 * minute, None).1,
        "the fallback was still said past its hour"
    );
    fs::remove_file(place.root.join("state/routes.json")).expect("routes gone");
    run(AT + 80 * minute, refused);
    let (to, said, hour) = run(AT + 95 * minute, refused);
    assert!(
        to.is_empty() && !said && hour.hubless_since.is_none(),
        "a listener nobody set up counted or said the node out: {hour:?}"
    );
}

#[test]
fn a_pain_waits_out_one_blink_however_far_apart_the_runs_are() {
    let place = Place::new("hubless-gap");
    place.routes();
    a_day(&place);
    let minute = 60_000;
    let run = |at: i64| {
        let ears = Ears {
            at,
            hubless: Some("connection refused".into()),
            ..place.ears()
        };
        let mut to = Vec::new();
        let hour = hear(&ears, CELLS, &mut |owner, text| {
            if text.contains("why is the launcher so slow") && !text.contains(route::FOR_THE_KEEPER)
            {
                to.push(owner.to_owned());
            }
            true
        })
        .expect("a run");
        (to, hour)
    };
    let (to, _) = run(AT);
    assert!(
        to.is_empty(),
        "the void's pain didn't wait out the blink: {to:?}"
    );
    // Runs skipped between, so the outage's count starts again.
    let (to, hour) = run(AT + 15 * minute);
    assert_eq!(hour.hubless_since, Some(AT + 15 * minute), "{hour:?}");
    assert_eq!(
        to,
        ["ORCH01"],
        "a gap between runs started the pain's wait again: {hour:?}"
    );
}

#[test]
fn a_node_out_of_reach_while_no_pain_needs_a_hub_is_no_trouble() {
    let place = Place::new("hubless-quiet");
    place.routes();
    let minute = 60_000;
    for at in [
        AT,
        AT + 5 * minute,
        AT + 10 * minute,
        AT + 15 * minute,
        AT + 20 * minute,
    ] {
        let ears = Ears {
            at,
            hubless: Some("connection refused".into()),
            ..place.ears()
        };
        let hour = hear(&ears, CELLS, &mut |_, _| true).expect("a run");
        assert!(
            hour.troubles.is_empty(),
            "a node out of reach failed a run with no pain to route: {hour:?}"
        );
    }
}

#[test]
fn a_pain_for_want_of_hubs_whose_send_failed_is_not_said_to_have_gone() {
    let place = Place::new("hubless-failed");
    place.routes();
    a_day(&place);
    let minute = 60_000;
    let mut hour = Hour::default();
    let mut tried = Vec::new();
    // Runs five minutes apart, as its timer runs them, so the outage is
    // carried from each to the next until it is past a blink.
    for at in [AT, AT + 5 * minute, AT + 10 * minute, AT + 15 * minute] {
        let ears = Ears {
            at,
            hubless: Some("connection refused".into()),
            ..place.ears()
        };
        hour = hear(&ears, CELLS, &mut |owner, text| {
            if text.contains("why is the launcher so slow") && !text.contains(route::FOR_THE_KEEPER)
            {
                tried.push(owner.to_owned());
            }
            owner != "ORCH01"
        })
        .expect("a run");
    }
    // It was tried at the fallback, so the absence below is of a claim,
    // not of a send (#7022's round, F7).
    assert_eq!(tried, ["ORCH01"], "the fallback was never tried: {hour:?}");
    assert!(
        !hour.troubles.iter().any(|t| t.contains("named no hubs")),
        "a fallback send that failed was said to have gone for want of hubs: {hour:?}"
    );
}

#[test]
fn a_stall_no_hub_could_take_goes_to_the_fallback_at_once_while_the_node_blinks() {
    let place = Place::new("hubless-stall");
    place.routes();
    let said = [user("a", "2026-11-16T19:50:00.000Z", "make the bar green")];
    place.log("src", "s-live", Path::new("/home/j/src"), &said);
    // 19:50 UTC, and a three-minute wait for pi's first call, in a
    // session that has ended and named no purpose, so the stall says no
    // word a hub could take it by.
    timed(&place, "s-live", 1_794_858_600_000, 180_000);
    let cells = r#"{"Ok":{"Sessions":[
        {"id":"s-live","address":"ABC123","state":"ended"}]}}"#;
    let ears = Ears {
        hubless: Some("connection refused".into()),
        ..place.ears()
    };
    let mut to = Vec::new();
    let hour = hear(&ears, cells, &mut |owner, text| {
        if text.contains("stalled") && !text.contains(route::FOR_THE_KEEPER) {
            to.push(owner.to_owned());
        }
        true
    })
    .expect("a run");
    assert_eq!(
        to,
        ["ORCH01"],
        "a stall no hub could take waited out a blink: {hour:?}"
    );
    assert!(
        !hour.troubles.iter().any(|t| t.contains("named no hubs")),
        "a stall no hub could take was said to go for want of hubs: {hour:?}"
    );
}

#[test]
fn a_node_that_couldnt_be_asked_for_hubs_is_named_by_its_socket() {
    let place = Place::new("hubless-socket");
    let runtime = place.root.join("run");
    let (hubs, why) = hubs_named(&place.root, Some(runtime.clone()), Some("nowhere".into()));
    let socket = runtime.join("mycelium/nowhere.sock");
    assert!(hubs.is_empty(), "a node never asked named hubs: {hubs:?}");
    let why = why.expect("a node not there couldn't be asked");
    assert!(
        why.starts_with(&format!("{}: ", socket.display())),
        "the socket asked isn't named: {why}"
    );
    assert_eq!(
        hubs_named(&place.root, None, Some("nowhere".into())),
        (Vec::new(), None),
        "a machine with no runtime folder read as a node that couldn't be asked"
    );
}

#[test]
fn pains_sent_for_want_of_hubs_add_up_over_the_hour() {
    let place = Place::new("hubless-adds");
    place.routes();
    a_day(&place);
    let minute = 60_000;
    // The run before, past a blink already, sent two there.
    let before = Hour {
        at: AT,
        hubless_since: Some(AT - 15 * minute),
        hubless: Some(Hubless {
            at: AT,
            sent: 2,
            why: "connection refused".into(),
        }),
        ..Hour::default()
    };
    fs::write(
        place.root.join("state/hour.json"),
        serde_json::to_string(&before).expect("an hour"),
    )
    .expect("hour.json");
    let ears = Ears {
        at: AT + 5 * minute,
        hubless: Some("connection refused".into()),
        ..place.ears()
    };
    let mut to = Vec::new();
    let hour = hear(&ears, CELLS, &mut |owner, text| {
        if text.contains("why is the launcher so slow") && !text.contains(route::FOR_THE_KEEPER) {
            to.push(owner.to_owned());
        }
        true
    })
    .expect("a run");
    assert_eq!(to, ["ORCH01"], "the void's pain didn't go to the fallback");
    assert_eq!(
        hour.hubless.as_ref().map(|h| h.sent),
        Some(3),
        "this run's count replaced the hour's rather than adding to it: {hour:?}"
    );
    assert!(
        hour.troubles.iter().any(|t| t.contains("3 pains")),
        "the card didn't say the hour's count: {hour:?}"
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
