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
//!   call, so a closed laptop holds nothing back.
//!
//! Untold and unseen machines, and a node here that won't answer, are
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
    /// Why each node here that didn't answer couldn't, naming its socket.
    pub unanswered: Vec<String>,
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
        ..judge(&asked.answers, &here, now)
    }
}

/// What one machine's records say, weakest first, so the strongest of
/// its realms' records wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Said {
    Unseen,
    Untold,
    NoCall,
    Call,
}

/// What the nodes' answers to a `Machines` request say of every machine
/// but `here`, `now` in seconds since the epoch. An answer that isn't a
/// node's list of machines says nothing; the caller counts a node that
/// couldn't answer.
pub fn judge(answers: &[Value], here: &str, now: i64) -> Heard {
    let mut machines: BTreeMap<&str, Said> = BTreeMap::new();
    let records = answers
        .iter()
        .filter_map(|answer| answer["Ok"]["Machines"].as_array())
        .flatten();
    for record in records {
        let Some(name) = record["name"].as_str().filter(|name| *name != here) else {
            continue;
        };
        let Some(said) = said(record, now) else {
            continue;
        };
        let strongest = machines.entry(name).or_insert(said);
        *strongest = (*strongest).max(said);
    }
    let mut heard = Heard::default();
    for (name, said) in machines {
        let list = match said {
            Said::Call => &mut heard.calls,
            Said::Untold => &mut heard.untold,
            Said::Unseen => &mut heard.unseen,
            Said::NoCall => continue,
        };
        list.push(name.to_owned());
    }
    heard
}

/// What one machine's record says, or nothing when it isn't fresh.
fn said(record: &Value, now: i64) -> Option<Said> {
    let measured = record["measured_at"].as_str().and_then(seconds_of)?;
    if !(now - FRESH..=now + 60).contains(&measured) {
        return None;
    }
    if record["declared"]["probes"].get("call").is_none() {
        return Some(Said::Unseen);
    }
    Some(match record["reached"]["call"] {
        Value::Bool(true) => Said::Call,
        Value::Bool(false) => Said::NoCall,
        _ => Said::Untold,
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
    let in_range = (1..=12).contains(month)
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
            ("2026-11-17T14:17:01Z", "a record from the future"),
            ("not a time", "a record with no time"),
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
                assert_eq!(heard, Heard::default(), "{why} counted: {heard:?}");
            }
        }
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
