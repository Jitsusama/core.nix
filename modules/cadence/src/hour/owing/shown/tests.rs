use super::super::super::measure;
use super::super::super::put::asks;
use super::super::{Class, Measure, file};
use super::*;
use crate::findings::hubs::put_late;
use crate::findings::shown::LIVE_WITHIN;

const NOW: i64 = 1_800_000_000;

fn red(key: &str) -> Measure {
    measure(key, Class::Slip, 3.0, Some(0.0))
}

fn back(key: &str) -> Measure {
    measure(key, Class::Slip, 0.0, Some(0.0))
}

/// A message on the mesh, its id stamped at `at`, in seconds.
fn message(at: i64, from: &str, to: &str, re: Option<&str>, text: &str) -> Message {
    Message {
        v: 1,
        id: format!("{}-{from}-1-{at:08x}", at * 1000),
        at: String::new(),
        from: from.to_owned(),
        to: vec![format!("@{to}")],
        intent: "question".to_owned(),
        text: text.to_owned(),
        pid: 1,
        re: re.map(str::to_owned),
        supersedes: None,
        ends_hold: false,
    }
}

/// Files `measures` at `now`, then asks each owner as the hour does,
/// marking each line asked with its question's id; the questions sent.
fn file_and_ask(
    ledger: &mut Vec<Entry>,
    measures: &[Measure],
    date: &str,
    now: i64,
) -> Vec<Message> {
    let told = file(ledger, measures, date, now);
    asks(told)
        .into_iter()
        .map(|(owner, text)| {
            let question = message(now, "CADENCE", &owner, None, &text);
            super::super::super::put::mark_asked(ledger, &owner, now, &question.id);
            question
        })
        .collect()
}

/// Whether the hour, at `now`, would put `entry` to anyone as late.
fn goes_late(ledger: &mut [Entry], now: i64) -> bool {
    put_late(ledger, &[], |_| true, now);
    ledger.iter().any(|e| e.late)
}

#[test]
fn a_line_back_in_stays_open_never_late_until_its_fix_is_named() {
    let mut ledger = Vec::new();
    let questions = file_and_ask(&mut ledger, &[red("slips")], "2026-11-14", NOW);
    assert!(
        questions[0]
            .text
            .contains("`cadence owe --done 'hour:slips' 'FIX'`"),
        "the owner wasn't told how its line closes: {}",
        questions[0].text
    );
    file(&mut ledger, &[back("slips")], "2026-11-14", NOW + 3600);
    assert_eq!(
        (ledger[0].open, ledger[0].back.as_deref()),
        (true, Some("2026-11-14")),
        "a line back in closed on its figure alone, or wasn't marked back"
    );
    assert!(
        !goes_late(&mut ledger, NOW + 2 * DAY),
        "a line back in, waiting on its owner's word, went late"
    );
    assert_eq!(
        tally(&ledger, NOW + 3600).as_deref(),
        Some("Lines held to a fix shown: 1 back, its fix unnamed.\n"),
        "the card didn't count a line back with its fix unnamed"
    );
    ledger[0].shown.word = Some(Word {
        said: Said::Fix("core #7196".to_owned()),
        at: NOW + 4000,
    });
    file(&mut ledger, &[back("slips")], "2026-11-14", NOW + 7200);
    assert_eq!(
        (ledger[0].open, ledger[0].shown.ended.map(|e| e.how)),
        (false, Some(How::Fixed)),
        "a line back in with its fix named didn't close as fixed"
    );
}

#[test]
fn a_fix_named_while_red_waits_for_the_figure_to_come_back() {
    let mut ledger = Vec::new();
    file(&mut ledger, &[red("slips")], "2026-11-14", NOW);
    ledger[0].shown.word = Some(Word {
        said: Said::Fix("core #7196".to_owned()),
        at: NOW + 100,
    });
    file(&mut ledger, &[red("slips")], "2026-11-14", NOW + 3600);
    assert!(
        ledger[0].open,
        "a line still past its target closed on its owner's word alone"
    );
}

#[test]
fn a_reply_naming_a_change_closes_its_line_once_the_change_runs_here() {
    let mut ledger = Vec::new();
    let questions = file_and_ask(&mut ledger, &[red("slips")], "2026-11-14", NOW);
    let reply = message(
        NOW + 600,
        "OWNER",
        "CADENCE",
        Some(&questions[0].id),
        "OWNER: landed core #7196, which stops it.",
    );
    hear(&mut ledger, &questions, &[reply]);
    assert_eq!(
        changes(&ledger),
        [("landed core #7196, which stops it.".to_owned(), NOW + 600)],
        "the owner's reply wasn't kept as the line's word, past its address"
    );
    let live =
        |live: Live| BTreeMap::from([("landed core #7196, which stops it.".to_owned(), live)]);
    judge(&mut ledger, &live(Live::Merged));
    assert!(
        !goes_late(&mut ledger, NOW + DAY + 1),
        "a line whose named fix merged but doesn't run yet went late on its owner"
    );
    file(&mut ledger, &[back("slips")], "2026-11-14", NOW + 3600);
    assert!(
        ledger[0].open,
        "a line closed on a change that doesn't run here yet"
    );
    assert_eq!(
        tally(&ledger, NOW + 3600).as_deref(),
        Some("Lines held to a fix shown: 1 its fix named, not yet live.\n"),
        "the card didn't count a fix named but not yet live apart"
    );
    assert_eq!(
        tally(&ledger, NOW + 600 + LIVE_WITHIN).as_deref(),
        Some("Lines held to a fix shown: 1 back, its fix unnamed.\n"),
        "a fix not live two days on still held the line as named"
    );
    judge(&mut ledger, &live(Live::Runs));
    file(&mut ledger, &[back("slips")], "2026-11-14", NOW + 7200);
    assert_eq!(
        (ledger[0].open, ledger[0].shown.ended.map(|e| e.how)),
        (false, Some(How::Fixed)),
        "a line back in whose named change runs here didn't close as fixed"
    );
}

#[test]
fn a_word_from_anyone_but_the_lines_owner_or_its_askee_is_left_out() {
    let mut ledger = Vec::new();
    let questions = file_and_ask(&mut ledger, &[red("slips")], "2026-11-14", NOW);
    let reply = message(
        NOW + 600,
        "STRANGER",
        "CADENCE",
        Some(&questions[0].id),
        "None: it fixed itself.",
    );
    hear(&mut ledger, &questions, &[reply]);
    assert_eq!(
        ledger[0].shown.word, None,
        "another session's reply named the fix of a line not theirs"
    );
}

#[test]
fn a_numbered_answer_names_the_fix_only_of_the_line_its_number_asked() {
    let mut ledger = Vec::new();
    let questions = file_and_ask(
        &mut ledger,
        &[red("slips"), red("parked-days")],
        "2026-11-14",
        NOW,
    );
    assert_eq!(
        questions.len(),
        1,
        "the owner's two lines weren't one question"
    );
    let numbered = |key: &str| {
        let needle = causes::shell_quoted(&format!("hour:{key}"));
        questions[0]
            .text
            .split("\n\n")
            .find(|p| p.contains(&needle))
            .and_then(|p| p.split_once(". "))
            .map(|(n, _)| n.to_owned())
            .expect("each line is numbered with its key")
    };
    let text = format!(
        "{}. None: the slip was a holiday.\n{}: Landed core #7196\nwhich stops it.",
        numbered("slips"),
        numbered("parked-days")
    );
    let reply = message(NOW + 600, "OWNER", "CADENCE", Some(&questions[0].id), &text);
    hear(&mut ledger, &questions, &[reply]);
    let word = |key: &str| {
        ledger
            .iter()
            .find(|e| e.key == format!("hour:{key}"))
            .and_then(|e| e.shown.word.as_ref())
            .map(|w| w.said.clone())
    };
    assert_eq!(
        (word("slips"), word("parked-days")),
        (
            Some(Said::Unneeded("the slip was a holiday.".to_owned())),
            Some(Said::Change {
                text: "Landed core #7196 which stops it.".to_owned(),
                live: Live::No
            })
        ),
        "an item's word went to another line than its number asked"
    );
}

#[test]
fn a_line_needing_no_fix_closes_once_back_counted_apart() {
    let mut ledger = Vec::new();
    let questions = file_and_ask(&mut ledger, &[red("slips")], "2026-11-14", NOW);
    let reply = message(
        NOW + 600,
        "OWNER",
        "CADENCE",
        Some(&questions[0].id),
        "none: a holiday, nothing to fix.",
    );
    hear(&mut ledger, &questions, &[reply]);
    file(&mut ledger, &[red("slips")], "2026-11-14", NOW + 3600);
    assert!(
        ledger[0].open,
        "a line needing no fix closed while still red"
    );
    file(&mut ledger, &[back("slips")], "2026-11-14", NOW + 7200);
    assert_eq!(
        tally(&ledger, NOW + 7200).as_deref(),
        Some("Lines held to a fix shown: 1 back, no fix needed, this week.\n"),
        "a line needing no fix didn't close once back, counted apart"
    );
}

#[test]
fn a_line_back_a_week_unexplained_closes_and_a_fall_reopens_its_own_line() {
    let mut ledger = Vec::new();
    file(&mut ledger, &[red("slips")], "2026-11-14", NOW);
    file(&mut ledger, &[back("slips")], "2026-11-14", NOW + 3600);
    file(&mut ledger, &[back("slips")], "2026-11-20", NOW + 6 * DAY);
    assert!(ledger[0].open, "a line closed unexplained before its week");
    file(&mut ledger, &[back("slips")], "2026-11-21", NOW + 7 * DAY);
    assert_eq!(
        (ledger[0].open, ledger[0].shown.ended.map(|e| e.how)),
        (false, Some(How::Unexplained)),
        "a line back a week with no word didn't close unexplained"
    );
    assert_eq!(
        tally(&ledger, NOW + 7 * DAY).as_deref(),
        Some("Lines held to a fix shown: 1 back unexplained this week.\n"),
        "the card didn't count the week's line back unexplained"
    );
    file(&mut ledger, &[red("slips")], "2026-11-22", NOW + 8 * DAY);
    assert_eq!(
        (
            ledger.len(),
            ledger[0].open,
            ledger[0].reopened,
            ledger[0].shown.is_empty()
        ),
        (1, true, 1, true),
        "a fall after closing unexplained didn't reopen its own line, counted"
    );
}

/// A line routed to each cell's owner closes as its cell goes, and the
/// cell idle again within the week reopens its owner's own line, counted,
/// so it reads as a recurrence rather than a fresh line (1JQPPC's rider
/// of 27 November).
#[test]
fn a_routed_line_closes_as_its_cell_goes_and_reopens_counted_when_it_comes_back() {
    let idle = |behind: &[&str]| {
        let mut m = measure("cells-idle", Class::Cells, 0.0, Some(0.0));
        m.behind = behind.iter().map(|&line| line.to_owned()).collect();
        m.now = f64::from(u8::try_from(behind.len()).expect("a few cells"));
        m
    };
    let cell = "AAAAAA idle 30 min: write it · ORCH";
    let mut ledger = Vec::new();
    file(&mut ledger, &[idle(&[cell])], "2026-11-14", NOW);
    file(&mut ledger, &[idle(&[])], "2026-11-14", NOW + 3600);
    assert_eq!(
        (ledger.len(), ledger[0].open, ledger[0].shown.is_empty()),
        (1, false, true),
        "a routed line didn't close as its cell went, or waited on a fix shown"
    );
    let told = file(&mut ledger, &[idle(&[cell])], "2026-11-17", NOW + 3 * DAY);
    assert_eq!(
        (
            ledger.len(),
            ledger[0].open,
            ledger[0].reopened,
            told.iter().filter(|(to, _)| to == "ORCH").count()
        ),
        (1, true, 1, 1),
        "a cell idle again within the week didn't reopen its owner's line, counted"
    );
}

#[test]
fn a_line_with_a_rule_of_its_own_or_routed_to_each_owner_still_closes_by_it() {
    let slip = Class::Slip.name();
    for key in ["hour:queue:x", "hour:miss:x", "hour:causes-recurring:x"] {
        assert!(
            !held_to_a_fix(key, slip),
            "{key}, which closes by a rule of its own, was held to a fix shown"
        );
    }
    for class in [Class::Cells, Class::Clean] {
        assert!(
            !held_to_a_fix("hour:cells-idle:ABC123", class.name()),
            "a line routed to each owner on {} was held to a fix shown",
            class.name()
        );
    }
    assert!(
        held_to_a_fix("hour:slips", slip)
            && held_to_a_fix("hour:relay-x", Class::Functional.name()),
        "a drag or red line wasn't held to a fix shown"
    );
}
