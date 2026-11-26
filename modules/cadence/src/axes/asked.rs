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
//! asks gone, so cadence-spec keeps the last rows; one the hour never saw
//! and not there holds no asks yet; and the hour's record unreadable leaves
//! the list unread, as the hour reads it then too (round 2, F9 and F20).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use jiff::Timestamp;
use serde_json::{Value, json};
use terminal_kit::Failure;

use super::probes::{self, Probed};
use super::spec::{launcher_words, quests, titled};
use super::words::Words;
use crate::hour::{Kept, first_owner, ledger};
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
    let mut hour = Kept::last(&folder.join("hour.json"));
    let (asks, unread) = ledger::whole(&state_home.join("mission/asks.jsonl"), &mut hour.asks, now);
    if let Some(why) = hour.unread.or(unread) {
        return Err(failed(format!("the asks list is unread, as {why}")));
    }
    let open = open(&asks);
    let owners: BTreeSet<&str> = open.iter().map(|ask| ask.answers.as_str()).collect();
    let titles = titled(&quests(&home), &owners);
    let runs = probes::gather(&state_home);
    let unrun = probes::unread(&runs);
    if unrun > 0 {
        eprintln!(
            "cadence: {unrun} lines of the probes record aren't runs, so their asks read as \
             not run"
        );
    }
    let probed = probes::read(&runs);
    let made = made(&open, &probed, &titles, &words, now);
    println!("{}", json!({"rows": made.0, "head": made.1}));
    Ok(())
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
                moved: ask["promised"]
                    .as_array()
                    .map_or(0, |given| given.len().saturating_sub(1)),
                change: ask["pr"].as_str().unwrap_or("").to_owned(),
                probed: ask["probe"].is_object(),
            })
        })
        .collect()
}

/// Words as one line, each run of spaces or line breaks one space.
fn one_line(words: &str) -> String {
    words.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The rows in the order shown and the place's headline, at `now`.
fn made(
    open: &[Ask],
    probed: &BTreeMap<String, Probed>,
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
        .map(|ask| row(ask, probed.get(&ask.id), titles, words, now))
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
    probe: Option<&Probed>,
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
    let failing = probe.is_some_and(|probe| !probe.passed);
    // The side says "probe" in each phrase, the drawer's Probe by its name.
    let probe_in = |id: &str| match probe {
        Some(probe) if probe.passed => json!({"said": words.said(id, "passes", &[])}),
        Some(probe) => timed(
            &words.said(id, "fails", &[]),
            probe.since.unwrap_or(probe.at),
            "moment",
        ),
        None if ask.probed => json!({"said": words.said(id, "unrun", &[])}),
        None => json!({"said": words.said(id, "unprobed", &[])}),
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
    if let Some(probe) = probe {
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
            &BTreeMap::new(),
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
                r#", "probe": {"check": "z"}"#,
            ),
            line("n4", "2026-11-26T09:00:00-05:00", "unprobed", ""),
        ]
        .join("\n");
        let runs = [
            r#"{"ask": "p1", "at": "2026-11-26T11:00:00-05:00", "passed": true}"#,
            r#"{"ask": "f2", "at": "2026-11-26T10:00:00-05:00", "passed": false}"#,
            r#"{"ask": "f2", "at": "2026-11-26T11:00:00-05:00", "passed": false}"#,
        ]
        .join("\n");
        let (rows, _) = made(
            &open(&text),
            &probes::read(&runs),
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
        assert_eq!(passing["side"][1]["said"], "probe passes");
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
        assert_eq!(by("n4")["side"][1]["said"], "no probe yet");
    }

    #[test]
    fn an_owner_is_named_by_its_quest_and_return_copies_its_change() {
        let text = line(
            "c1",
            "2026-11-26T09:00:00-05:00",
            // A line break as JSON writes one, which a row draws as a space.
            "the\\nlauncher   view",
            r#", "pr": "core.nix#6846", "promised": [{}, {}, {}]"#,
        );
        let titles = BTreeMap::from([("Y2YBTS".to_owned(), "Launcher".to_owned())]);
        let (rows, _) = made(&open(&text), &BTreeMap::new(), &titles, &words(), NOON);
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
            "three dates given is two moves: {facts:?}"
        );
    }

    #[test]
    fn nothing_open_says_so() {
        let (rows, head) = made(&[], &BTreeMap::new(), &BTreeMap::new(), &words(), NOON);
        assert_eq!(rows, Vec::<Value>::new());
        assert_eq!(head["said"], "nothing you asked for is open");
        assert!(head.get("glyph").is_none());
    }
}
