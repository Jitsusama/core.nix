//! `cadence axes --asked WORDS`: what Joel has asked for and isn't done,
//! as the launcher's Asked For rows, printed as one line of JSON,
//! `{"rows": [...], "head": {...}}`, for cadence-spec (../../spec) to
//! serve beside Spec. Joel asked for one view of his asks (WCJQSG,
//! 2026-11-26, d903dead, loop 6): who owns each, when it's due, and whether
//! its probe passes.
//!
//! The asks are the orchestration's record, `mission/asks.jsonl`, each
//! line one ask with its `id` and the cell that `answers` for it (J7XHQN,
//! 775a3aa8), read as his where cadence counts them his (tracking.rs) and
//! open until it `landed`. An ask is written again whole as its date
//! moves or it lands, so each is read from its latest line, as the hour's
//! measures read it. An owner kept by hand before `answers` reads as the
//! hour reads one (hour/measures/owner.rs). Its `live` is a batch's stamp, not when it went
//! live (A210E4CA, 20e58736), so it says nothing here. Each probe stands
//! by its newest run (probes.rs). An ask late comes first, the longest late
//! leading, then those due soonest, then those given no date, oldest
//! asked first. Times are served as `{at, as}` for the shell to word, so a
//! row read at 13:59 still says how it stands at 14:01.
//!
//! The list is read whole against what the hour saw of it (hour/ledger.rs,
//! hour/card.rs), never a second record of its own on another clock: one
//! written over, holding fewer asks than the hour read in it, is unread
//! until the hour takes it as pruned, rather than served as most of his
//! asks gone. Unread, it prints a headline saying so and asks cadence-spec
//! to keep the last rows (`keep`), so Asked For says it can't read the
//! list rather than waiting on it for good (round 4, F57); one the hour never saw
//! and not there holds no asks yet; and the hour's record unreadable leaves
//! the list unread, as the hour reads it then too (round 2, F9 and F20).
//! A list where the hour has kept no record at all, as on a machine whose
//! hour sends nothing and so keeps nothing, is unread too, since nothing
//! could tell it written over (round 3, F30). A date moved is counted from
//! the dates the hour reads in an ask (hour/measures/promised.rs), and a
//! probes record with a line lost reads every probe unread (probes.rs).
//! A probe stands only for an ask given one, whatever runs name it, and a
//! pass says when it ran, so one from weeks ago never reads as this
//! morning's (round 4, F70).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use jiff::Timestamp;
use serde_json::{Value, json};
use terminal_kit::Failure;

use super::probes::{self, Probed};
use super::spec::{launcher_words, quests, titled};
use super::words::Words;
use crate::hour::{Kept, first_owner, ledger, promised};
use crate::tracking::{self, Tracking};
use crate::{failed, folders, now_ms, wrongly};

/// The place its rows are served in.
pub const PLACE: &str = "asked";

/// `cadence axes --asked WORDS`, given what follows `--asked`.
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), Failure> {
    let (Some(vocabulary), None) = (args.next().map(PathBuf::from), args.next()) else {
        return Err(wrongly("axes", "--asked takes the launcher's words alone"));
    };
    let words = launcher_words(&vocabulary)?;
    let (home, state_home, folder) = folders().map_err(failed)?;
    let now = now_ms() / 1000;
    // Read, never written: the hour alone keeps what it saw.
    let kept = folder.join("hour.json");
    let list = state_home.join("mission/asks.jsonl");
    if !kept.exists() && list.exists() {
        unread(&words, "the hour has kept no record to read it against");
        return Ok(());
    }
    let mut hour = Kept::last(&kept);
    let (asks, why) = ledger::whole(&list, &mut hour.asks, now);
    if let Some(why) = hour.unread.or(why) {
        unread(&words, &why);
        return Ok(());
    }
    let open = open(&asks);
    let owners: BTreeSet<&str> = open.iter().map(|ask| ask.answers.as_str()).collect();
    let titles = titled(&quests(&home), &owners);
    let probed = probes::gather(&state_home)
        .inspect_err(|why| {
            eprintln!("cadence: the probes record is unread, as {why}, so every probe reads so");
        })
        .ok()
        .map(|runs| probes::read(&runs));
    let made = made(&open, probed.as_ref(), &titles, &words, now);
    println!("{}", json!({"rows": made.0, "head": made.1}));
    Ok(())
}

/// Says the list is unread, as `why`, in the journal and to cadence-spec:
/// a headline saying so and no rows, the last rows kept.
fn unread(words: &Words, why: &str) {
    eprintln!("cadence: the asks list is unread, as {why}, so Asked For keeps its last rows");
    println!("{}", unread_made(words));
}

/// What's printed for a list unread: its headline, and `keep` in place of
/// rows, so the place keeps the last it served.
fn unread_made(words: &Words) -> Value {
    json!({
        "keep": true,
        "head": {
            "key": format!("head:{PLACE}"),
            "place": PLACE,
            "said": words.said("asked:head", "unread", &[]),
            "glyph": "failed",
        },
    })
}

/// One open ask of his, its times in seconds.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ask {
    id: String,
    at: i64,
    words: String,
    answers: String,
    expected: Option<i64>,
    moved: usize,
    change: String,
    probed: bool,
}

/// His asks in `text` not yet landed, each with the id it's known by and
/// read from its latest line; one with no id or time is none, since
/// nothing could find it again.
fn open(text: &str) -> Vec<Ask> {
    let time = |v: &Value| {
        v.as_str()
            .and_then(|t| t.parse::<Timestamp>().ok())
            .map(Timestamp::as_second)
    };
    let mut latest: BTreeMap<String, Value> = BTreeMap::new();
    for ask in text
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
    {
        if let Some(id) = ask["id"].as_str().filter(|id| !id.is_empty()) {
            latest.insert(id.to_owned(), ask);
        }
    }
    latest
        .into_values()
        .filter(|ask| tracking::of(ask) == Some(Tracking::His) && ask["landed"].is_null())
        .filter_map(|ask| {
            let answers = ask["answers"].as_str().map_or_else(
                || first_owner(ask["owner"].as_str().unwrap_or("")),
                str::to_owned,
            );
            Some(Ask {
                id: ask["id"].as_str().filter(|id| !id.is_empty())?.to_owned(),
                at: time(&ask["at"])?,
                words: one_line(ask["words"].as_str().unwrap_or("")),
                answers,
                expected: time(&ask["expected"]),
                moved: promised(&ask).len().saturating_sub(1),
                change: ask["pr"].as_str().unwrap_or("").to_owned(),
                probed: given(&ask["probe"]),
            })
        })
        .collect()
}

/// Whether an ask was given a probe: one check as an object, several as a
/// list, as the ledger holds both (148 and 18 on 27 Nov), or one named by
/// a string; never one null, empty or false (round 5, F96).
fn given(probe: &Value) -> bool {
    match probe {
        Value::Object(check) => !check.is_empty(),
        Value::Array(checks) => !checks.is_empty(),
        Value::String(named) => !named.is_empty(),
        _ => false,
    }
}

/// Words as one line, each run of spaces or line breaks one space.
fn one_line(words: &str) -> String {
    words.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// How an ask's probe reads: none given it, by its newest run, not run
/// yet, or the record unread, so a lost newest run never reads as the one
/// before it.
#[derive(Clone, Copy)]
enum Probe<'a> {
    Unprobed,
    Ran(&'a Probed),
    NotRun,
    Unread,
}

/// The rows in the order shown and the place's headline, at `now`, each
/// probe by `probed`, or none read where the probes record is unread.
fn made(
    open: &[Ask],
    probed: Option<&BTreeMap<String, Probed>>,
    titles: &BTreeMap<String, String>,
    words: &Words,
    now: i64,
) -> (Vec<Value>, Value) {
    let mut ordered: Vec<&Ask> = open.iter().collect();
    ordered.sort_by_key(|ask| match ask.expected {
        Some(due) => (u8::from(due >= now), due, ask.at),
        None => (2, 0, ask.at),
    });
    let late = open
        .iter()
        .filter(|ask| ask.expected.is_some_and(|due| due < now))
        .count();
    let rows = ordered
        .into_iter()
        .map(|ask| {
            // A run naming an ask given no probe stands for nothing.
            let probe = if ask.probed {
                probed.map_or(Probe::Unread, |probed| {
                    probed.get(&ask.id).map_or(Probe::NotRun, Probe::Ran)
                })
            } else {
                Probe::Unprobed
            };
            row(ask, probe, titles, words, now)
        })
        .collect();
    let (count, late_count) = (open.len().to_string(), late.to_string());
    let said = if open.is_empty() {
        words.said("asked:head", "none", &[])
    } else if late == 0 {
        words.said("asked:head", "open", &[("count", &count)])
    } else {
        words.said(
            "asked:head",
            "late",
            &[("late", &late_count), ("count", &count)],
        )
    };
    let mut head = json!({"key": format!("head:{PLACE}"), "place": PLACE, "said": said});
    if late > 0 {
        head["glyph"] = json!("failed");
    }
    (rows, head)
}

/// A time as the shell words it, in milliseconds, put in `said`'s {time}.
fn timed(said: &str, at: i64, form: &str) -> Value {
    json!({"said": said, "at": at * 1000, "as": form})
}

fn row(
    ask: &Ask,
    probe: Probe<'_>,
    titles: &BTreeMap<String, String>,
    words: &Words,
    now: i64,
) -> Value {
    let said = |phrase: &str| words.said("asked:row", phrase, &[]);
    let late = ask.expected.is_some_and(|due| due < now);
    let due = match ask.expected {
        Some(due) if late => timed(&said("late"), due, "moment"),
        Some(due) => timed(&said("due"), due, "meeting"),
        None => json!({"said": said("undated")}),
    };
    // The drawer's Due gives the day in full, its name saying the rest.
    let due_fact = ask.expected.map_or_else(
        || json!({"said": words.said("asked:due", "undated", &[])}),
        |due| timed("", due, "dated"),
    );
    let failing = matches!(probe, Probe::Ran(ran) if !ran.passed);
    // The side says "probe" in each phrase, the drawer's Probe by its name.
    let probe_in = |id: &str| match probe {
        Probe::Ran(ran) if ran.passed => timed(&words.said(id, "passes", &[]), ran.at, "moment"),
        Probe::Ran(ran) => timed(
            &words.said(id, "fails", &[]),
            ran.since.unwrap_or(ran.at),
            "moment",
        ),
        Probe::Unprobed => json!({"said": words.said(id, "unprobed", &[])}),
        Probe::Unread => json!({"said": words.said(id, "unread", &[])}),
        Probe::NotRun => json!({"said": words.said(id, "unrun", &[])}),
    };
    let probe_said = probe_in("asked:row");
    let owner = titles
        .get(&ask.answers)
        .cloned()
        .unwrap_or_else(|| words.said("spec:reading:owner", "untitled", &[]));
    let fact = |id: &str, said: Value| {
        let mut fact = said;
        fact["name"] = json!(words.name(id));
        fact
    };
    let mut facts = vec![
        fact("asked:owner", json!({"said": owner})),
        fact("asked:at", timed("", ask.at, "dated")),
        fact("asked:due", due_fact),
        fact("asked:probe", probe_in("asked:probe")),
    ];
    if ask.moved > 0 {
        let moved = ask.moved.to_string();
        let times = if ask.moved == 1 {
            words.said("asked:moved", "once", &[])
        } else {
            words.said("asked:moved", "times", &[("count", &moved)])
        };
        facts.push(fact("asked:moved", json!({"said": times})));
    }
    if !ask.change.is_empty() {
        facts.push(fact("asked:change", json!({"said": ask.change})));
    }
    let mut nerd = vec![
        json!({"label": "id", "value": ask.id}),
        json!({"label": "answers", "value": ask.answers}),
    ];
    if let Probe::Ran(probe) = probe {
        nerd.push(json!({"label": "probed by", "value": probe.by}));
        if let Some(switch) = probe.switch {
            nerd.push(json!({"label": "switch", "value": switch.to_string()}));
        }
    }
    let mut row = json!({
        "key": format!("row:{PLACE}:{}", ask.id),
        "thing": format!("asked:{}", ask.id),
        "place": PLACE,
        "kind": "asked",
        "depth": 0,
        "name": ask.words,
        "note": owner,
        "owner": ask.answers,
        "side": [due, probe_said],
        // Return copies its change, or else the ask's own words, to say to
        // its owner, until the shell can go to the owner's session.
        "acts": [{"key": "verb:copy", "copy": if ask.change.is_empty() {
            ask.words.clone()
        } else {
            ask.change.clone()
        }}],
        "drawer": {
            "eyebrow": words.said("asked:drawer", "eyebrow", &[]),
            "title": ask.words,
            "facts": facts,
            "nerd": nerd,
        },
    });
    if late || failing {
        row["glyph"] = json!("failed");
    }
    row
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Words {
        let text = include_str!("../../tests/words.json");
        Words::new(&serde_json::from_str(text).expect("the test's words are JSON"))
    }

    /// 12:00 on 26 November 2026, Toronto time, in seconds.
    const NOON: i64 = 1_795_712_400;

    fn line(id: &str, at: &str, words: &str, more: &str) -> String {
        format!(r#"{{"id": "{id}", "at": "{at}", "words": "{words}", "answers": "Y2YBTS"{more}}}"#)
    }

    #[test]
    fn only_his_open_asks_are_listed() {
        let text = [
            line("a1", "2026-11-26T09:00:00-05:00", "mine, open", ""),
            line(
                "b2",
                "2026-11-26T09:00:00-05:00",
                "mine, landed",
                r#", "landed": "2026-11-26T10:00:00-05:00""#,
            ),
            line(
                "c3",
                "2026-11-26T09:00:00-05:00",
                "derived",
                r#", "tracking": "derived""#,
            ),
            line(
                "d4",
                "2026-11-26T09:00:00-05:00",
                "put down",
                r#", "closed": "put down""#,
            ),
            r#"{"at": "2026-11-26T09:00:00-05:00", "words": "no id"}"#.to_owned(),
        ]
        .join("\n");
        let ids: Vec<String> = open(&text).into_iter().map(|ask| ask.id).collect();
        assert_eq!(ids, ["a1"]);
    }

    #[test]
    fn an_ask_written_again_is_read_from_its_latest_line() {
        let at = "2026-11-26T09:00:00-05:00";
        let text = [
            line(
                "m1",
                at,
                "moved",
                r#", "expected": "2026-11-26T10:00:00-05:00""#,
            ),
            line("l2", at, "landed", ""),
            line(
                "m1",
                at,
                "moved",
                r#", "expected": "2026-11-26T15:00:00-05:00""#,
            ),
            line(
                "l2",
                at,
                "landed",
                r#", "landed": "2026-11-26T11:00:00-05:00""#,
            ),
        ]
        .join("\n");
        let open = open(&text);
        let read: Vec<(&str, Option<i64>)> = open
            .iter()
            .map(|ask| (ask.id.as_str(), ask.expected))
            .collect();
        assert_eq!(
            read,
            [("m1", Some(NOON + 3 * 3600))],
            "one row for an ask whose date moved, at its new date, and none for one landed \
             on a later line"
        );
    }

    #[test]
    fn an_owner_kept_by_hand_reads_as_the_hour_reads_it() {
        let by_hand = |id: &str, owner: &str| {
            serde_json::json!({"id": id, "at": "2026-11-26T09:00:00-05:00", "words": id,
                "owner": owner})
            .to_string()
        };
        let text = [
            by_hand("o1", "PN2VZC (H6EDLC)"),
            by_hand("o2", "QEST-20261111-Y2YBTS"),
        ]
        .join("\n");
        let owners: Vec<String> = open(&text).into_iter().map(|ask| ask.answers).collect();
        assert_eq!(
            owners,
            ["PN2VZC", "Y2YBTS"],
            "the first owner, by the address it answers to"
        );
    }

    #[test]
    fn late_asks_lead_then_the_soonest_due_then_the_undated() {
        let due = |hour: u8| format!(r#", "expected": "2026-11-26T{hour}:00:00-05:00""#);
        let text = [
            line("undated", "2026-11-25T09:00:00-05:00", "no date", ""),
            line("later", "2026-11-26T09:00:00-05:00", "due at 15", &due(15)),
            line(
                "late",
                "2026-11-26T09:00:00-05:00",
                "was due at 10",
                &due(10),
            ),
            line("soon", "2026-11-26T09:00:00-05:00", "due at 13", &due(13)),
            line(
                "latest",
                "2026-11-26T09:00:00-05:00",
                "was due at 11",
                &due(11),
            ),
        ]
        .join("\n");
        let (rows, head) = made(
            &open(&text),
            Some(&BTreeMap::new()),
            &BTreeMap::new(),
            &words(),
            NOON,
        );
        let keys: Vec<&str> = rows.iter().filter_map(|row| row["key"].as_str()).collect();
        assert_eq!(
            keys,
            [
                "row:asked:late",
                "row:asked:latest",
                "row:asked:soon",
                "row:asked:later",
                "row:asked:undated"
            ]
        );
        assert_eq!(head["said"], "2 late of 5");
        assert_eq!(head["glyph"], "failed");
        assert_eq!(
            rows[0]["glyph"], "failed",
            "a late ask wears the failed mark"
        );
        assert!(rows[2].get("glyph").is_none(), "one due later wears none");
    }

    #[test]
    fn a_row_says_when_its_due_and_how_its_probe_stands() {
        let text = [
            line(
                "p1",
                "2026-11-26T09:00:00-05:00",
                "passing",
                r#", "expected": "2026-11-26T15:00:00-05:00", "probe": {"check": "x"}"#,
            ),
            line(
                "f2",
                "2026-11-26T09:00:00-05:00",
                "failing",
                r#", "probe": {"check": "y"}"#,
            ),
            line(
                "u3",
                "2026-11-26T09:00:00-05:00",
                "unrun",
                // Several checks, as a list, as the ledger holds some.
                r#", "probe": [{"check": "z"}, {"check": "zz"}]"#,
            ),
            line("n4", "2026-11-26T09:00:00-05:00", "unprobed", ""),
        ]
        .join("\n");
        let runs = [
            r#"{"ask": "p1", "at": "2026-11-26T11:00:00-05:00", "passed": true}"#,
            r#"{"ask": "f2", "at": "2026-11-26T10:00:00-05:00", "passed": false}"#,
            r#"{"ask": "f2", "at": "2026-11-26T11:00:00-05:00", "passed": false}"#,
            // A run naming an ask given no probe, as any writer could add.
            r#"{"ask": "n4", "at": "2026-11-26T11:00:00-05:00", "passed": false}"#,
        ]
        .join("\n");
        let (rows, _) = made(
            &open(&text),
            Some(&probes::read(&runs)),
            &BTreeMap::new(),
            &words(),
            NOON,
        );
        let by = |key: &str| {
            rows.iter()
                .find(|row| row["key"] == format!("row:asked:{key}"))
                .expect("its row")
                .clone()
        };
        let passing = by("p1");
        assert_eq!(passing["side"][0]["said"], "due {time}");
        assert_eq!(passing["side"][0]["as"], "meeting");
        assert_eq!(passing["side"][1]["said"], "probe passed {time}");
        let eleven: Timestamp = "2026-11-26T11:00:00-05:00".parse().expect("a time");
        assert_eq!(
            passing["side"][1]["at"],
            eleven.as_millisecond(),
            "a pass says when it ran, so an old one reads old"
        );
        let failing = by("f2");
        assert_eq!(failing["side"][0]["said"], "no date yet");
        assert_eq!(failing["side"][1]["said"], "probe fails since {time}");
        let ten: Timestamp = "2026-11-26T10:00:00-05:00".parse().expect("a time");
        assert_eq!(
            failing["side"][1]["at"],
            ten.as_millisecond(),
            "since its first fail"
        );
        assert_eq!(
            failing["glyph"], "failed",
            "a failing probe wears the failed mark"
        );
        assert_eq!(by("u3")["side"][1]["said"], "probe not run yet");
        let unprobed = by("n4");
        assert_eq!(
            unprobed["side"][1]["said"], "no probe yet",
            "a run stands only for an ask given a probe"
        );
        assert!(
            unprobed.get("glyph").is_none(),
            "nor marks one failed: {unprobed}"
        );
        // The record unread, a probe that passed reads unread, never as the
        // run before a lost one; one with no probe still says so.
        let (rows, _) = made(&open(&text), None, &BTreeMap::new(), &words(), NOON);
        let side = |key: &str| {
            rows.iter()
                .find(|row| row["key"] == format!("row:asked:{key}"))
                .expect("its row")["side"][1]["said"]
                .clone()
        };
        assert_eq!(side("p1"), "probe can't be read");
        assert_eq!(side("n4"), "no probe yet");
    }

    #[test]
    fn an_owner_is_named_by_its_quest_and_return_copies_its_change() {
        let text = line(
            "c1",
            "2026-11-26T09:00:00-05:00",
            // A line break as JSON writes one, which a row draws as a space.
            "the\\nlauncher   view",
            concat!(
                r#", "pr": "core.nix#6846", "expected": "2026-11-27T09:00:00-05:00","#,
                r#" "promised": [{"expected": "2026-11-26T15:00:00-05:00"}, "a", {},"#,
                r#" {"expected": "2026-11-26T18:00:00-05:00"}]"#,
            ),
        );
        let titles = BTreeMap::from([("Y2YBTS".to_owned(), "Launcher".to_owned())]);
        let (rows, _) = made(
            &open(&text),
            Some(&BTreeMap::new()),
            &titles,
            &words(),
            NOON,
        );
        assert_eq!(
            rows[0]["name"], "the launcher view",
            "one line, as a row draws it"
        );
        assert_eq!(rows[0]["note"], "Launcher");
        assert_eq!(rows[0]["acts"][0]["copy"], "core.nix#6846");
        let facts = rows[0]["drawer"]["facts"].as_array().expect("facts");
        assert!(
            facts
                .iter()
                .any(|fact| fact["name"] == "Date Moved" && fact["said"] == "2 times"),
            "three dates, as the hour reads them, is two moves: {facts:?}"
        );
    }

    #[test]
    fn a_list_unread_says_so_and_keeps_the_last_rows() {
        let made = unread_made(&words());
        assert_eq!(made["keep"], true, "the place keeps what it last served");
        assert!(made.get("rows").is_none(), "and is handed no rows to take");
        assert_eq!(made["head"]["key"], "head:asked");
        assert_eq!(made["head"]["said"], "can't read the list now");
        assert_eq!(made["head"]["glyph"], "failed");
    }

    #[test]
    fn nothing_open_says_so() {
        let (rows, head) = made(
            &[],
            Some(&BTreeMap::new()),
            &BTreeMap::new(),
            &words(),
            NOON,
        );
        assert_eq!(rows, Vec::<Value>::new());
        assert_eq!(head["said"], "nothing you asked for is open");
        assert!(head.get("glyph").is_none());
    }
}
