//! Pains said in a cell kept apart from the keeper, as his own errands:
//! sent to that cell alone, tried as often as any other, given up when no
//! one can take them, and never the keeper's nor the misses ledger's.

use super::*;

/// A place whose routes keep ABC123 apart, with `said` in its session.
fn apart_place(name: &str, said: &[String]) -> Place {
    let place = Place::new(name);
    routed(&place, r#""apart":["abc123"]"#);
    place.log("src", "s-live", Path::new("/home/j/src"), said);
    place
}

/// A run at `place` over `cells`, each send to ABC123 going when `went`
/// and every other going, with what was sent to whom.
fn run(place: &Place, cells: &str, went: bool) -> (Hour, Vec<(String, String)>) {
    let mut sent = Vec::new();
    let hour = hear(&place.ears(), cells, &mut |to, text| {
        sent.push((to.to_owned(), text.to_owned()));
        to != "ABC123" || went
    })
    .expect("a run");
    (hour, sent)
}

/// Whether the hour says pains were given up on.
fn given_up(hour: &Hour) -> bool {
    hour.troubles.iter().any(|t| t.contains("gave up"))
}

#[test]
fn a_pain_hours_old_in_a_cell_kept_apart_still_goes_to_its_owner_alone() {
    let said = [user("old", "2026-11-16T15:00:00.000Z", "no, that's broken")];
    let place = apart_place("stale-apart", &said);
    let (hour, sent) = run(&place, CELLS, false);
    assert!(
        sent.len() == 1 && sent[0].0 == "ABC123",
        "a stale pain kept apart went past its owner as its send failed: {sent:?}"
    );
    let heard = place.heard();
    assert!(
        heard.len() == 1
            && heard[0].tries == 1
            && heard[0].routed_at.is_none()
            && heard[0].to.is_none(),
        "a failed send to its owner wasn't kept to be tried again: {heard:?}"
    );
    assert!(
        !given_up(&hour),
        "a pain still being tried was given up: {hour:?}"
    );
    let (_, sent) = run(&place, CELLS, true);
    assert!(
        sent.iter()
            .any(|(to, text)| to == "ABC123" && text.contains("broken"))
            && !sent.iter().any(|(to, _)| to == "ORCH01"),
        "a stale pain kept apart went to nobody, or to the keeper: {sent:?}"
    );
    assert!(
        place.heard().iter().all(|h| h.routed_at.is_some()),
        "the stale pain wasn't marked sent: {:?}",
        place.heard()
    );
    assert_eq!(
        fs::read_to_string(place.root.join("misses.jsonl")).unwrap_or_default(),
        "",
        "a pain sent to a cell kept apart was written into the misses ledger"
    );
}

#[test]
fn a_pain_in_a_cell_kept_apart_is_never_the_keepers_carried_though_a_ledger_quotes_it() {
    let said = [user(
        "e1",
        "2026-11-16T19:30:00.000Z",
        "the budget sheet is broken again",
    )];
    let place = apart_place("quoted-apart", &said);
    fs::write(
        place.root.join("asks.jsonl"),
        r#"{"at":"x","words":"the budget sheet is broken again","owner":"Y"}"#,
    )
    .expect("a ledger");
    let (_, sent) = run(&place, CELLS, true);
    assert!(
        place.heard().iter().all(|h| !h.carried),
        "a pain kept apart was read as carried by another's ledger line"
    );
    assert!(
        sent.iter()
            .any(|(to, text)| to == "ABC123" && text.contains("budget")),
        "a pain kept apart that a ledger quotes wasn't sent to its own cell: {sent:?}"
    );
}

#[test]
fn a_pain_in_a_cell_kept_apart_is_tried_three_times_then_given_up_never_the_keepers() {
    let said = [user(
        "e1",
        "2026-11-16T19:30:00.000Z",
        "no, that's broken again",
    )];
    let place = apart_place("tries-apart", &said);
    let mut asked = 0;
    for n in 1..=5 {
        let (hour, sent) = run(&place, CELLS, false);
        assert!(
            !sent.iter().any(|(to, _)| to == "ORCH01"),
            "the keeper was sent a pain kept apart on run {n}: {sent:?}"
        );
        asked += sent.len();
        assert_eq!(
            given_up(&hour),
            n >= 3,
            "run {n} said it gave up, or didn't, wrongly: {hour:?}"
        );
    }
    assert_eq!(asked, 3, "its owner wasn't asked three times, and no more");
}

#[test]
fn a_pain_in_a_cell_kept_apart_that_ended_goes_to_no_one_and_is_given_up_once_old() {
    let ended = r#"{"Ok":{"Sessions":[
        {"id":"s-live","address":"ABC123","state":"ended"},
        {"id":"s-void","address":"VOID01","state":"idle"}]}}"#;
    let said = [user(
        "e1",
        "2026-11-16T19:30:00.000Z",
        "no, that's broken again",
    )];
    let place = apart_place("ended-apart", &said);
    let (hour, sent) = run(&place, ended, true);
    assert!(
        sent.is_empty(),
        "a pain said in a cell kept apart went elsewhere once its session ended: {sent:?}"
    );
    assert!(!given_up(&hour), "a fresh pain was given up: {hour:?}");
    let later = Ears {
        at: AT + APART_MS,
        ..place.ears()
    };
    let mut sent: Vec<String> = Vec::new();
    let hour = hear(&later, ended, &mut |to, _| {
        sent.push(to.to_owned());
        true
    })
    .expect("a later run");
    assert!(
        sent.is_empty() && given_up(&hour),
        "an ended cell's pain past its time went somewhere, or wasn't given up: {sent:?} {hour:?}"
    );
}

#[test]
fn a_pain_in_a_cell_kept_apart_past_its_time_is_given_up_unsent() {
    let said = [user("old", "2026-11-16T11:00:00.000Z", "no, that's broken")];
    let place = apart_place("old-apart", &said);
    let (hour, sent) = run(&place, CELLS, true);
    assert!(
        sent.is_empty(),
        "a pain nine hours old went to a cell kept apart: {sent:?}"
    );
    assert!(
        given_up(&hour),
        "a pain no one will be sent wasn't said as given up: {hour:?}"
    );
}

#[test]
fn a_stall_hours_old_in_a_cell_kept_apart_goes_to_no_one() {
    let said = [user("a", "2026-11-16T17:00:00.000Z", "make the bar green")];
    let place = apart_place("stall-apart", &said);
    // 17:00 UTC, three hours before the run, and a three-minute wait.
    timed(&place, "s-live", AT - 3 * 3_600_000, 180_000);
    let (hour, sent) = run(&place, CELLS, true);
    assert!(
        place.heard().iter().any(stalled::is),
        "the stall wasn't heard at all, so this case proves nothing"
    );
    assert!(
        sent.is_empty(),
        "a stall three hours old was sent to a cell kept apart: {sent:?}"
    );
    assert!(
        !given_up(&hour),
        "a stall heard late was given up, though no one carries one: {hour:?}"
    );
}
