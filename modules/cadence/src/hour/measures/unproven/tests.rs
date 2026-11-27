//! Joel's asks unproven, read from asks lists and probe runs in the shapes
//! their keepers write.

use serde_json::{Value, json};

use super::super::slipped::{at, list};
use super::*;

const NOW: i64 = 1_794_700_000;

/// An ask of Joel's asked a day before landing `ago` seconds before now,
/// known by its words as its id, answered for by `answers`, with `more`
/// fields.
fn landed(words: &str, ago: i64, answers: &str, more: &Value) -> Value {
    let mut ask = json!({
        "id": words, "at": at(asked(ago)), "words": words, "owner": "J7XHQN (WCJQSG)",
        "answers": answers, "expected": at(NOW - ago), "landed": at(NOW - ago),
        "probe": {"cmd": "true"},
    });
    if let (Some(ask), Some(more)) = (ask.as_object_mut(), more.as_object()) {
        ask.extend(more.clone());
    }
    ask
}

/// A probe run for the ask with the id `ask`, `ago` seconds before now.
fn run(ask: &str, ago: i64, passed: bool, switch: i64) -> String {
    json!({
        "ask": ask, "at": at(NOW - ago), "passed": passed, "switch": switch,
        "by": "J7XHQN",
    })
    .to_string()
}

/// When the ask landing `ago` seconds before now was asked.
const fn asked(ago: i64) -> i64 {
    NOW - ago - DAY
}

/// The measure over `asks`, with the runs `probes`.
fn read(asks: &[Value], probes: &[String]) -> Measure {
    unproven(&list(asks), Ok(&probes.join("\n")), "", NOW)
}

#[test]
fn an_ask_landed_past_two_hours_with_no_probe_run_is_asked_of_who_answers_for_it() {
    let m = read(
        &[landed("show the card", 3 * 3600, "1JQPPC", &json!({}))],
        &[],
    );
    assert!((m.now - 1.0).abs() < 0.5, "one unproven, got {}", m.now);
    assert_eq!(m.target, Some(0.0), "held to none");
    assert_eq!(
        m.owner, "1JQPPC",
        "who answers for the result owns it, not the ask's owner"
    );
    assert_eq!(
        m.behind,
        [format!(
            "show the card, asked {}; its probe hasn't run since it landed · 1JQPPC",
            at(asked(3 * 3600))
        )],
        "the line names the ask, when it was asked, its state and who answers for it"
    );
    assert_eq!(m.key, KEY, "known by its key from hour to hour");
}

#[test]
fn an_ask_inside_its_two_hours_is_not_counted_and_one_past_the_week_counts_on_one_line() {
    let m = read(
        &[
            landed("just landed", 3600, "A", &json!({})),
            landed("long ago", 8 * DAY, "B", &json!({})),
            landed("longer ago", 30 * DAY, "C", &json!({})),
        ],
        &[],
    );
    assert!(
        (m.now - 2.0).abs() < 0.5,
        "two older asks counted, got {}",
        m.now
    );
    assert_eq!(
        m.behind,
        [format!(
            "2 older than a week, left to the probes' backfill · {KEEPER}"
        )],
        "an ask still in its grace was counted, or one past the week was asked apart"
    );
}

#[test]
fn an_ask_landed_after_the_last_switch_says_it_isnt_live_yet() {
    let ago = 3 * 3600;
    let asks = list(&[landed("waits", ago, "A", &json!({}))]);
    let before = json!({"at": at(NOW - ago - 60)}).to_string();
    let m = unproven(&asks, Ok(""), &before, NOW);
    assert!(
        m.behind[0].contains("; its probe hasn't run since it landed, not live yet,"),
        "an ask no switch has carried didn't say so: {}",
        m.behind[0]
    );
    let after = json!({"at": at(NOW - 60)}).to_string();
    let m = unproven(&asks, Ok(""), &format!("{before}\n{after}"), NOW);
    assert!(
        !m.behind[0].contains("not live yet"),
        "an ask a switch has come since said it wasn't live: {}",
        m.behind[0]
    );
}

#[test]
fn a_probe_passing_since_landing_proves_it_and_its_latest_run_decides() {
    let ago = 5 * 3600;
    // Every ask asked at the same moment, so only its id tells its runs apart.
    let asks = [
        "passed",
        "failed after passing",
        "passed after failing",
        "passed before landing",
    ]
    .map(|words| landed(words, ago, "A", &json!({})));
    let probes = [
        run("passed", 3600, true, 1200),
        run("failed after passing", 7200, true, 1199),
        run("failed after passing", 3600, false, 1200),
        run("passed after failing", 7200, false, 1199),
        run("passed after failing", 3600, true, 1200),
        run("passed before landing", ago + 60, true, 1198),
    ];
    let m = read(&asks, &probes);
    let states: Vec<&str> = m
        .behind
        .iter()
        .filter_map(|line| line.split_once(", asked "))
        .map(|(words, _)| words)
        .collect();
    assert_eq!(
        states,
        ["failed after passing", "passed before landing"],
        "a latest run failing, or a pass only before landing, left it proven"
    );
    assert!(
        m.behind[0].contains("its probe failed after switch 1200"),
        "a failed run names the switch it ran after: {}",
        m.behind[0]
    );
}

#[test]
fn an_ask_he_confirmed_put_down_parked_derived_or_still_open_needs_no_probe() {
    let ago = 3 * 3600;
    let m = read(
        &[
            landed("confirmed", ago, "A", &json!({"confirmed": at(NOW - 60)})),
            landed("parked", ago, "B", &json!({"parked": "b95ea57b"})),
            landed(
                "put down",
                ago,
                "C",
                &json!({"closed": "put down", "landed": null}),
            ),
            landed("derived", ago, "D", &json!({"tracking": "derived"})),
            landed("open", ago, "E", &json!({"landed": null})),
        ],
        &[],
    );
    assert!(
        m.behind.is_empty(),
        "an ask needing no probe was asked: {:?}",
        m.behind
    );
}

#[test]
fn asks_naming_no_probe_count_on_their_keepers_line_once_the_list_names_any() {
    let ago = 3 * 3600;
    let bare = landed("no probe", ago, "A", &json!({"probe": null}));
    let m = read(std::slice::from_ref(&bare), &[]);
    assert_eq!(
        (m.target, m.says.contains("no ask names a probe yet")),
        (None, true),
        "a list naming no probe yet was judged: {} at {:?}",
        m.says,
        m.target
    );
    let other = landed("no probe either", ago, "C", &json!({"probe": null}));
    let m = read(&[bare, other, landed("probed", ago, "B", &json!({}))], &[]);
    assert_eq!(m.target, Some(0.0), "a list naming a probe is judged");
    assert!((m.now - 3.0).abs() < 0.5, "all three count, got {}", m.now);
    assert_eq!(
        m.behind[1..],
        [format!("{UNNAMED}; 2 of them · {KEEPER}")],
        "asks naming no probe were asked of a cell that can't write one: {:?}",
        m.behind
    );
}

#[test]
fn unread_probe_runs_leave_it_unjudged_and_an_ask_is_read_from_its_latest_line() {
    let ago = 3 * 3600;
    let first = landed("moved", ago, "A", &json!({"landed": null}));
    let latest = landed("moved", ago, "B", &json!({}));
    let m = unproven(&list(&[first, latest]), Err("permission denied"), "", NOW);
    assert_eq!(
        (
            m.target,
            m.says.contains("the probe runs unread: permission denied")
        ),
        (None, true),
        "unread runs passed as none passed: {} at {:?}",
        m.says,
        m.target
    );
    assert_eq!(
        m.behind.len(),
        1,
        "an ask written twice counted twice: {:?}",
        m.behind
    );
    assert!(
        m.behind[0].ends_with(" · B"),
        "not its latest line: {}",
        m.behind[0]
    );
    assert!(
        m.behind[0].contains("; its probe runs unread · "),
        "an unread record's line said no probe ran: {}",
        m.behind[0]
    );
}

#[test]
fn a_twin_folded_into_its_kept_record_neither_counts_nor_hides_it() {
    let ago = 3 * 3600;
    let kept = landed("kept", ago, "A", &json!({}));
    // Landed as its record did, so it would count again were it read.
    let twin = landed(
        "kept",
        ago,
        "B",
        &json!({"id": "kept~1", "closed": "folded"}),
    );
    let m = read(&[kept, twin], &[]);
    assert_eq!(
        m.behind
            .iter()
            .map(|line| line.ends_with(" · A"))
            .collect::<Vec<_>>(),
        [true],
        "a folded twin counted, or stood in for its kept record: {:?}",
        m.behind
    );
}

#[test]
fn an_ask_with_no_id_reads_no_runs() {
    let ago = 3 * 3600;
    let bare = landed("no id", ago, "A", &json!({"id": null}));
    let m = read(&[bare], &[run("no id", 60, true, 1200)]);
    assert!(
        m.behind
            .first()
            .is_some_and(|line| line.contains("its probe hasn't run since it landed")),
        "an ask with no id read a run by its words: {:?}",
        m.behind
    );
}

#[test]
fn an_ask_written_first_with_no_id_and_then_with_one_counts_once() {
    let ago = 3 * 3600;
    let bare = landed("given an id", ago, "A", &json!({"id": null}));
    let empty = landed("given an id", ago, "A", &json!({"id": ""}));
    let named = landed("given an id", ago, "B", &json!({}));
    let m = read(&[bare, empty, named], &[run("given an id", 60, true, 1200)]);
    assert!(
        m.behind.is_empty(),
        "its lines with no id or an empty one counted apart from it: {:?}",
        m.behind
    );
}

#[test]
fn a_confirmed_written_false_is_not_confirmed() {
    let m = read(
        &[landed(
            "unconfirmed",
            3 * 3600,
            "A",
            &json!({"confirmed": false}),
        )],
        &[],
    );
    assert_eq!(m.behind.len(), 1, "confirmed false read as confirmed");
}

#[test]
fn a_probe_record_kept_is_read_where_its_keeper_writes_it() {
    let home = tempfile::tempdir().expect("a state folder");
    std::fs::create_dir_all(home.path().join("mission")).expect("a folder for the record");
    std::fs::write(home.path().join(PROBES), run("proven", 3600, true, 1200)).expect("a record");
    let asks = list(&[landed("proven", 3 * 3600, "A", &json!({}))]);
    let m = super::read(home.path(), &asks, "", NOW);
    assert_eq!(
        (m.target, m.behind.len()),
        (Some(0.0), 0),
        "a pass kept at mission/probes.jsonl wasn't read: {} at {:?}, {:?}",
        m.says,
        m.target,
        m.behind
    );
}

#[test]
fn a_probe_record_gone_leaves_it_unjudged() {
    let home = tempfile::tempdir().expect("a state folder");
    let asks = list(&[landed("proven", 3 * 3600, "A", &json!({}))]);
    let m = super::read(home.path(), &asks, "", NOW);
    assert_eq!(
        (
            m.target,
            m.says.contains("the probe runs unread: none kept")
        ),
        (None, true),
        "a probe record gone read as none passed: {} at {:?}",
        m.says,
        m.target
    );
}

#[test]
fn an_emptied_probe_record_leaves_it_unjudged() {
    let home = tempfile::tempdir().expect("a state folder");
    std::fs::create_dir_all(home.path().join("mission")).expect("a folder for the record");
    std::fs::write(home.path().join(PROBES), "\n").expect("an emptied record");
    let asks = list(&[landed("proven", 3 * 3600, "A", &json!({}))]);
    let m = super::read(home.path(), &asks, "", NOW);
    assert_eq!(
        (
            m.target,
            m.says.contains("the probe runs unread: none kept")
        ),
        (None, true),
        "an emptied probe record read as none passed: {} at {:?}",
        m.says,
        m.target
    );
}

#[test]
fn a_torn_probe_line_leaves_the_runs_unread_rather_than_skipped() {
    let asks = [landed("torn", 5 * 3600, "A", &json!({}))];
    let probes = [
        run("torn", 7200, true, 1199),
        r#"{"ask": "torn", "at": "#.to_owned(),
    ];
    let m = read(&asks, &probes);
    assert_eq!(
        (
            m.target,
            m.says.contains("the probe runs unread: line 2 won't parse")
        ),
        (None, true),
        "a torn run after a pass was skipped, so the ask read as proven: {} at {:?}",
        m.says,
        m.target
    );
}

#[test]
fn a_line_that_parses_but_isnt_a_run_leaves_the_runs_unread_rather_than_skipped() {
    let asks = [landed("odd", 5 * 3600, "A", &json!({}))];
    for (odd, why) in [
        (
            json!({"ask": "odd", "at": NOW - 3600, "passed": false}),
            "an epoch time",
        ),
        (
            json!({"ask": "odd", "at": at(NOW - 3600), "passed": "false"}),
            "a word passed",
        ),
        (
            json!({"ask_id": "odd", "at": at(NOW - 3600), "passed": false}),
            "no ask",
        ),
        (
            json!({"ask": "odd", "at": at(NOW - 3600), "passed": false, "switch": "abc1"}),
            "a switch by rev",
        ),
        (json!(null), "no record"),
    ] {
        let probes = [run("odd", 7200, true, 1199), odd.to_string()];
        let m = read(&asks, &probes);
        assert_eq!(
            (
                m.target,
                m.says.contains("the probe runs unread: line 2 isn't a run")
            ),
            (None, true),
            "a run with {why} was skipped, so the ask read as proven: {} at {:?}",
            m.says,
            m.target
        );
    }
}

#[test]
fn a_run_dated_past_now_is_not_its_latest() {
    let asks = [landed("fails now", 5 * 3600, "A", &json!({}))];
    let probes = [
        run("fails now", -DAY, true, 1201),
        run("fails now", 3600, false, 1200),
    ];
    let m = read(&asks, &probes);
    assert!(
        m.behind
            .first()
            .is_some_and(|line| line.contains("its probe failed after switch 1200")),
        "a pass dated tomorrow hid the failure since: {:?}",
        m.behind
    );
}

#[test]
fn an_empty_probe_names_none() {
    let ago = 3 * 3600;
    let m = read(
        &[
            landed("empty", ago, "A", &json!({"probe": ""})),
            landed("bare", ago, "B", &json!({"probe": {}})),
            landed("blank", ago, "C", &json!({"probe": "  "})),
            landed("listed", ago, "D", &json!({"probe": []})),
        ],
        &[],
    );
    assert!(
        m.says.contains("no ask names a probe yet"),
        "an empty probe counted as named: {}",
        m.says
    );
}

#[test]
fn who_answers_falls_back_to_the_asks_owner_then_its_keeper() {
    let ago = 3 * 3600;
    let m = read(
        &[
            landed(
                "by owner",
                ago,
                "",
                &json!({"answers": null, "owner": "Y2YBTS (WCJQSG)"}),
            ),
            landed(
                "by keeper",
                ago + 60,
                "",
                &json!({"answers": null, "owner": null}),
            ),
        ],
        &[],
    );
    assert_eq!(
        m.behind
            .iter()
            .filter_map(|line| line.rsplit_once(" · "))
            .map(|(_, owner)| owner)
            .collect::<Vec<_>>(),
        [KEEPER, "Y2YBTS"],
        "with no answers cell, the ask's first owner, else the list's keeper, answers"
    );
}

#[test]
fn words_cant_forge_the_separators_a_line_is_split_on() {
    let ago = 3 * 3600;
    // Nine words, so none is cut: a doubled separator, and one a
    // non-breaking space would close once shortened.
    let words = "a; b · C\nd · · z;\u{a0}w";
    let m = read(&[landed(words, ago, "A:B", &json!({}))], &[]);
    let line = &m.behind[0];
    assert_eq!(
        (line.matches("; ").count(), line.matches(" · ").count()),
        (1, 1),
        "an ask's words added a separator: {line}"
    );
    assert!(
        line.ends_with(" · A?B"),
        "an owner's colon would split its key: {line}"
    );
}
