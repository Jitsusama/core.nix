//! How Joel's apps answer while the machine is under every pressure at
//! once, from apps-under-load's hourly read: a call's frames and freezes,
//! Chrome's and Slack's input to paint, and how fast the launcher opens.
//! Joel ranked these above everything else on 19 November ("have
//! everything be very quick to respond ... run insanely high quality
//! video and audio while the machine is totally under all the
//! pressures"), so the card leads with them and their drag comes first.

use serde_json::Value;

use super::{Class, Measure};

/// Where apps-under-load writes its hour, beside the state home.
pub const FILE: &str = "apps-under-load/hour.json";

/// Who keeps apps-under-load and its figures (WCJQSG, 19 November).
const OWNER: &str = "5193FV";

/// How old an hour may be and still be read: two of them, so one missed
/// write is forgiven and a stopped one isn't taken for passing.
const STALE: i64 = 2 * 3600;

/// The key the one line takes when there's no hour to read.
const UNREAD: &str = "apps-under-load";

/// Each figure in apps-under-load's `hour.json`, written as code-health
/// writes its hour: `{"at": seconds, "measures": [{"measure", "now",
/// "target", "lower_is_better", "owner", "behind"}]}`. A figure better
/// higher, such as frames a second, is judged by how far short of its
/// target it falls, so every line reads as at most a target like the rest
/// of the card. With no hour, or one older than two, a single unjudged
/// line says it wasn't read.
#[must_use]
pub fn apps(text: Option<&str>, now: i64) -> Vec<Measure> {
    let read: Value = text
        .and_then(|text| serde_json::from_str(text).ok())
        .unwrap_or_default();
    let fresh = read["at"]
        .as_i64()
        .is_some_and(|at| at > now - STALE && at <= now);
    let figures: Vec<Measure> = if fresh {
        read["measures"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(figure)
            .collect()
    } else {
        Vec::new()
    };
    if figures.is_empty() {
        return vec![Measure {
            key: UNREAD.to_owned(),
            class: Class::Apps,
            // apps-under-load leaves its hour unwritten while a call is on.
            says: "his apps under load, unread: no hour in 2 h, a call or a fault".to_owned(),
            now: 0.0,
            target: None,
            owner: OWNER.to_owned(),
            behind: Vec::new(),
        }];
    }
    figures
}

/// One figure as a measure, or none without a name or a reading.
fn figure(m: &Value) -> Option<Measure> {
    let measure = m["measure"].as_str()?;
    let now = m["now"].as_f64()?;
    let target = m["target"].as_f64();
    let lower = m["lower_is_better"].as_bool().unwrap_or(true);
    let (says, now, target) = match target {
        Some(target) if !lower => (
            format!("{measure} short of {}", super::super::lines::figure(target)),
            (target - now).max(0.0),
            Some(0.0),
        ),
        _ => (measure.to_owned(), now, target),
    };
    Some(Measure {
        key: format!("apps:{measure}"),
        class: Class::Apps,
        says,
        now,
        target,
        owner: m["owner"].as_str().unwrap_or(OWNER).to_owned(),
        behind: m["behind"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_794_700_000;

    fn hour(at: i64) -> String {
        serde_json::json!({"at": at, "measures": [
            {"measure": "call fps p10", "now": 24.0, "target": 30.0,
             "lower_is_better": false, "owner": "5193FV", "behind": ["encoder starved"]},
            {"measure": "launcher open ms p99", "now": 180.0, "target": 120.0,
             "lower_is_better": true},
            {"measure": "freezes per 10 minutes", "now": 0.0, "target": 0.0,
             "lower_is_better": true},
            {"measure": "slack input to paint ms", "now": 90.0, "target": null},
        ]})
        .to_string()
    }

    #[test]
    fn each_figure_reads_as_at_most_a_target_and_one_better_higher_by_its_shortfall() {
        let read = apps(Some(&hour(NOW - 600)), NOW);
        let lines: Vec<(&str, &str, f64, Option<f64>)> = read
            .iter()
            .map(|m| (m.key.as_str(), m.says.as_str(), m.now, m.target))
            .collect();
        assert_eq!(
            lines,
            [
                (
                    "apps:call fps p10",
                    "call fps p10 short of 30",
                    6.0,
                    Some(0.0)
                ),
                (
                    "apps:launcher open ms p99",
                    "launcher open ms p99",
                    180.0,
                    Some(120.0)
                ),
                (
                    "apps:freezes per 10 minutes",
                    "freezes per 10 minutes",
                    0.0,
                    Some(0.0)
                ),
                (
                    "apps:slack input to paint ms",
                    "slack input to paint ms",
                    90.0,
                    None
                ),
            ]
        );
        assert_eq!(read[0].behind, ["encoder starved"]);
        assert_eq!(
            read[1].owner, "5193FV",
            "its keeper, when a figure names none"
        );
        assert!(read.iter().all(|m| m.class == Class::Apps));
    }

    #[test]
    fn an_hour_missing_torn_or_two_hours_old_is_unread_not_passing() {
        for text in [
            None,
            Some("{"),
            Some(r#"{"at": 1794700000, "measures": []}"#),
        ] {
            let read = apps(text, NOW);
            assert_eq!(read.len(), 1, "one line for {text:?}");
            assert_eq!(read[0].target, None, "unjudged for {text:?}");
            assert!(read[0].says.contains("unread"), "{}", read[0].says);
        }
        let old = apps(Some(&hour(NOW - STALE)), NOW);
        assert_eq!(old.len(), 1, "an hour two old is not read");
        assert_eq!(old[0].key, "apps-under-load");
        assert_eq!(apps(Some(&hour(NOW - STALE + 1)), NOW).len(), 4);
    }

    #[test]
    fn his_apps_lead_the_drag_ahead_of_his_misses() {
        let mut miss = apps(None, NOW).remove(0);
        miss.class = Class::Miss;
        miss.target = Some(0.0);
        miss.now = 3.0;
        let read = apps(Some(&hour(NOW - 600)), NOW);
        let mut all = vec![miss];
        all.extend(read);
        let drag = super::super::drag(&all).expect("two past their targets");
        assert_eq!(drag.class, Class::Apps, "his apps come before every class");
        // Six frames short of a target of nought is six times past it, and
        // the launcher's 180 against 120 half again: a call he can see
        // stutter leads, as it led his list.
        assert_eq!(drag.key, "apps:call fps p10", "the furthest past");
    }
}
