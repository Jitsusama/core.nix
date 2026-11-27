//! Joel's asks unproven, read from asks lists and probe runs in the shapes
//! their keepers write. Long because each case pins one shape a keeper's
//! record or a switch has taken, and they read best side by side.

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
    unproven(&list(asks), Ok(&probes.join("\n")), Ok(""), &ANY, NOW)
}

/// A judge of switches that takes each to carry every change.
const ANY: fn(&Value, Repo, u32) -> Option<bool> = |_, _, _| Some(true);

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
            landed("just landed", GRACE - 1, "A", &json!({})),
            landed("at its grace", GRACE, "D", &json!({})),
            landed("at the week", SINCE, "E", &json!({})),
            landed("just past the week", SINCE + 1, "F", &json!({})),
            landed("long ago", 8 * DAY, "B", &json!({})),
            landed("longer ago", 30 * DAY, "C", &json!({})),
        ],
        &[],
    );
    assert!(
        (m.now - 5.0).abs() < 0.5,
        "two at the edges and three older asks counted, got {}",
        m.now
    );
    assert_eq!(
        m.behind
            .iter()
            .map(|line| line.split(", asked ").next().unwrap_or_default())
            .collect::<Vec<_>>(),
        [
            "at the week",
            "at its grace",
            &format!("3 older than a week, left to the probes' backfill · {KEEPER}"),
        ],
        "an ask still in its grace was counted, or one past the week was asked apart"
    );
}

#[test]
fn an_ask_landed_after_the_last_switch_says_it_isnt_live_yet() {
    let ago = 3 * 3600;
    let asks = list(&[landed("waits", ago, "A", &json!({}))]);
    let before = json!({"at": at(NOW - ago - 60)}).to_string();
    let m = unproven(&asks, Ok(""), Ok(&before), &ANY, NOW);
    assert!(
        m.behind[0].contains("; its probe hasn't run since it landed, not live yet,"),
        "an ask no switch has carried didn't say so: {}",
        m.behind[0]
    );
    let after = json!({"at": at(NOW - 60)}).to_string();
    let m = unproven(&asks, Ok(""), Ok(&format!("{before}\n{after}")), &ANY, NOW);
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
fn an_ask_he_confirmed_put_down_derived_or_still_open_needs_no_probe() {
    let ago = 3 * 3600;
    let m = read(
        &[
            landed("confirmed", ago, "A", &json!({"confirmed": at(NOW - 60)})),
            // Landed before it was put down, so only its being put down
            // takes it out.
            landed("put down", ago, "C", &json!({"closed": "put down"})),
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
fn a_landed_ask_closed_in_any_form_or_confirmed_in_words_needs_no_probe() {
    let ago = 3 * 3600;
    let closed = |words: &str, why: &str| landed(words, ago, "A", &json!({"closed": why}));
    let m = read(
        &[
            closed(
                "twin",
                "folded into its identical twin x at 2026-11-26T09:00:00-05:00: same",
            ),
            closed("superseded", "superseded (by another)"),
            closed("answered", "answered (in the thread)"),
            closed("done", "2026-11-26T09:00:00-05:00"),
            landed("agreed", ago, "B", &json!({"confirmed": true})),
            landed(
                "said so",
                ago,
                "B",
                &json!({"confirmed": "agent: he said it works"}),
            ),
            landed("still owed", ago, "C", &json!({})),
        ],
        &[],
    );
    assert_eq!(
        m.behind
            .iter()
            .map(|line| line.split(", asked ").next())
            .collect::<Vec<_>>(),
        [Some("still owed")],
        "a closed or confirmed ask was asked, or the one still owed wasn't"
    );
}

#[test]
fn an_ask_landed_at_his_own_offset_is_read_at_its_true_time() {
    // The list writes every time with -05:00, never Z; one landed an hour
    // ago by the clock there is an hour old, not inside the grace.
    let five = 5 * 3600;
    let local = |ago: i64| {
        let at = jiff::Timestamp::from_second(NOW - ago - five).expect("a time");
        format!("{}-05:00", at.strftime("%Y-%m-%dT%H:%M:%S"))
    };
    let late = landed("late", 3600, "A", &json!({"landed": local(3600)}));
    let old = landed("old", 3 * 3600, "A", &json!({"landed": local(3 * 3600)}));
    let m = read(&[late, old], &[]);
    assert_eq!(
        m.behind
            .iter()
            .map(|line| line.split(", asked ").next())
            .collect::<Vec<_>>(),
        [Some("old")],
        "an offset time was misread, so the grace slipped by five hours"
    );
}

#[test]
fn an_ask_parked_on_a_ruling_before_it_landed_is_still_asked_once_landed() {
    let parked = landed("parked", 3 * 3600, "B", &json!({"parked": "b95ea57b"}));
    let m = read(&[parked], &[]);
    assert!(
        m.behind
            .first()
            .is_some_and(|line| line.starts_with("parked, asked ")),
        "a landed ask its old ruling still names escaped the measure: {:?}",
        m.behind
    );
}

#[test]
fn a_pass_from_before_any_switch_since_it_landed_proves_nothing() {
    let ago = 3 * 3600;
    let asks = list(&[landed("early", ago, "A", &json!({}))]);
    let before = json!({"at": at(NOW - ago - 60)}).to_string();
    let early = run("early", 3600, true, 1199);
    let m = unproven(&asks, Ok(&early), Ok(&before), &ANY, NOW);
    assert!(
        m.behind.first().is_some_and(|line| {
            line.contains("; its probe passed before a switch could carry it, not live yet,")
        }),
        "a pass on what doesn't carry it proved the ask: {:?}",
        m.behind
    );
    // A switch since, then a pass after it, proves it; a pass between
    // landing and that switch still doesn't.
    let switch = json!({"at": at(NOW - 1800)}).to_string();
    let switches = format!("{before}\n{switch}");
    let m = unproven(&asks, Ok(&early), Ok(&switches), &ANY, NOW);
    assert!(
        m.behind
            .first()
            .is_some_and(|line| line.contains("passed before a switch")),
        "a pass from before the switch that carried it proved the ask: {:?}",
        m.behind
    );
    let late = [early, run("early", 600, true, 1200)].join("\n");
    let m = unproven(&asks, Ok(&late), Ok(&switches), &ANY, NOW);
    assert!(
        m.behind.is_empty(),
        "a pass after the switch that carried it didn't prove it: {:?}",
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
    let m = unproven(
        &list(&[first, latest]),
        Err("permission denied"),
        Ok(""),
        &ANY,
        NOW,
    );
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
    let twin = landed("kept", ago, "B", &json!({"id": "kept~1"}));
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
    std::fs::create_dir_all(home.path().join("integration")).expect("a folder for switches");
    let switch = json!({"at": at(NOW - 7200)}).to_string();
    std::fs::write(home.path().join(SWITCHES), switch).expect("a switch record");
    let asks = list(&[landed("proven", 3 * 3600, "A", &json!({}))]);
    let m = super::read(home.path(), home.path(), &[], &asks, NOW);
    assert_eq!(
        (m.target, m.behind.len()),
        (Some(0.0), 0),
        "a pass after a switch kept where their keepers write them wasn't read: {} at {:?}, {:?}",
        m.says,
        m.target,
        m.behind
    );
}

#[test]
fn a_switch_record_that_cant_be_read_leaves_it_unjudged() {
    let home = tempfile::tempdir().expect("a state folder");
    std::fs::create_dir_all(home.path().join("mission")).expect("a folder for the record");
    std::fs::write(home.path().join(PROBES), run("proven", 3600, true, 1200)).expect("a record");
    // A folder where the record is, so reading it fails as no file gone does.
    std::fs::create_dir_all(home.path().join(SWITCHES)).expect("a folder in its place");
    let asks = list(&[landed("proven", 3 * 3600, "A", &json!({}))]);
    let m = super::read(home.path(), home.path(), &[], &asks, NOW);
    assert_eq!(
        (m.target, m.says.contains("the switch record unread: ")),
        (None, true),
        "a switch record that couldn't be read let any pass count: {} at {:?}",
        m.says,
        m.target
    );
}

#[test]
fn a_probe_record_gone_leaves_it_unjudged() {
    let home = tempfile::tempdir().expect("a state folder");
    let asks = list(&[landed("proven", 3 * 3600, "A", &json!({}))]);
    let m = super::read(home.path(), home.path(), &[], &asks, NOW);
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
    let m = super::read(home.path(), home.path(), &[], &asks, NOW);
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
            // The older ask's id sorts after the newer's, so the lines
            // are seen to follow landing, not the id.
            landed(
                "by owner",
                ago + 60,
                "",
                &json!({"answers": null, "owner": "Y2YBTS (WCJQSG)"}),
            ),
            landed(
                "by keeper",
                ago,
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
        ["Y2YBTS", KEEPER],
        "with no answers cell, the ask's first owner, else the list's keeper, answers"
    );
    assert_eq!(
        m.owner, "Y2YBTS",
        "the measure isn't owned by its oldest line's cell"
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
        line.ends_with(" · AB"),
        "an owner's colon would split its key: {line}"
    );
}

#[test]
fn nothing_in_an_asks_words_or_cell_can_close_the_span_its_close_command_is_in() {
    let ago = 3 * 3600;
    let m = read(
        &[landed(
            "use `nix fmt` \"strictly\"",
            ago,
            "A`B\"C\u{1b}",
            &json!({}),
        )],
        &[],
    );
    let line = &m.behind[0];
    assert!(
        !line.contains(['`', '"', '\u{1b}']),
        "a backtick, double quote or escape reached the line: {line}"
    );
    assert!(
        line.starts_with("use 'nix fmt' 'strictly', asked ") && line.ends_with(" · ABC"),
        "the words or the cell were cleaned past reading: {line}"
    );
}

#[test]
fn a_run_before_the_switch_that_carries_it_is_said_so_whether_it_passed_or_failed() {
    let ago = 3 * 3600;
    let asks = list(&[landed("early", ago, "A", &json!({}))]);
    let switches = [ago + 60, 1800]
        .map(|before| json!({"at": at(NOW - before)}).to_string())
        .join("\n");
    let m = unproven(
        &asks,
        Ok(&run("early", 3600, false, 1199)),
        Ok(&switches),
        &ANY,
        NOW,
    );
    assert!(
        m.behind.first().is_some_and(|line| {
            line.contains("; its probe failed before a switch could carry it · A")
        }),
        "a failure on what doesn't carry it read as the change failing: {:?}",
        m.behind
    );
}

#[test]
fn an_ask_only_the_harness_carries_counts_a_pass_with_no_switch_since() {
    let ago = 3 * 3600;
    let harness = json!({"pr": "agentic-harness.pi#2326"});
    // A part naming no change found might be one a switch carries, so only
    // a switch since it landed counts.
    let unfound = json!({"pr": "agentic-harness.pi#2326, elsewhere#2"});
    let plain = json!({});
    let asks = list(&[
        landed("installed", ago, "A", &harness),
        landed("switched", ago, "B", &plain),
        landed("unfound", ago, "C", &unfound),
    ]);
    let before = json!({"at": at(NOW - ago - 60)}).to_string();
    let passes = ["installed", "switched", "unfound"].map(|ask| run(ask, 600, true, 1199));
    let m = unproven(&asks, Ok(&passes.join("\n")), Ok(&before), &ANY, NOW);
    assert_eq!(
        m.behind
            .iter()
            .map(|line| line.split(", asked ").next())
            .collect::<Vec<_>>(),
        [Some("switched"), Some("unfound")],
        "a harness ask waited on a switch, or one a switch may carry didn't"
    );
    assert!(
        m.behind[1].contains("passed before a switch could carry it"),
        "a part naming no change found let a pass count: {}",
        m.behind[1]
    );
}

#[test]
fn a_switch_record_in_another_shape_leaves_the_measure_unjudged() {
    let ago = 3 * 3600;
    let asks = list(&[landed("waits", ago, "A", &json!({}))]);
    let passed = run("waits", 600, true, 1199);
    let shape = json!({"at": 1_794_000_000}).to_string();
    for (switches, why) in [
        (Ok(shape.as_str()), "no line names when it ran"),
        (Err("permission denied"), "permission denied"),
    ] {
        let m = unproven(&asks, Ok(&passed), switches, &ANY, NOW);
        assert_eq!(
            (
                m.target,
                m.says.contains(&format!("the switch record unread: {why}"))
            ),
            (None, true),
            "a switch record read as none let any pass count: {}",
            m.says
        );
    }
}

/// Runs git in `repo` at `when`, apart from the account's config, so it
/// neither signs nor asks who commits, and says what it printed.
fn git(repo: &Path, args: &[&str], when: i64) -> String {
    let date = format!("@{when} +0000");
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "here")
        .env("GIT_AUTHOR_EMAIL", "here@example.org")
        .env("GIT_COMMITTER_NAME", "here")
        .env("GIT_COMMITTER_EMAIL", "here@example.org")
        .env("GIT_AUTHOR_DATE", &date)
        .env("GIT_COMMITTER_DATE", &date)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// Commits what's staged in `repo`, or nothing, as `subject` at `when`,
/// and says the commit.
fn commit(repo: &Path, subject: &str, when: i64) -> String {
    git(
        repo,
        &["commit", "-q", "--allow-empty", "-m", subject],
        when,
    );
    git(repo, &["rev-parse", "HEAD"], when)
}

/// A scratch repository at `path`, a commit before the merge of change
/// `number` and the merge, three hours ago; says the two.
fn scratch(path: &Path, number: u32) -> (String, String) {
    std::fs::create_dir_all(path).expect("a scratch repository");
    git(path, &["init", "-q", "-b", "main"], NOW);
    let old = commit(path, "before", NOW - 3 * 3600 - 600);
    let merge = commit(path, &format!("feat: it (#{number})"), NOW - 3 * 3600 - 60);
    (old, merge)
}

/// A state folder holding a pass of the ask "card" half an hour ago, and
/// room for the switches.
fn state() -> tempfile::TempDir {
    let home = tempfile::tempdir().expect("a state folder");
    std::fs::create_dir_all(home.path().join("mission")).expect("a folder for the record");
    std::fs::write(home.path().join(PROBES), run("card", 1800, true, 1200)).expect("a record");
    std::fs::create_dir_all(home.path().join("integration")).expect("a folder for switches");
    home
}

/// A switch `ago` seconds before now to `generation`, built from `core`.
fn switch(ago: i64, generation: u64, core: &str) -> String {
    json!({"at": at(NOW - ago), "generation": generation, "revs": {"core": core}}).to_string()
}

/// The measure of the ask "card" landed three hours ago naming `pr`, kept
/// in `home` with `switches`, on a machine running `running`, with
/// `checkouts` besides core's.
fn measured(
    home: &Path,
    pr: &str,
    switches: &[String],
    running: u64,
    checkouts: &[String],
) -> Measure {
    let asks = list(&[landed("card", 3 * 3600, "A", &json!({"pr": pr}))]);
    std::fs::write(home.join(SWITCHES), switches.join("\n")).expect("switches");
    super::read_on(home, home, checkouts, &asks, NOW, Some(running))
}

#[test]
fn a_switch_counts_only_once_the_core_it_built_holds_the_merge() {
    let home = state();
    let (old, merge) = scratch(&home.path().join("core.git"), 6835);
    let over = |switches: &[String]| measured(home.path(), "core.nix#6835", switches, 7, &[]);
    // Built from a core before the merge, the pass ran on old code.
    let m = over(&[switch(3600, 7, &old)]);
    assert!(
        m.behind.first().is_some_and(|line| line.contains(
            "passed before a switch could carry it, not live yet, as the switch running \
             doesn't carry it"
        )),
        "a pass on a switch built before the merge proved it: {:?}",
        m.behind
    );
    // A revision core's checkout lacks can't be judged, so isn't live, and
    // isn't the cell's to mend.
    let m = over(&[switch(3600, 7, "0123456789abcdef")]);
    assert!(
        m.behind.first().is_some_and(|line| {
            line.contains("its probe passed on a switch cadence can't judge")
                && line.contains(BLIND)
        }),
        "a switch that couldn't be judged counted as carrying it: {:?}",
        m.behind
    );
    // Built from the merge, the pass after it proves it.
    let m = over(&[switch(3600, 6, &old), switch(2400, 7, &merge)]);
    assert_eq!(
        (m.target, m.behind.len()),
        (Some(0.0), 0),
        "a pass on a switch holding the merge didn't prove it: {} {:?}",
        m.says,
        m.behind
    );
}

#[test]
fn a_switch_back_to_what_lacks_the_merge_holds_the_ask_back() {
    let home = state();
    let (old, merge) = scratch(&home.path().join("core.git"), 6835);
    let over = |switches: &[String], running: u64| {
        measured(home.path(), "core.nix#6835", switches, running, &[])
    };
    // Switched to the merge, then to a core before it: the pass ran on the
    // later, whatever an earlier switch held.
    let m = over(&[switch(3600, 6, &merge), switch(2400, 7, &old)], 7);
    assert!(
        m.behind.first().is_some_and(|line| line.contains(
            "passed before a switch could carry it, not live yet, as the switch running \
             doesn't carry it"
        )),
        "a pass after a switch back to old code proved it: {:?}",
        m.behind
    );
    // Rolled back, the record's last switch no longer runs, so nothing
    // since can be judged by it.
    let m = over(&[switch(3600, 6, &old), switch(2400, 7, &merge)], 6);
    assert!(
        m.behind.first().is_some_and(|line| line.contains(BLIND)),
        "a pass after a rollback proved what no longer runs: {:?}",
        m.behind
    );
}

#[test]
fn a_merge_or_checkout_cadence_cant_find_leaves_the_ask_unjudged() {
    let home = state();
    let (old, merge) = scratch(&home.path().join("core.git"), 6835);
    let switches = [switch(3600, 6, &old), switch(2400, 7, &merge)];
    for pr in ["core.nix#9999", "world#5"] {
        let m = measured(home.path(), pr, &switches, 7, &[]);
        assert!(
            m.behind.first().is_some_and(|line| line.contains(BLIND)),
            "{pr}, which cadence can't find, counted as carried: {:?}",
            m.behind
        );
        // Its line stays on the card, uncounted, so the hour's outage
        // isn't the drag, and with nothing else there the measure can't
        // say it's in.
        assert_eq!(
            (m.now, m.target, m.owner.as_str()),
            (0.0, None, ""),
            "{pr}, which only cadence's sight holds, counted, named its cell or read as in: {}",
            m.says
        );
        assert!(
            m.says
                .contains("cadence can't tell whether 1 of them are live"),
            "an hour blind to every ask didn't say so: {}",
            m.says
        );
    }
}

#[test]
fn a_switch_before_landing_that_git_cant_judge_leaves_the_ask_unjudged() {
    let ago = 3 * 3600;
    let asks = list(&[landed("card", ago, "A", &json!({"pr": "core.nix#6835"}))]);
    let before = json!({"at": at(NOW - ago - 30)}).to_string();
    let unjudged = |_: &Value, _: Repo, _: u32| None;
    let m = unproven(
        &asks,
        Ok(&run("card", 600, true, 1199)),
        Ok(&before),
        &unjudged,
        NOW,
    );
    assert!(
        m.behind.first().is_some_and(|line| blind(line)),
        "a switch before landing that git couldn't judge was put to the cell: {:?}",
        m.behind
    );
    assert_eq!(
        (m.now, m.target),
        (0.0, None),
        "a switch before landing that git couldn't judge was counted or read as in: {}",
        m.says
    );
}

#[test]
fn a_switch_before_landing_counts_only_for_an_ask_whose_every_part_is_a_merge_it_holds() {
    let ago = 3 * 3600;
    let asks = list(&[
        landed("merged", ago, "A", &json!({"pr": "core.nix#6835"})),
        // The harness's install isn't read for a switch.
        landed(
            "mixed",
            ago,
            "B",
            &json!({"pr": "agentic-harness.pi#2326, core.nix#6835"}),
        ),
        landed(
            "unfound",
            ago,
            "C",
            &json!({"pr": "core.nix#6835, elsewhere#2"}),
        ),
    ]);
    let before = json!({"at": at(NOW - ago - 30)}).to_string();
    let passes = ["merged", "mixed", "unfound"].map(|ask| run(ask, 600, true, 1199));
    let m = unproven(&asks, Ok(&passes.join("\n")), Ok(&before), &ANY, NOW);
    let mut waits: Vec<_> = m
        .behind
        .iter()
        .filter(|line| line.contains("passed before a switch could carry it"))
        .filter_map(|line| line.split(", asked ").next())
        .collect();
    waits.sort_unstable();
    assert_eq!(
        (waits, m.behind.len()),
        (vec!["mixed", "unfound"], 2),
        "a switch before landing proved a part it can't be shown to hold, or not the merge: {:?}",
        m.behind
    );
}

#[test]
fn a_line_only_cadences_sight_holds_names_no_owner() {
    let world = json!({"pr": "world#5"});
    let core = json!({"pr": "core.nix#6835"});
    // Cadence can judge core but not World.
    let judge = |_: &Value, repo: Repo, _: u32| (repo == Repo::Core).then_some(true);
    let switched = json!({"at": at(NOW - 2 * 3600)}).to_string();
    let failed = run("core", 600, false, 1200);
    let blind_first = list(&[
        landed("world", 4 * 3600, "A", &world),
        landed("core", 3 * 3600, "B", &core),
    ]);
    let m = unproven(&blind_first, Ok(&failed), Ok(&switched), &judge, NOW);
    assert_eq!(
        (m.now, m.owner.as_str(), m.behind.len()),
        (1.0, "B", 2),
        "a blind line counted or named the owner ahead of one its cell can mend: {:?}",
        m.behind
    );
    let beside_the_keepers = list(&[
        landed("world", 4 * 3600, "A", &world),
        landed("long ago", 8 * DAY, "C", &json!({})),
    ]);
    let m = unproven(&beside_the_keepers, Ok(""), Ok(&switched), &judge, NOW);
    assert_eq!(
        (m.now, m.owner.as_str()),
        (1.0, KEEPER),
        "a blind line named the owner ahead of the keeper's: {:?}",
        m.behind
    );
    let alone = list(&[landed("world", 4 * 3600, "A", &world)]);
    let m = unproven(&alone, Ok(""), Ok(&switched), &judge, NOW);
    assert_eq!(
        (m.owner.as_str(), m.target, sightless(&m)),
        ("", None, true),
        "a card only cadence's sight holds named a cell, read as judged or didn't say why: {}",
        m.says
    );
}

#[test]
fn an_asks_own_words_cant_mark_its_line_as_one_only_cadences_sight_holds() {
    let words = format!("fix it{BLIND}");
    let core = json!({"pr": "core.nix#6835"});
    let judge = |_: &Value, _: Repo, _: u32| Some(true);
    let switched = json!({"at": at(NOW - 2 * 3600)}).to_string();
    let asks = list(&[landed(&words, 3 * 3600, "B", &core)]);
    let m = unproven(
        &asks,
        Ok(&run(&words, 600, false, 1200)),
        Ok(&switched),
        &judge,
        NOW,
    );
    assert_eq!(
        (m.now, m.owner.as_str(), m.target),
        (1.0, "B", Some(0.0)),
        "an ask quoting cadence's blindness went uncounted or off its cell: {:?}",
        m.behind
    );
    assert!(
        m.behind.first().is_some_and(|line| !blind(line)),
        "an ask's own words marked its line blind: {:?}",
        m.behind
    );
    assert!(
        !sightless(&m),
        "a card judged read as sightless: {}",
        m.says
    );
}

#[test]
fn a_switch_before_its_ask_was_marked_landed_counts_once_git_shows_it_holds_the_merge() {
    let home = state();
    let (old, merge) = scratch(&home.path().join("core.git"), 6835);
    // Merged, switched, then marked landed: git shows what the mark can't.
    let between = 3 * 3600 + 30;
    let over = |switches: &[String]| measured(home.path(), "core.nix#6835", switches, 7, &[]);
    let m = over(&[switch(between, 7, &merge)]);
    assert_eq!(
        (m.target, m.behind.len()),
        (Some(0.0), 0),
        "a switch holding the merge before its landed mark didn't count: {} {:?}",
        m.says,
        m.behind
    );
    let m = over(&[switch(between, 7, &old)]);
    assert!(
        m.behind
            .first()
            .is_some_and(|line| line.contains("as the switch running doesn't carry it")),
        "a switch from before landing lacking the merge counted: {:?}",
        m.behind
    );
}

#[test]
fn a_failure_on_what_carries_it_is_its_cells_while_cadence_cant_tell_what_runs() {
    let home = state();
    std::fs::write(home.path().join(PROBES), run("card", 3000, false, 6)).expect("a run");
    let (_, merge) = scratch(&home.path().join("core.git"), 6835);
    // Rolled back to 6, which carries it, so 7 can't be judged.
    let switches = [switch(3600, 6, &merge), switch(2400, 7, &merge)];
    let m = measured(home.path(), "core.nix#6835", &switches, 6, &[]);
    assert!(
        m.behind.first().is_some_and(|line| {
            line.ends_with(
                "; its probe failed after switch 6, though cadence can't tell whether the \
                 switch running now carries it · A",
            ) && !line.contains(BLIND)
        }),
        "a failure on a switch that carries it was kept from its cell: {:?}",
        m.behind
    );
    assert_eq!(
        (m.now, m.owner.as_str()),
        (1.0, "A"),
        "a failure on a switch that carries it wasn't counted as its cell's: {}",
        m.says
    );
}

#[test]
fn an_emptied_switch_record_leaves_it_unjudged() {
    let home = state();
    let m = measured(home.path(), "core.nix#6835", &[], 7, &[]);
    assert_eq!(
        (
            m.target,
            m.says.contains("the switch record unread: emptied")
        ),
        (None, true),
        "an emptied switch record read as a machine never switched: {}",
        m.says
    );
}

#[test]
fn an_ask_past_the_week_judges_only_the_switch_its_run_was_on() {
    let (old, pr) = (8 * DAY, json!({"pr": "core.nix#6835"}));
    let first = at(NOW - old + 60);
    // Only the switch its run was on carries it; the one running now doesn't.
    let calls = std::cell::Cell::new(0);
    let judge = |line: &Value, _: Repo, _: u32| {
        calls.set(calls.get() + 1);
        Some(line["at"] == first)
    };
    let asks = list(&[landed("unrun", old, "A", &pr), landed("ran", old, "B", &pr)]);
    let switches = format!("{}\n{}", json!({"at": &first}), json!({"at": at(NOW - 60)}));
    let ran = run("ran", old - 120, true, 1200);
    let m = unproven(&asks, Ok(&ran), Ok(&switches), &judge, NOW);
    assert_eq!(
        (calls.get(), m.behind.len()),
        (1, 1),
        "an ask past the week judged more than its run's switch: {:?}",
        m.behind
    );
    assert!(
        m.behind[0].starts_with("1 older than a week"),
        "a pass on the switch that carried it didn't prove a week-old ask: {:?}",
        m.behind
    );
}

#[test]
fn a_mycelium_change_counts_once_the_core_switched_to_pins_it() {
    let home = state();
    let core = home.path().join("core.git");
    let mycelium = home.path().join("mycelium");
    let (pinned_old, pinned_merge) = scratch(&mycelium, 5);
    std::fs::create_dir_all(&core).expect("a scratch core");
    git(&core, &["init", "-q", "-b", "main"], NOW);
    let pinning = |rev: &str, when: i64| {
        let lock = json!({"nodes": {
            "root": {"inputs": {"mycelium": "mycelium"}},
            "mycelium": {"locked": {"rev": rev}},
        }});
        std::fs::write(core.join("flake.lock"), lock.to_string()).expect("a lock");
        git(&core, &["add", "flake.lock"], when);
        commit(&core, "chore: pin", when)
    };
    let lags = pinning(&pinned_old, NOW - 3 * 3600);
    let pins = pinning(&pinned_merge, NOW - 3 * 3600 + 60);
    let checkouts = [format!("mycelium={}", mycelium.display())];
    let over = |switches: &[String]| measured(home.path(), "mycelium#5", switches, 7, &checkouts);
    let m = over(&[switch(3600, 7, &lags)]);
    assert!(
        m.behind
            .first()
            .is_some_and(|line| line.contains("as the switch running doesn't carry it")),
        "a core pinning mycelium before its merge carried it: {:?}",
        m.behind
    );
    let m = over(&[switch(3600, 6, &lags), switch(2400, 7, &pins)]);
    assert_eq!(
        (m.target, m.behind.len()),
        (Some(0.0), 0),
        "a pass on a core pinning the merge didn't prove it: {} {:?}",
        m.says,
        m.behind
    );
}

#[test]
fn the_harness_part_of_an_ask_waits_on_its_install_not_the_switch() {
    let pr = json!({"pr": "agentic-harness.pi#2326, core.nix#6835"});
    let asks = list(&[landed("mixed", 3 * 3600, "A", &pr)]);
    let switch = json!({"at": at(NOW - 3600)}).to_string();
    let judge = |_: &Value, repo: Repo, _: u32| (repo != Repo::Harness).then_some(true);
    let passed = run("mixed", 600, true, 1200);
    let m = unproven(&asks, Ok(&passed), Ok(&switch), &judge, NOW);
    assert!(
        m.behind.is_empty(),
        "a harness part no switch names held back a pass on the core: {:?}",
        m.behind
    );
}

#[test]
fn a_part_a_switch_doesnt_carry_holds_it_back() {
    let ago = 3 * 3600;
    let pr = json!({"pr": "agentic-harness.pi#2326, mycelium#5"});
    let asks = list(&[landed("mixed", ago, "A", &pr)]);
    let switch = json!({"at": at(NOW - 3600)}).to_string();
    let judge = |_: &Value, repo: Repo, _: u32| match repo {
        Repo::Harness => None,
        _ => Some(false),
    };
    let passed = run("mixed", 600, true, 1200);
    let m = unproven(&asks, Ok(&passed), Ok(&switch), &judge, NOW);
    assert!(
        m.behind
            .first()
            .is_some_and(|line| line.contains("as the switch running doesn't carry it")),
        "a part the switch doesn't carry let a pass count: {:?}",
        m.behind
    );
}

#[test]
fn a_run_at_the_second_its_switch_came_counts() {
    let asks = list(&[landed("same", 3 * 3600, "A", &json!({}))]);
    let switch = json!({"at": at(NOW - 1800)}).to_string();
    let passed = run("same", 1800, true, 1200);
    let m = unproven(&asks, Ok(&passed), Ok(&switch), &ANY, NOW);
    assert!(
        m.behind.is_empty(),
        "a pass at its switch's second didn't count: {:?}",
        m.behind
    );
}

#[test]
fn probes_kept_before_any_switch_record_count_any_pass_since_landing() {
    let home = tempfile::tempdir().expect("a state folder");
    std::fs::create_dir_all(home.path().join("mission")).expect("a folder for the record");
    std::fs::write(home.path().join(PROBES), run("proven", 3600, true, 1200)).expect("a record");
    let asks = list(&[landed("proven", 3 * 3600, "A", &json!({}))]);
    let m = super::read(home.path(), home.path(), &[], &asks, NOW);
    assert_eq!(
        (m.target, m.behind.len()),
        (Some(0.0), 0),
        "a machine never switched read its pass as unproven: {} {:?}",
        m.says,
        m.behind
    );
}

#[test]
fn marks_that_hide_text_and_a_time_not_written_as_one_stay_out_of_the_line() {
    let ago = 3 * 3600;
    let m = read(
        &[
            landed(
                "a\u{202e}b\u{200b}c\u{061c}d\ne",
                ago,
                "A",
                &json!({"at": NOW}),
            ),
            landed("plain", ago - 60, "A", &json!({"at": "soon"})),
        ],
        &[],
    );
    assert_eq!(
        m.behind
            .iter()
            .map(|line| line.split("; ").next().unwrap_or_default())
            .collect::<Vec<_>>(),
        ["abcd e, asked ?", "plain, asked ?"],
        "a mark hiding text, or a time not written as one, reached the line"
    );
}
