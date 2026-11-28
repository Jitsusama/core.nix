//! A pain no live cell of its own takes goes through its subject's hub to
//! a holder of that subject, kept as a need, and to the fallback only
//! where no hub takes it.

use subject_hubs::Hub;

use super::*;

/// The day's cells, with a holder and a hub besides, and a live cell the
/// routes say never to send to.
const HELD: &str = r#"{"Ok":{"Sessions":[
    {"id":"s-live","address":"ABC123","state":"busy"},
    {"id":"s-void","address":"VOID01","state":"idle"},
    {"id":"s-never","address":"NEVER1","state":"idle"},
    {"id":"s-fit","address":"FIT002","state":"idle"},
    {"id":"s-hub","address":"HUB001","state":"idle"}]}}"#;

/// This place's ears, hearing `hubs` from the node.
fn ears_with(place: &Place, hubs: Vec<Hub>) -> Ears {
    Ears {
        hubs,
        ..place.ears()
    }
}

/// Each question a run asked, by whom it went to and what it said.
fn asked_in(ears: &Ears, cells: &str, answer: impl Fn(&str) -> Sent) -> Vec<(String, String)> {
    let mut asked = Vec::new();
    hear_putting(ears, cells, &mut |to, text, put| {
        if put == Put::Asked {
            asked.push((to.to_owned(), text.to_owned()));
        }
        answer(to)
    })
    .expect("a run");
    asked
}

const VOIDS: &str = "why is the launcher so slow";

/// Whom the pains saying `words` were put to as their owner's, in `asked`:
/// not the keeper's note of what was heard, which goes to the ledger
/// whoever owns each.
fn owners_of<'a>(asked: &'a [(String, String)], words: &str) -> Vec<&'a str> {
    asked
        .iter()
        .filter(|(_, text)| !text.contains(route::FOR_THE_KEEPER) && text.contains(words))
        .map(|(to, _)| to.as_str())
        .collect()
}

#[test]
fn a_void_pain_goes_to_a_holder_of_its_words_subject_and_is_kept_as_a_need() {
    let place = Place::new("hub-void");
    place.routes();
    a_day(&place);
    let ears = ears_with(
        &place,
        vec![Hub::new(
            "launcher",
            "HUB001",
            &["NEVER1", "VOID01", "FIT002"],
        )],
    );
    let asked = asked_in(&ears, HELD, |_| Sent::Went);
    assert_eq!(
        owners_of(&asked, VOIDS),
        ["FIT002"],
        "not the refused NEVER1, not the void's own VOID01, nor the fallback: {asked:?}"
    );
    assert!(
        asked
            .iter()
            .any(|(to, text)| to == "ORCH01" && text.contains("sent to FIT002")),
        "the ledger wasn't told the void's pain went to its holder: {asked:?}"
    );
    let why = "put to you as a holder of the subject \"launcher\"";
    assert!(
        asked
            .iter()
            .any(|(to, text)| to == "FIT002" && text.contains(why)),
        "its holder wasn't told why a pain from another's session is its own: {asked:?}"
    );
    let heard = place.heard();
    let void = heard
        .iter()
        .find(|h| h.words.contains(VOIDS))
        .expect("the void's pain heard");
    assert_eq!(void.to.as_deref(), Some("FIT002"), "{void:?}");
    let needs = place.needs();
    assert_eq!(needs.len(), 1, "one need wasn't kept: {needs:?}");
    let need = &needs[0];
    assert_eq!(
        (
            &need["asker"],
            &need["words"],
            &need["path"],
            &need["fit"],
            &need["cells"]
        ),
        (
            &json!("EAR001"),
            &json!(["launcher"]),
            &json!(["HUB001", "FIT002"]),
            &json!("FIT002"),
            &json!(5)
        ),
        "the need wasn't kept as the harness keeps one: {need}"
    );
}

#[test]
fn a_pain_held_through_a_blink_goes_to_its_holder_once_hubs_are_named_again() {
    let place = Place::new("hub-after-blink");
    place.routes();
    a_day(&place);
    let blinking = Ears {
        hubless: Some("connection refused".into()),
        ..place.ears()
    };
    let asked = asked_in(&blinking, HELD, |_| Sent::Went);
    assert!(
        owners_of(&asked, VOIDS).is_empty(),
        "the void's pain didn't wait out the blink: {asked:?}"
    );
    // The next run, five minutes on, the node names hubs again (#7022's
    // round, F6).
    let named = Ears {
        at: AT + 5 * 60_000,
        ..ears_with(&place, vec![Hub::new("launcher", "HUB001", &["FIT002"])])
    };
    let asked = asked_in(&named, HELD, |_| Sent::Went);
    assert_eq!(
        owners_of(&asked, VOIDS),
        ["FIT002"],
        "the pain held through a blink didn't reach its holder: {asked:?}"
    );
}

#[test]
fn an_ended_sessions_pain_goes_to_a_holder_of_what_it_worked_on() {
    let place = Place::new("hub-ended");
    place.routes();
    a_day(&place);
    let cells = r#"{"Ok":{"Sessions":[
        {"id":"s-live","address":"ABC123","state":"ended","working_on":"the dock's drawer"},
        {"id":"s-fit","address":"FIT002","state":"idle"},
        {"id":"s-hub","address":"HUB001","state":"idle"}]}}"#;
    let ears = ears_with(&place, vec![Hub::new("dock:drawer", "HUB001", &["FIT002"])]);
    let asked = asked_in(&ears, cells, |_| Sent::Went);
    assert_eq!(
        owners_of(&asked, "broken again"),
        ["FIT002"],
        "an ended session's pain didn't go by its purpose: {asked:?}"
    );
    assert!(
        asked.iter().all(|(_, text)| !text.contains(VOIDS)),
        "a pain said in a session the mycelium doesn't know didn't wait: {asked:?}"
    );
}

#[test]
fn a_session_known_to_be_the_void_holds_no_others_pain_though_the_routes_forgot_it() {
    let place = Place::new("hub-void-holder");
    // The routes name only NEVER1 never to send to, so nothing in them
    // keeps the void's VOID01 from holding a pain.
    place.routes();
    a_day(&place);
    let cells = r#"{"Ok":{"Sessions":[
        {"id":"s-live","address":"ABC123","state":"ended","working_on":"the launcher"},
        {"id":"s-void","address":"VOID01","state":"idle"},
        {"id":"s-hub","address":"HUB001","state":"idle"}]}}"#;
    let ears = ears_with(&place, vec![Hub::new("launcher", "HUB001", &["VOID01"])]);
    let asked = asked_in(&ears, cells, |_| Sent::Went);
    assert_eq!(
        owners_of(&asked, "broken again"),
        ["ORCH01"],
        "an ended session's pain went to the void, a hub's only holder: {asked:?}"
    );
}

#[test]
fn a_session_heard_in_the_void_holds_no_others_pain_though_none_of_its_own_is_kept() {
    let place = Place::new("hub-void-painless");
    place.routes();
    a_day(&place);
    // A second session in the void that said nothing a pain is made of.
    place.log(
        "void",
        "s-void2",
        &place.root.join("void"),
        &[user("w1", "2026-11-16T19:50:00.000Z", "I love this page!")],
    );
    let cells = r#"{"Ok":{"Sessions":[
        {"id":"s-live","address":"ABC123","state":"ended","working_on":"the launcher"},
        {"id":"s-void","address":"VOID01","state":"idle"},
        {"id":"s-void2","address":"VOID02","state":"idle"},
        {"id":"s-hub","address":"HUB001","state":"idle"}]}}"#;
    let ears = ears_with(&place, vec![Hub::new("launcher", "HUB001", &["VOID02"])]);
    let asked = asked_in(&ears, cells, |_| Sent::Went);
    assert!(
        place.heard().iter().all(|h| h.session != "s-void2"),
        "the second void session's words were kept as a pain"
    );
    assert_eq!(
        owners_of(&asked, "broken again"),
        ["ORCH01"],
        "a pain went to a void session with no pain of its own kept: {asked:?}"
    );
}

#[test]
fn one_no_hub_takes_goes_to_the_fallback_as_before() {
    for (hubs, why) in [
        (vec![], "no hub named"),
        (
            vec![Hub::new("switch", "HUB001", &["FIT002"])],
            "another subject's",
        ),
        (
            vec![Hub::new("launcher", "HUB001", &["VOID01", "NEVER1"])],
            "its only holders its own and a refused one",
        ),
        (
            vec![Hub::new("launcher", "GONE01", &["FIT002"])],
            "its hub gone",
        ),
    ] {
        let place = Place::new("hub-none");
        place.routes();
        a_day(&place);
        let asked = asked_in(&ears_with(&place, hubs), HELD, |_| Sent::Went);
        let to: Vec<&str> = asked
            .iter()
            .filter(|(_, text)| text.contains(VOIDS))
            .map(|(to, _)| to.as_str())
            .collect();
        assert_eq!(to, ["ORCH01"], "{why}: not to the fallback: {asked:?}");
        assert!(
            place.needs().is_empty(),
            "{why}: a need was kept for the fallback"
        );
    }
}

#[test]
fn only_a_send_to_a_holder_that_failed_spends_a_try() {
    for (answer, tries) in [
        (Sent::Failed, 1),
        (Sent::Late, 1),
        (Sent::Held, 0),
        (Sent::OutOfTime, 0),
    ] {
        let place = Place::new("hub-tries");
        place.routes();
        a_day(&place);
        let ears = ears_with(&place, vec![Hub::new("launcher", "HUB001", &["FIT002"])]);
        let _ = asked_in(&ears, HELD, |to| {
            if to == "FIT002" { answer } else { Sent::Went }
        });
        let heard = place.heard();
        let void = heard
            .iter()
            .find(|h| h.words.contains(VOIDS))
            .expect("the void's pain heard");
        assert_eq!(
            (void.tries, void.to.as_deref()),
            (tries, None),
            "{answer:?} to its holder spent the wrong tries, or was taken as sent: {void:?}"
        );
        assert!(
            place.needs().is_empty(),
            "{answer:?}: a need was kept for a send that didn't go"
        );
    }
}

#[test]
fn a_retry_goes_to_another_holder_than_the_one_it_failed_at_however_they_reorder() {
    // Named the other way round, a retry that counted its failures into
    // the list would land on FIT002 again; named the same way, one that
    // forgot where it failed would.
    for (n, order) in [["FIT003", "FIT002"], ["FIT002", "FIT003"]]
        .into_iter()
        .enumerate()
    {
        let retried = retried_after(&format!("hub-retry-{n}"), &order, Sent::Failed);
        assert_eq!(
            retried,
            ["FIT003"],
            "named {order:?}, the retry went back to the holder it failed at"
        );
    }
}

#[test]
fn a_retry_after_a_send_that_ran_late_goes_to_the_same_holder() {
    // The late send may have gone, so a second holder would work on the
    // pain beside the first; named either way, the first keeps it.
    for (n, order) in [["FIT003", "FIT002"], ["FIT002", "FIT003"]]
        .into_iter()
        .enumerate()
    {
        let retried = retried_after(&format!("hub-late-{n}"), &order, Sent::Late);
        assert_eq!(
            retried,
            ["FIT002"],
            "named {order:?}, the retry after a late send went to a second holder"
        );
    }
}

#[test]
fn a_retry_after_a_late_send_then_a_failed_one_goes_to_another_holder() {
    let place = Place::new("hub-late-failed");
    place.routes();
    a_day(&place);
    let cells = r#"{"Ok":{"Sessions":[
        {"id":"s-live","address":"ABC123","state":"busy"},
        {"id":"s-void","address":"VOID01","state":"idle"},
        {"id":"s-fit","address":"FIT002","state":"idle"},
        {"id":"s-fit3","address":"FIT003","state":"idle"},
        {"id":"s-hub","address":"HUB001","state":"idle"}]}}"#;
    let ears = ears_with(
        &place,
        vec![Hub::new("launcher", "HUB001", &["FIT002", "FIT003"])],
    );
    let mut runs = [Sent::Late, Sent::Failed].into_iter().map(|first| {
        let tried = asked_in(&ears, cells, |to| {
            if to == "FIT002" { first } else { Sent::Went }
        });
        owners_of(&tried, VOIDS)
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    });
    assert_eq!(runs.next().expect("late"), ["FIT002"], "the first send");
    assert_eq!(
        runs.next().expect("failed"),
        ["FIT002"],
        "the retry after a late send went to a second holder"
    );
    let retried = asked_in(&ears, cells, |_| Sent::Went);
    assert_eq!(
        owners_of(&retried, VOIDS),
        ["FIT003"],
        "once its send failed, the holder it ran late at kept the pain"
    );
}

#[test]
fn a_retry_after_a_late_send_goes_where_it_went_though_its_path_changed() {
    // From a hub's holder to where no hub takes it, and from the
    // fallback to where a hub does: either way the late send may have
    // landed, so a second cell would work on the pain beside the first
    // (#7022's round, F1, F12 and F16).
    let hub = || vec![Hub::new("launcher", "HUB001", &["FIT002"])];
    for (n, (first, then, late_at)) in [(hub(), vec![], "FIT002"), (vec![], hub(), "ORCH01")]
        .into_iter()
        .enumerate()
    {
        let place = Place::new(&format!("hub-late-path-{n}"));
        place.routes();
        a_day(&place);
        let went = asked_in(&ears_with(&place, first), HELD, |to| {
            if to == late_at {
                Sent::Late
            } else {
                Sent::Went
            }
        });
        assert_eq!(owners_of(&went, VOIDS), [late_at], "the first send");
        let retried = asked_in(&ears_with(&place, then), HELD, |_| Sent::Went);
        assert_eq!(
            owners_of(&retried, VOIDS),
            [late_at],
            "its path changed, the retry after a late send went to a second cell"
        );
        assert!(
            place.needs().is_empty(),
            "a need was kept by a hub the retry didn't go by"
        );
    }
}

#[test]
fn a_retry_after_a_late_send_goes_on_once_that_holder_has_ended() {
    let place = Place::new("hub-late-ended");
    place.routes();
    a_day(&place);
    let ears = ears_with(&place, vec![Hub::new("launcher", "HUB001", &["FIT002"])]);
    let _ = asked_in(&ears, HELD, |to| {
        if to == "FIT002" {
            Sent::Late
        } else {
            Sent::Went
        }
    });
    let ended = r#"{"Ok":{"Sessions":[
        {"id":"s-live","address":"ABC123","state":"busy"},
        {"id":"s-void","address":"VOID01","state":"idle"},
        {"id":"s-never","address":"NEVER1","state":"idle"},
        {"id":"s-hub","address":"HUB001","state":"idle"}]}}"#;
    let retried = asked_in(&ears, ended, |_| Sent::Went);
    assert_eq!(
        owners_of(&retried, VOIDS),
        ["ORCH01"],
        "the retry waited on a holder that has ended"
    );
}

/// A run whose send to FIT002 answers `first`, then one whose hub names
/// its holders in `order`: the owners the retry gave the void's pain to.
fn retried_after(name: &str, order: &[&str], first: Sent) -> Vec<String> {
    let place = Place::new(name);
    place.routes();
    a_day(&place);
    let cells = r#"{"Ok":{"Sessions":[
        {"id":"s-live","address":"ABC123","state":"busy"},
        {"id":"s-void","address":"VOID01","state":"idle"},
        {"id":"s-never","address":"NEVER1","state":"idle"},
        {"id":"s-fit","address":"FIT002","state":"idle"},
        {"id":"s-fit3","address":"FIT003","state":"idle"},
        {"id":"s-hub","address":"HUB001","state":"idle"}]}}"#;
    let once = ears_with(
        &place,
        vec![Hub::new("launcher", "HUB001", &["FIT002", "FIT003"])],
    );
    let tried = asked_in(&once, cells, |to| {
        if to == "FIT002" { first } else { Sent::Went }
    });
    assert_eq!(owners_of(&tried, VOIDS), ["FIT002"], "{tried:?}");
    let then = ears_with(&place, vec![Hub::new("launcher", "HUB001", order)]);
    let retried = asked_in(&then, cells, |_| Sent::Went);
    owners_of(&retried, VOIDS)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

#[test]
fn pains_put_to_a_hub_in_one_run_spread_by_least_held_each_need_its_own_id() {
    let place = Place::new("hub-spread");
    place.routes();
    a_day(&place);
    let froze = "the launcher froze again, wtf";
    place.log(
        "void",
        "s-void",
        &place.root.join("void"),
        &[
            user("v1", "2026-11-16T19:50:00.000Z", VOIDS),
            user("v2", "2026-11-16T19:52:00.000Z", froze),
        ],
    );
    let cells = r#"{"Ok":{"Sessions":[
        {"id":"s-live","address":"ABC123","state":"busy"},
        {"id":"s-void","address":"VOID01","state":"idle"},
        {"id":"s-fit","address":"FIT002","state":"idle"},
        {"id":"s-fit3","address":"FIT003","state":"idle"},
        {"id":"s-hub","address":"HUB001","state":"idle"}]}}"#;
    let ears = ears_with(
        &place,
        vec![Hub::new("launcher", "HUB001", &["FIT002", "FIT003"])],
    );
    let asked = asked_in(&ears, cells, |_| Sent::Went);
    let mut owners = [owners_of(&asked, VOIDS), owners_of(&asked, froze)].concat();
    owners.sort_unstable();
    assert_eq!(
        owners,
        ["FIT002", "FIT003"],
        "two pains in one run weren't spread one to each holder: {asked:?}"
    );
    let needs = place.needs();
    let ids: BTreeSet<&str> = needs.iter().filter_map(|n| n["need"].as_str()).collect();
    assert_eq!(
        (needs.len(), ids.len()),
        (2, 2),
        "two needs put in one run didn't each keep an id of its own: {needs:?}"
    );
}

#[test]
fn a_need_that_couldnt_be_kept_is_a_trouble_and_the_pain_still_sent() {
    let place = Place::new("hub-unkept");
    place.routes();
    a_day(&place);
    // A file where the mesh's folder goes, so no needs folder can be made.
    fs::write(place.root.join("mesh"), "").expect("a file in the folder's place");
    let ears = ears_with(&place, vec![Hub::new("launcher", "HUB001", &["FIT002"])]);
    let hour = hear_putting(&ears, HELD, &mut |_, _, _| Sent::Went).expect("a run");
    assert!(
        hour.troubles
            .iter()
            .any(|t| t.starts_with("the needs put to hubs couldn't be kept: ")),
        "a need lost wasn't said: {hour:?}"
    );
    let heard = place.heard();
    let void = heard
        .iter()
        .find(|h| h.words.contains(VOIDS))
        .expect("the void's pain heard");
    assert_eq!(
        void.to.as_deref(),
        Some("FIT002"),
        "a need not kept unsent its pain: {void:?}"
    );
}

#[test]
fn a_need_is_kept_before_the_next_owners_send() {
    let place = Place::new("hub-kept-each");
    place.routes();
    a_day(&place);
    // The holder sorts before ABC123, whose own pain is sent after it, so
    // a run stopped at that send has kept the holder's need already.
    let cells = r#"{"Ok":{"Sessions":[
        {"id":"s-live","address":"ABC123","state":"busy"},
        {"id":"s-void","address":"VOID01","state":"idle"},
        {"id":"s-never","address":"NEVER1","state":"idle"},
        {"id":"s-fit","address":"A0FIT2","state":"idle"},
        {"id":"s-hub","address":"HUB001","state":"idle"}]}}"#;
    let ears = ears_with(&place, vec![Hub::new("launcher", "HUB001", &["A0FIT2"])]);
    let mut kept_by_then = None;
    hear_putting(&ears, cells, &mut |to, text, _| {
        if to == "ABC123" && text.contains("broken again") {
            kept_by_then.get_or_insert_with(|| place.needs().len());
        }
        Sent::Went
    })
    .expect("a run");
    assert_eq!(
        kept_by_then,
        Some(1),
        "the holder's need wasn't kept before the next send"
    );
    assert_eq!(place.needs().len(), 1, "a need was kept twice");
}

#[test]
fn a_need_that_couldnt_be_kept_stays_a_trouble_for_the_hour() {
    let place = Place::new("hub-unkept-hour");
    place.routes();
    a_day(&place);
    fs::write(place.root.join("mesh"), "").expect("a file in the folder's place");
    let hubs = || vec![Hub::new("launcher", "HUB001", &["FIT002"])];
    let ears = ears_with(&place, hubs());
    hear_putting(&ears, HELD, &mut |_, _, _| Sent::Went).expect("the run that lost it");
    // The folder can be made now, and nothing new is put to a hub.
    fs::remove_file(place.root.join("mesh")).expect("the file gone");
    let said_at = |at: i64| {
        let ears = Ears {
            at,
            ..ears_with(&place, hubs())
        };
        hear_putting(&ears, HELD, &mut |_, _, _| Sent::Went)
            .expect("a later run")
            .troubles
            .iter()
            .any(|t| t.starts_with("the needs put to hubs couldn't be kept: "))
    };
    let minute = 60_000;
    assert!(said_at(ears.at + 5 * minute), "the next run forgot it");
    assert!(said_at(ears.at + 59 * minute), "forgotten within the hour");
    assert!(!said_at(ears.at + 61 * minute), "still said past its hour");
}

#[test]
fn a_need_unkept_ahead_of_the_run_is_no_trouble() {
    let place = Place::new("hub-unkept-ahead");
    let ears = place.ears();
    let ahead = Hour {
        at: ears.at,
        unkept: Some(Unkept {
            at: ears.at + 60_000,
            why: "read-only".into(),
        }),
        ..Hour::default()
    };
    fs::write(
        place.root.join("state/hour.json"),
        serde_json::to_string(&ahead).expect("an hour"),
    )
    .expect("hour.json");
    assert_eq!(
        crate::hear::troubles::unkept(&ears.state, ears.at, None),
        None,
        "one dated ahead was carried"
    );
}
