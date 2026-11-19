//! Whether a call is on on another of the person's machines, as each
//! machine's mycelium node says: a call carried from one machine to the
//! laptop beside it is the same person's call, and whatever mustn't cut
//! one here mustn't cut it there either. A switch went ahead mid-call on
//! 18 November, the call moved to a Mac, because a-call-is-on heard only
//! this machine's sound server.
//!
//! Every node runs a-call-is-on, asked about its own machine alone
//! (`--here`), as a probe, and says what it answered as `reached.call` in
//! its machine's record, beside the probes it declares; every node of a
//! realm hears its peers' records (modules/mycelium). [`judge`] reads
//! those records:
//!
//! - a fresh record whose `reached.call` is a literal `true` is a call;
//! - a fresh one that declares the call probe but holds no yes or no, as
//!   when the probe couldn't tell, is untold;
//! - a fresh one that declares no call probe, as a machine on a core
//!   from before the probe or one declaring nothing, is unseen;
//! - a record older than [`FRESH`], a machine asleep, off or away, is no
//!   call, so a closed laptop holds nothing back;
//! - a record dated a minute or more ahead of this machine's clock, or
//!   with no time it can read, is undated, since how old it is can't be
//!   told.
//!
//! A machine's name keeps only a host name's printable ASCII, so no record
//! steers or hides itself in the terminal or the day it's named in, and at
//! most [`MACHINES_AT_MOST`] machines are kept, so no answer names as many
//! as it likes; a call past them still counts, and a machine past them on
//! no call is passed over unremarked.
//!
//! Untold, unseen and undated machines, and a node here that won't answer, are
//! named rather than waited on: whatever asks counts can't tell as busy
//! with no end, so one broken peer, or a wedged node, would hold every
//! switch for ever, the one that would mend it included. Only this
//! machine's own sound server not answering is can't tell.
//!
//! This machine's own record is passed over, since its sound server
//! already says; the probe asking only here is what keeps two machines
//! from each holding the other's call up for ever.

use serde_json::Value;
use std::collections::BTreeMap;

/// How old a record may be, in seconds, and still be read: three of the
/// node's minutes, so one late measure doesn't drop a call, while a
/// machine gone quiet stops counting within a few minutes. jobs-sorter's
/// brake reads records the same way (modules/slices/jobs-sorter).
pub const FRESH: i64 = 180;

/// What the records of the other machines say, each machine named once
/// under the strongest thing any of its records says.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Heard {
    /// Machines whose fresh record says a call is on.
    pub calls: Vec<String>,
    /// Machines whose fresh record declares the call probe but holds no
    /// yes or no.
    pub untold: Vec<String>,
    /// Machines whose fresh record declares no call probe.
    pub unseen: Vec<String>,
    /// Machines whose record is dated ahead of this machine's clock, or
    /// carries no time it can read.
    pub undated: Vec<String>,
    /// Why each node here that didn't answer couldn't, naming its socket.
    pub unanswered: Vec<String>,
    /// How many nodes here answered.
    pub answered: usize,
    /// How many machines past the first [`MACHINES_AT_MOST`] were passed
    /// over unnamed, each by its strongest record: none of them a call or
    /// on none.
    pub unnamed: usize,
}

/// What this machine's nodes under `runtimes` hear of its peers now, on
/// Linux (src/nodes.rs), each node asked whatever another said.
#[cfg(target_os = "linux")]
#[must_use]
pub fn hear(runtimes: &std::path::Path) -> Heard {
    let asked = crate::nodes::answers(runtimes);
    // mycelium names a machine by its hostname.
    let here = std::fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|name| name.trim().to_owned())
        .unwrap_or_default();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(0));
    Heard {
        unanswered: asked.unanswered,
        answered: asked.answers.len(),
        ..judge(&asked.answers, &here, now)
    }
}

/// What one machine's records say, weakest first, so the strongest of
/// its realms' records wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Said {
    Undated,
    Unseen,
    Untold,
    NoCall,
    Call,
}

/// The most characters of a machine's name kept, a host name's most: a
/// longer one is cut there, so no record can make a fault or a line as
/// long as it likes.
const NAMED_IN: usize = 64;

/// The most machines the records may name: a person's realms hold a
/// handful, so more says something lists what isn't theirs.
pub const MACHINES_AT_MOST: usize = 64;

/// What the nodes' answers to a `Machines` request say of every machine
/// but `here`, `now` in seconds since the epoch. An answer that isn't a
/// node's list of machines says nothing; the caller counts a node that
/// couldn't answer.
pub fn judge(answers: &[Value], here: &str, now: i64) -> Heard {
    let mut machines: BTreeMap<String, Said> = BTreeMap::new();
    // Each machine past the most named, by its strongest record, so one
    // seen in several realms counts once, and not at all when any says it's
    // on no call.
    let mut past: BTreeMap<String, Said> = BTreeMap::new();
    let records = answers
        .iter()
        .filter_map(|answer| answer["Ok"]["Machines"].as_array())
        .flatten();
    for record in records {
        let Some(name) = record["name"]
            .as_str()
            .map(|name| {
                // A host name's letters: no control, bidi or zero-width
                // character from a peer reaches a line or the day.
                name.chars()
                    .filter(char::is_ascii_graphic)
                    .take(NAMED_IN)
                    .collect::<String>()
            })
            .filter(|name| !name.is_empty() && name != here)
        else {
            continue;
        };
        let Some(said) = said(record, now) else {
            continue;
        };
        if said != Said::Call && machines.len() >= MACHINES_AT_MOST && !machines.contains_key(&name)
        {
            let strongest = past.entry(name).or_insert(said);
            *strongest = (*strongest).max(said);
            continue;
        }
        let strongest = machines.entry(name).or_insert(said);
        *strongest = (*strongest).max(said);
    }
    // A call past the most is named all the same, and a machine that said
    // it's on no call goes unnamed anyway.
    let unnamed = past
        .iter()
        .filter(|(name, said)| **said != Said::NoCall && !machines.contains_key(*name))
        .count();
    let mut heard = Heard {
        unnamed,
        ..Heard::default()
    };
    for (name, said) in machines {
        let list = match said {
            Said::Call => &mut heard.calls,
            Said::Untold => &mut heard.untold,
            Said::Unseen => &mut heard.unseen,
            Said::Undated => &mut heard.undated,
            Said::NoCall => continue,
        };
        list.push(name);
    }
    heard
}

/// What one machine's record says, or nothing when it's gone stale.
fn said(record: &Value, now: i64) -> Option<Said> {
    let Some(measured) = record["measured_at"].as_str().and_then(seconds_of) else {
        return Some(Said::Undated);
    };
    if measured > now + 60 {
        return Some(Said::Undated);
    }
    if measured < now - FRESH {
        return None;
    }
    let reached = &record["reached"]["call"];
    if *reached == Value::Bool(true) {
        return Some(Said::Call);
    }
    if record["declared"]["probes"].get("call").is_none() {
        return Some(Said::Unseen);
    }
    Some(if *reached == Value::Bool(false) {
        Said::NoCall
    } else {
        Said::Untold
    })
}

/// The seconds since the epoch of an RFC 3339 time in UTC, as mycelium
/// writes it, `2026-11-17T14:16:00.36Z`; any other shape is none.
fn seconds_of(stamp: &str) -> Option<i64> {
    let stamp = stamp.strip_suffix('Z')?;
    let (date, time) = stamp.split_once('T')?;
    let time = time.split_once('.').map_or(time, |(whole, _)| whole);
    let figures = |text: &str| -> Option<Vec<i64>> {
        text.split(['-', ':'])
            .map(|figure| figure.parse().ok())
            .collect()
    };
    let (date, time) = (figures(date)?, figures(time)?);
    let ([year, month, day], [hour, minute, second]) = (date.as_slice(), time.as_slice()) else {
        return None;
    };
    let in_range = (1970..=9999).contains(year)
        && (1..=12).contains(month)
        && (1..=31).contains(day)
        && (0..24).contains(hour)
        && (0..60).contains(minute)
        && (0..=60).contains(second);
    in_range.then(|| {
        days_from_civil(*year, *month, *day) * 86_400 + hour * 3_600 + minute * 60 + second
    })
}

/// Days since 1970-01-01 of a date in the proleptic Gregorian calendar
/// (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * 146_097 + of_cycle - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// This machine's name, as the tests give it.
    const HERE: &str = "cistern";

    /// 2026-11-17T14:16:00Z, in seconds since the epoch.
    const AT: i64 = 1_794_924_960;

    /// A record measured at `measured_at`, declaring the call probe when
    /// `declares_call`, its probes saying `reached`.
    fn record(name: &str, measured_at: &str, declares_call: bool, reached: &Value) -> Value {
        let probes = if declares_call {
            json!({"call": {"run": ["a-call-is-on", "--here"]}})
        } else {
            json!({"home": {"run": ["probe"]}})
        };
        json!({"name": name, "measured_at": measured_at,
               "declared": {"probes": probes}, "reached": reached})
    }

    /// A node's answer holding `records`.
    fn answer(records: &[Value]) -> Value {
        json!({"Ok": {"Machines": records}})
    }

    const NOW: &str = "2026-11-17T14:16:00.365918927Z";

    #[test]
    fn mycelium_times_are_read_as_seconds_since_the_epoch() {
        assert_eq!(seconds_of("1970-01-01T00:00:00Z"), Some(0), "the epoch");
        assert_eq!(
            seconds_of(NOW),
            Some(AT),
            "a node's measure, with its fraction"
        );
        assert_eq!(
            seconds_of("2024-02-29T23:59:59Z"),
            Some(1_709_251_199),
            "a leap day"
        );
        for unread in [
            "2026-11-17T14:16:00",
            "2026-11-17T14:16:00+01:00",
            "2026-13-17T14:16:00Z",
            "2026-11-17T24:16:00Z",
            "",
        ] {
            assert_eq!(seconds_of(unread), None, "{unread:?} was read as a time");
        }
    }

    #[test]
    fn only_a_literal_yes_in_a_fresh_record_is_a_call() {
        let heard = judge(
            &[answer(&[record(
                "skiff",
                NOW,
                true,
                &json!({"call": true}),
            )])],
            HERE,
            AT + 30,
        );
        assert_eq!(
            heard.calls,
            ["skiff"],
            "a fresh yes wasn't a call: {heard:?}"
        );
        for (reached, why) in [
            (json!({"call": false}), "a no"),
            (json!({"call": "true"}), "a string"),
            (json!({"call": 1}), "a number"),
        ] {
            let heard = judge(&[answer(&[record("skiff", NOW, true, &reached)])], HERE, AT);
            assert!(heard.calls.is_empty(), "{why} was a call: {heard:?}");
        }
    }

    #[test]
    fn this_machines_own_record_is_passed_over() {
        let heard = judge(
            &[answer(&[record(HERE, NOW, true, &json!({"call": true}))])],
            HERE,
            AT,
        );
        assert_eq!(
            heard,
            Heard::default(),
            "its own call was heard as one elsewhere"
        );
    }

    #[test]
    fn a_probe_declared_with_no_yes_or_no_is_untold() {
        for (reached, why) in [
            (json!({}), "no reading"),
            (json!(null), "no probes reached"),
            (json!({"call": "true"}), "a string"),
        ] {
            let heard = judge(&[answer(&[record("skiff", NOW, true, &reached)])], HERE, AT);
            assert_eq!(heard.untold, ["skiff"], "{why} wasn't untold: {heard:?}");
        }
    }

    #[test]
    fn a_machine_declaring_no_call_probe_is_unseen_rather_than_a_wait() {
        let heard = judge(
            &[answer(&[record(
                "skiff",
                NOW,
                false,
                &json!({"home": true}),
            )])],
            HERE,
            AT,
        );
        assert_eq!(
            heard,
            Heard {
                unseen: vec!["skiff".to_owned()],
                ..Heard::default()
            },
            "a machine with no call probe wasn't named unseen alone"
        );
    }

    #[test]
    fn a_record_gone_stale_is_no_call() {
        for (measured_at, why) in [
            ("2026-11-17T14:12:59Z", "a record past three minutes old"),
            ("2026-11-17T14:00:00Z", "a record a quarter hour old"),
        ] {
            for declares_call in [true, false] {
                let heard = judge(
                    &[answer(&[record(
                        "skiff",
                        measured_at,
                        declares_call,
                        &json!({"call": false}),
                    )])],
                    HERE,
                    AT,
                );
                assert_eq!(heard, Heard::default(), "{why} counted: {heard:?}");
            }
        }
        let heard = judge(
            &[answer(&[record(
                "skiff",
                "2026-11-17T14:12:59Z",
                true,
                &json!({"call": true}),
            )])],
            HERE,
            AT,
        );
        assert_eq!(heard, Heard::default(), "a stale yes counted: {heard:?}");
        let heard = judge(
            &[answer(&[record(
                "skiff",
                "2026-11-17T14:13:00Z",
                true,
                &json!({"call": true}),
            )])],
            HERE,
            AT,
        );
        assert_eq!(heard.calls, ["skiff"], "a yes just fresh wasn't a call");
    }

    #[test]
    fn a_record_dated_ahead_or_with_no_time_is_undated() {
        for (measured_at, why) in [
            ("2026-11-17T14:17:01Z", "a record from the future"),
            ("not a time", "a record with no time"),
            (
                "999999999999999999-11-17T14:16:00Z",
                "a record from a year past any calendar",
            ),
        ] {
            for declares_call in [true, false] {
                let heard = judge(
                    &[answer(&[record(
                        "skiff",
                        measured_at,
                        declares_call,
                        &json!({"call": true}),
                    )])],
                    HERE,
                    AT,
                );
                assert_eq!(
                    heard,
                    Heard {
                        undated: vec!["skiff".to_owned()],
                        ..Heard::default()
                    },
                    "{why} wasn't undated alone"
                );
            }
        }
        let heard = judge(
            &[answer(&[record(
                "skiff",
                "2026-11-17T14:17:00Z",
                true,
                &json!({"call": true}),
            )])],
            HERE,
            AT,
        );
        assert_eq!(heard.calls, ["skiff"], "a yes a minute ahead wasn't a call");
    }

    #[test]
    fn a_name_keeps_only_printable_ascii() {
        let heard = judge(
            &[answer(&[
                record("sk\u{1b}]52;c;aGk=\u{7}iff", NOW, false, &json!({})),
                record("\u{1b}\u{7}", NOW, false, &json!({})),
                record("pu\u{202e}n\u{200b}t", NOW, false, &json!({})),
            ])],
            HERE,
            AT,
        );
        assert_eq!(
            heard.unseen,
            ["punt", "sk]52;c;aGk=iff"],
            "a name kept a control, bidi or zero-width character, \
             or one of nothing but them was named"
        );
    }

    #[test]
    fn a_long_name_is_cut_at_a_host_names_most() {
        let long = "s".repeat(10_000);
        let heard = judge(
            &[answer(&[record(&long, NOW, false, &json!({}))])],
            HERE,
            AT,
        );
        assert_eq!(
            heard.unseen,
            ["s".repeat(64)],
            "a name wasn't cut at 64 characters"
        );
    }

    #[test]
    fn records_past_the_most_machines_go_unnamed_but_a_call_among_them_counts() {
        let mut records: Vec<Value> = (0..100)
            .map(|n| record(&format!("m{n:03}"), NOW, false, &json!({})))
            .collect();
        records.push(record("skiff", NOW, true, &json!({"call": true})));
        let heard = judge(&[answer(&records)], HERE, AT);
        assert_eq!(heard.calls, ["skiff"], "a call past the most went unheard");
        assert_eq!(
            heard.unseen.len(),
            MACHINES_AT_MOST,
            "more machines were named than the most"
        );
        assert_eq!(
            heard.unnamed,
            100 - MACHINES_AT_MOST,
            "the records passed over weren't counted"
        );
    }

    #[test]
    fn a_machine_named_before_the_most_keeps_its_strongest_record_after() {
        let first: Vec<Value> = (0..MACHINES_AT_MOST)
            .map(|n| record(&format!("m{n:03}"), NOW, false, &json!({})))
            .collect();
        let later = answer(&[
            record("m000", NOW, true, &json!({})),
            record("quiet", NOW, true, &json!({"call": false})),
        ]);
        let heard = judge(&[answer(&first), later], HERE, AT);
        assert_eq!(
            heard.untold,
            ["m000"],
            "a machine already named lost its later realm's stronger record"
        );
        assert_eq!(
            heard.unnamed, 0,
            "a machine past the most on no call was counted as unnamed"
        );
    }

    #[test]
    fn a_machine_past_the_most_counts_once_by_its_strongest_record() {
        let first: Vec<Value> = (0..MACHINES_AT_MOST)
            .map(|n| record(&format!("m{n:03}"), NOW, false, &json!({})))
            .collect();
        let work = answer(&[
            record("over", NOW, false, &json!({})),
            record("both", NOW, false, &json!({})),
            record("late", NOW, true, &json!({})),
        ]);
        let home = answer(&[
            record("over", NOW, false, &json!({})),
            record("both", NOW, true, &json!({"call": false})),
            record("late", NOW, true, &json!({"call": true})),
        ]);
        let heard = judge(&[answer(&first), work, home], HERE, AT);
        assert_eq!(heard.calls, ["late"], "a call past the most went unheard");
        assert_eq!(
            heard.unnamed, 1,
            "a machine past the most was counted by each record, not once by its strongest"
        );
    }

    #[test]
    fn a_machine_in_two_realms_is_named_once_by_its_strongest_record() {
        let personal = answer(&[record("skiff", NOW, false, &json!({}))]);
        let work = answer(&[record("skiff", NOW, true, &json!({"call": true}))]);
        let heard = judge(&[personal, work], HERE, AT);
        assert_eq!(
            heard,
            Heard {
                calls: vec!["skiff".to_owned()],
                ..Heard::default()
            },
            "a call in one realm didn't win over unseen in the other"
        );
        let heard = judge(
            &[
                answer(&[record("skiff", NOW, true, &json!({}))]),
                answer(&[record("skiff", NOW, true, &json!({"call": false}))]),
            ],
            HERE,
            AT,
        );
        assert_eq!(heard, Heard::default(), "a fresh no didn't win over untold");
        // The stronger first, so the last record can't be what wins.
        let heard = judge(
            &[
                answer(&[record("skiff", NOW, true, &json!({"call": true}))]),
                answer(&[record("skiff", NOW, false, &json!({}))]),
            ],
            HERE,
            AT,
        );
        assert_eq!(heard.calls, ["skiff"], "a later realm's weaker record won");
        let first: Vec<Value> = (0..MACHINES_AT_MOST)
            .map(|n| record(&format!("m{n:03}"), NOW, false, &json!({})))
            .collect();
        let heard = judge(
            &[
                answer(&first),
                answer(&[record("over", NOW, true, &json!({"call": false}))]),
                answer(&[record("over", NOW, false, &json!({}))]),
            ],
            HERE,
            AT,
        );
        assert_eq!(
            heard.unnamed, 0,
            "past the most, a later realm's weaker record won"
        );
    }

    #[test]
    fn an_answer_that_isnt_a_list_of_machines_says_nothing() {
        let heard = judge(
            &[json!({"Err": "no such realm"}), json!("Machines")],
            HERE,
            AT,
        );
        assert_eq!(heard, Heard::default());
    }
}
