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
    let why = "put to you as a holder of launcher";
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
    for (answer, tries) in [(Sent::Failed, 1), (Sent::Held, 0), (Sent::OutOfTime, 0)] {
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
