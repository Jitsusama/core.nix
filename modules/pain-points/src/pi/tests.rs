//! Joel's turns as pi.rs reads them from hand-built lines in pi's own
//! log shape, and the harness's: his messages, the calls answering them,
//! errors, retries, compactions and peers' messages crossing them.

use super::*;
use crate::pain::Signs;
use std::io::Cursor;

/// 2026-11-11 16:00:00 UTC.
pub(super) const AT: i64 = 1_794_412_800_000;

fn iso(at: i64) -> String {
    let secs = at.div_euclid(1000);
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    assert_eq!(days, 20_768, "these tests stay on 11 November 2026");
    format!(
        "2026-11-11T{:02}:{:02}:{:02}.{:03}Z",
        rest / 3600,
        rest / 60 % 60,
        rest % 60,
        at.rem_euclid(1000)
    )
}

fn typed(enter: i64, delivered: i64, text: &str) -> String {
    format!(
        r#"{{"type":"message","id":"a","parentId":"b","timestamp":"{}","message":{{"role":"user","content":[{{"type":"text","text":{}}}],"timestamp":{enter}}}}}"#,
        iso(delivered),
        serde_json::to_string(text).expect("a string serializes")
    ) + "\n"
}

fn reply(start: i64, ms: i64, stop: &str) -> String {
    format!(
        r#"{{"type":"message","id":"c","parentId":"a","timestamp":"{}","message":{{"role":"assistant","content":[{{"type":"text","text":"an answer"}}],"api":"x","timestamp":{start},"durationMs":{ms},"stopReason":"{stop}"}}}}"#,
        iso(start + ms)
    ) + "\n"
}

fn tool_result(at: i64, text: &str) -> String {
    format!(
        r#"{{"type":"message","id":"d","parentId":"c","timestamp":"{}","message":{{"role":"toolResult","content":[{{"type":"text","text":{}}}],"timestamp":{at}}}}}"#,
        iso(at),
        serde_json::to_string(text).expect("a string serializes")
    ) + "\n"
}

fn tool_ended(at: i64, name: &str) -> String {
    format!(
        r#"{{"type":"message","id":"d","parentId":"c","timestamp":"{}","message":{{"role":"toolResult","toolCallId":"t","toolName":"{name}","content":[{{"type":"text","text":"done"}}],"timestamp":{at}}}}}"#,
        iso(at)
    ) + "\n"
}

fn harness_call(of: i64, first_token: i64) -> String {
    format!(
        r#"{{"type":"custom","customType":"harness-call","data":{{"v":1,"of":{of},"turnAt":"{}","preparedMs":5,"firstTokenMs":{first_token},"durationMs":9000,"stopReason":"stop"}},"id":"h","parentId":"a","timestamp":"{}"}}"#,
        iso(of),
        iso(of + 9000)
    ) + "\n"
}

fn compaction_asked(at: i64) -> String {
    format!(
        r#"{{"type":"custom","customType":"compaction-asked","data":{{"at":{at},"resume":true}},"id":"k","parentId":"d","timestamp":"{}"}}"#,
        iso(at)
    ) + "\n"
}

/// As the harness writes it, read from a session's log on 20 November
/// with its ids changed.
fn compaction_settled(at: i64) -> String {
    format!(
        r#"{{"type":"custom","customType":"compaction-settled","data":{{"at":{at},"resumed":false,"by":"asker"}},"id":"n","parentId":"k","timestamp":"{}"}}"#,
        iso(at)
    ) + "\n"
}

fn compaction(at: i64) -> String {
    format!(
        r#"{{"type":"compaction","id":"m","parentId":"k","timestamp":"{}","summary":"what was said"}}"#,
        iso(at)
    ) + "\n"
}

fn mesh(at: i64) -> String {
    format!(
        r#"{{"type":"custom_message","customType":"mesh","content":"Mesh message from a peer","timestamp":"{}"}}"#,
        iso(at)
    ) + "\n"
}

fn read(text: &str) -> Log {
    let mut log = Log::default();
    follow(&mut Cursor::new(text.as_bytes().to_vec()), &mut log).expect("a cursor reads");
    log
}

pub(super) fn turn(enter: i64, calls: &[(i64, i64, Stop)]) -> Turn {
    Turn {
        offset: 0,
        enter,
        delivered: enter,
        calls: calls
            .iter()
            .map(|&(start, ms, stop)| Call {
                start,
                ms,
                stop,
                first: None,
            })
            .collect(),
        ..Turn::default()
    }
}

#[test]
fn what_runs_after_his_turn_is_handed_back_is_not_his() {
    let text = typed(AT, AT, "hello")
        + &reply(AT + 100, 1000, "stop")
        + &mesh(AT + 5000)
        + &reply(AT + 5100, 2000, "error")
        + &reply(AT + 8000, 500, "aborted");
    let log = read(&text);
    assert_eq!(
        log.turns[0].calls.len(),
        1,
        "calls a mesh message set off after his reply aren't his turn's"
    );
    assert!(!log.open, "the answer handed the turn back");
}

#[test]
fn a_peers_message_after_an_error_takes_the_turn_unless_pi_tries_again() {
    let retry = || r#"{"type":"context_edit","targetId":"f","replacement":null}"#.to_owned() + "\n";
    // The call that errored is entry "f", which pi's retry names.
    let failed = reply(AT + 100, 1000, "error").replacen(r#""id":"c""#, r#""id":"f""#, 1);
    let errored = || typed(AT, AT, "hello") + &failed;
    // An errored entry with no id leaves pi's retry nothing to name, so
    // whatever entry it edits out counts.
    let nameless = typed(AT, AT, "hello")
        + &reply(AT + 100, 1000, "error").replacen(r#""id":"c""#, r#""id":"""#, 1);
    // An extension may edit another entry, or put text in its place.
    let other = r#"{"type":"context_edit","targetId":"c","replacement":null}"#.to_owned()
        + "\n"
        + r#"{"type":"context_edit","targetId":"f","replacement":"x"}"#
        + "\n";
    let answered = || reply(AT + 5100, 2000, "stop");
    for (text, stops, what) in [
        (
            errored() + &mesh(AT + 5000) + &answered(),
            vec![Stop::Error],
            "a peer's turn after an error pi never tried again",
        ),
        (
            errored() + &mesh(AT + 5000) + &other + &answered(),
            vec![Stop::Error],
            "a peer's turn after edits that aren't pi's retry",
        ),
        (
            errored() + &mesh(AT + 5000) + &retry() + &answered(),
            vec![Stop::Error, Stop::Done],
            "a peer's message before pi's retry",
        ),
        (
            errored() + &retry() + &mesh(AT + 5000) + &answered(),
            vec![Stop::Error, Stop::Done],
            "a peer's message during pi's retry",
        ),
        (
            nameless + &mesh(AT + 5000) + &retry() + &answered(),
            vec![Stop::Error, Stop::Done],
            "pi's retry of an errored entry with no id",
        ),
        (
            typed(AT, AT, "hello")
                + &reply(AT + 100, 1000, "toolUse")
                + &mesh(AT + 5000)
                + &answered(),
            vec![Stop::Tool, Stop::Done],
            "a peer's message mid-turn with no error",
        ),
    ] {
        let log = read(&text);
        let got: Vec<Stop> = log.turns[0].calls.iter().map(|c| c.stop).collect();
        assert_eq!(got, stops, "{what} was read wrongly");
        assert!(!log.open, "{what}: the turn was handed back at its end");
    }
}

#[test]
fn his_next_message_leaves_an_error_behind() {
    // His second message opens a turn of its own, so a peer's message
    // after it crosses nothing, and the call after that is his.
    let text = typed(AT, AT, "hello")
        + &reply(AT + 100, 1000, "error")
        + &typed(AT + 3000, AT + 3000, "try the launcher")
        + &mesh(AT + 4000)
        + &reply(AT + 4100, 2000, "stop");
    let log = read(&text);
    let stops: Vec<Vec<Stop>> = log
        .turns
        .iter()
        .map(|t| t.calls.iter().map(|c| c.stop).collect())
        .collect();
    assert_eq!(stops, [vec![Stop::Error], vec![Stop::Done]]);
    assert!(!log.open, "the answer handed his second turn back");
}

#[test]
fn an_abort_pi_makes_to_compact_is_not_his_stop() {
    // As a session's log read on 23 November: a tool's result, the
    // harness asking to compact, the call it cut ending at once, the
    // compaction, and the run resumed by an automated message.
    // Its first reply comes 2.5 s after Enter, so the go is named for
    // that wait and its stopped count can be read.
    let text = typed(AT, AT, "check the gate")
        + &reply(AT + 2500, 18_000, "toolUse")
        + &tool_ended(AT + 20_800, "edit")
        + &compaction_asked(AT + 20_860)
        + &reply(AT + 20_975, 0, "aborted")
        + &compaction(AT + 21_600)
        + &typed(
            AT + 22_200,
            AT + 22_300,
            "[Automated message, no human input: the context was compacted]",
        )
        + &reply(AT + 22_400, 9000, "toolUse");
    let log = read(&text);
    let stops: Vec<Stop> = log.turns[0].calls.iter().map(|c| c.stop).collect();
    assert_eq!(stops, [Stop::Tool, Stop::Compacted]);
    let named = painful(&[&log.turns], AT - 1, AT + 1);
    assert_eq!(named.len(), 1, "the go is named for its wait");
    let Signs::Pi(signs) = &named[0].signs else {
        panic!("a pi moment carries pi's signs");
    };
    assert_eq!(
        (signs.stopped, named[0].part),
        (0, "pi"),
        "pi cut its own call, he stopped nothing"
    );
}

#[test]
fn an_ask_ends_where_it_lands_settles_or_he_types() {
    // Each abort below is his: the ask before it was spent first, or
    // came too long before it ended.
    let asked = || {
        typed(AT, AT, "check the gate")
            + &reply(AT + 500, 1000, "toolUse")
            + &compaction_asked(AT + 2000)
    };
    let stop = || reply(AT + 3000, 1000, "aborted");
    for (text, what) in [
        (
            asked() + &compaction(AT + 2500) + &stop(),
            "an ask that landed between his calls",
        ),
        (
            asked() + &compaction_settled(AT + 2500) + &stop(),
            "an ask settled with no compaction",
        ),
        (
            typed(AT, AT, "check the gate")
                + &reply(AT + 500, 1000, "stop")
                + &compaction_asked(AT + 2000)
                + &typed(AT + 2500, AT + 2500, "now the launcher")
                + &stop(),
            "an ask never settled, before he typed again",
        ),
        (
            asked() + &reply(AT + 3000, 9000, "aborted"),
            "an abort ending long after the ask",
        ),
    ] {
        let log = read(&text);
        let last = log.turns.last().and_then(|t| t.calls.last());
        assert_eq!(last.map(|c| c.stop), Some(Stop::Aborted), "{what}");
        assert_eq!(log.compacting, None, "{what}: no ask is left");
    }
}

#[test]
fn only_an_abort_is_cut_and_any_call_spends_the_ask() {
    let asked = || {
        typed(AT, AT, "check the gate")
            + &reply(AT + 500, 1000, "toolUse")
            + &compaction_asked(AT + 2000)
    };
    // The call right after the ask ran on to a tool, so the ask cut
    // nothing; the abort after it, still within 5 s of the ask, is his.
    let log = read(&(asked() + &reply(AT + 2100, 500, "toolUse")));
    let stops: Vec<Stop> = log.turns[0].calls.iter().map(|c| c.stop).collect();
    assert_eq!(stops, [Stop::Tool, Stop::Tool], "a tool call isn't a cut");
    assert!(log.open, "a tool call keeps his turn going");
    assert_eq!(log.compacting, None, "the tool call spent the ask");
    let log = read(
        &(asked() + &reply(AT + 2100, 500, "toolUse") + &reply(AT + 3000, 1000, "aborted")),
    );
    let stops: Vec<Stop> = log.turns[0].calls.iter().map(|c| c.stop).collect();
    assert_eq!(
        stops,
        [Stop::Tool, Stop::Tool, Stop::Aborted],
        "an abort after the ask was spent is his"
    );
}

#[test]
fn a_tool_holding_the_session_past_a_minute_is_pis_own_and_the_model_is_not() {
    let text = typed(AT, AT, "run the tests")
        + &reply(AT + 500, 10_000, "toolUse")
        + &tool_ended(AT + 100_500, "bash")
        + &reply(AT + 101_000, 5000, "stop");
    let log = read(&text);
    assert_eq!(log.turns[0].tools_ms, 90_000, "the tool ran 90 s");
    let p = &painful(&[&log.turns], AT - 1, AT + 1)[0];
    assert_eq!(
        (p.lost_seconds, p.slowed, p.part),
        (30.5, "waiting", "pi"),
        "of 106 s, the model's 15 aren't lost, nor the tool's first minute"
    );
    let Signs::Pi(signs) = &p.signs else {
        panic!("a pi moment carries pi's signs");
    };
    assert_eq!(
        (
            signs.model_ms,
            signs.provider_ms,
            signs.pi_ms,
            signs.tools_ms
        ),
        (15_000, 0, 91_000, 90_000)
    );
}

#[test]
fn a_question_put_to_him_is_not_a_wait() {
    let text = typed(AT, AT, "plan it")
        + &reply(AT + 500, 10_000, "toolUse")
        + &tool_ended(AT + 40_500, "ask")
        + &reply(AT + 41_000, 5000, "stop");
    let log = read(&text);
    assert!(
        painful(&[&log.turns], AT - 1, AT + 1).is_empty(),
        "the 30 s were his own, answering"
    );
}

#[test]
fn the_first_token_comes_from_the_harness_entry_before_its_call() {
    let text =
        typed(AT, AT, "hello") + &harness_call(AT + 500, 4000) + &reply(AT + 500, 9000, "stop");
    let log = read(&text);
    assert_eq!(log.turns[0].calls[0].first, Some(4000));
    let named = painful(&[&log.turns], AT - 1, AT + 1);
    assert!(named.is_empty(), "the model thinking is no fault");
}

#[test]
fn a_typed_message_and_its_replies_make_a_turn() {
    let text = typed(AT, AT + 4000, "make the launcher faster")
        + &reply(AT + 4100, 3000, "toolUse")
        + &tool_result(AT + 7200, "ok")
        + &reply(AT + 7300, 2000, "stop");
    let log = read(&text);
    assert_eq!(log.turns.len(), 1, "one message, one turn");
    let turn = &log.turns[0];
    assert_eq!(
        (turn.offset, turn.enter, turn.delivered),
        (0, AT, AT + 4000)
    );
    assert_eq!(
        turn.calls,
        vec![
            Call {
                start: AT + 4100,
                ms: 3000,
                stop: Stop::Tool,
                first: None,
            },
            Call {
                start: AT + 7300,
                ms: 2000,
                stop: Stop::Done,
                first: None,
            },
        ]
    );
    assert_eq!(log.end, u64::try_from(text.len()).expect("a length fits"));
}

#[test]
fn messages_he_did_not_type_are_not_his_turns() {
    let text = typed(AT, AT, "hello")
        + &reply(AT + 100, 1000, "stop")
        + &typed(
            AT + 5000,
            AT + 5000,
            "[Automated message, no human input: carry on]",
        )
        + &reply(AT + 5100, 9000, "stop")
        + &typed(
            AT + 20_000,
            AT + 20_000,
            "[Joel is asking from outside this window] hi",
        )
        + &mesh(AT + 30_000);
    let log = read(&text);
    assert_eq!(log.turns.len(), 1, "only the typed message is his");
    assert_eq!(
        log.turns[0].calls.len(),
        1,
        "a reply to an automated message isn't his turn's"
    );
    assert!(!log.open, "the last message in the log wasn't his");
}

#[test]
fn a_read_picks_up_where_the_last_one_stopped() {
    let first = typed(AT, AT, "fix the launcher") + &reply(AT + 100, 1000, "toolUse");
    let rest =
        reply(AT + 2000, 1000, "stop") + &typed(AT + 60_000, AT + 60_000, "fix the launcher!");
    let whole = read(&(first.clone() + &rest));
    let mut log = Log::default();
    let unended = first.clone() + &rest[..10];
    follow(&mut Cursor::new(unended.into_bytes()), &mut log).expect("a cursor reads");
    assert_eq!(
        log.end,
        u64::try_from(first.len()).expect("a length fits"),
        "a line not yet ended waits for the next read"
    );
    follow(&mut Cursor::new((first + &rest).into_bytes()), &mut log).expect("a cursor reads");
    assert_eq!(log, whole, "two reads make what one would");
    assert!(
        log.turns[1].again,
        "the second is the first said again, across reads"
    );
}

#[test]
fn an_error_or_an_ask_is_carried_across_reads_and_kept_state() {
    // pain-points keeps each log's state between reads as JSON, so a read
    // may stop on any line, the error's, the peer's or the ask's
    // included, and the next pick up from what was kept.
    let failed = reply(AT + 100, 1000, "error").replacen(r#""id":"c""#, r#""id":"f""#, 1);
    let retry = r#"{"type":"context_edit","targetId":"f","replacement":null}"#.to_owned() + "\n";
    let answered = reply(AT + 5100, 2000, "stop");
    for (lines, what) in [
        (
            vec![typed(AT, AT, "hello"), failed.clone(), mesh(AT + 5000), answered.clone()],
            "a peer's turn after an error",
        ),
        (
            vec![
                typed(AT, AT, "hello"),
                failed,
                mesh(AT + 5000),
                retry,
                answered,
            ],
            "a peer's message before pi's retry",
        ),
        (
            vec![
                typed(AT, AT, "check the gate"),
                reply(AT + 500, 1000, "toolUse"),
                compaction_asked(AT + 2000),
                reply(AT + 2100, 0, "aborted"),
            ],
            "an abort to compact",
        ),
    ] {
        let text = lines.concat();
        let whole = read(&text);
        for cut in 1..lines.len() {
            let mut log = read(&lines[..cut].concat());
            let kept = serde_json::to_string(&log).expect("a log serializes");
            log = serde_json::from_str(&kept).expect("a kept log reads back");
            follow(&mut Cursor::new(text.clone().into_bytes()), &mut log).expect("a cursor reads");
            assert_eq!(log, whole, "{what}, read to line {cut} and kept, then on");
        }
    }
}

#[test]
fn a_log_shorter_than_what_was_read_is_read_anew() {
    let mut log = read(&(typed(AT, AT, "one") + &typed(AT + 1000, AT + 1000, "two")));
    let shorter = typed(AT + 5000, AT + 5000, "three");
    follow(&mut Cursor::new(shorter.into_bytes()), &mut log).expect("a cursor reads");
    assert_eq!(log.turns.len(), 1, "what was read before is gone");
    assert_eq!(log.turns[0].enter, AT + 5000);
}

#[test]
fn a_message_said_again_soon_is_a_second_go() {
    let log = read(
        &(typed(AT, AT, "why is the launcher slow to open")
            + &typed(
                AT + 120_000,
                AT + 120_000,
                "why is the launcher so slow to open?",
            )
            + &typed(
                AT + 900_000,
                AT + 900_000,
                "why is the launcher so slow to open?",
            )
            + &typed(AT + 960_000, AT + 960_000, "now look at the backlight")),
    );
    let again: Vec<bool> = log.turns.iter().map(|t| t.again).collect();
    assert_eq!(again, [false, true, false, false]);
}

#[test]
fn what_he_typed_and_what_agents_said_is_never_kept() {
    let text = typed(AT, AT + 20_000, "the token is sk-FAKE-7QEY1S-0001 ok")
        + &reply(AT + 21_000, 1000, "stop")
        + &tool_result(AT + 22_000, "sk-FAKE-7QEY1S-0002")
        + &typed(
            AT + 30_000,
            AT + 30_000,
            "the token is sk-FAKE-7QEY1S-0001 ok!",
        );
    let log = read(&text);
    assert_eq!(log.turns.len(), 2, "both messages were read");
    let kept = serde_json::to_string(&log).expect("the state serializes")
        + &serde_json::to_string(&painful(&[&log.turns], AT - 1, AT + 1))
            .expect("the day serializes");
    assert!(
        !kept.contains("FAKE"),
        "nothing typed or answered is kept: {kept}"
    );
}

#[test]
fn iso_times_read_as_pi_writes_them() {
    assert_eq!(iso_ms("2026-11-10T19:16:00.282Z"), Some(1_794_338_160_282));
    assert_eq!(iso_ms("2026-11-10T19:16:00Z"), Some(1_794_338_160_000));
    assert_eq!(iso_ms("2026-11-10T19:16:00.2Z"), Some(1_794_338_160_200));
    assert_eq!(iso_ms("not a time"), None);
}

#[test]
fn pruning_keeps_the_last_turn_to_compare_with() {
    let mut log = Log {
        end: 0,
        open: true,
        turns: vec![turn(AT - 10, &[]), turn(AT - 5, &[])],
        next: None,
        compacting: None,
        errored: Errored::No,
        error: None,
    };
    log.prune(AT);
    assert_eq!(log.turns.len(), 1, "the older is gone");
    assert_eq!(log.turns[0].enter, AT - 5);
}
