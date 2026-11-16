//! How evenly the realm's machines carried agents' work over the hour,
//! read from the minute lines mycelium's balance service keeps
//! (`mycelium balance --follow`, one file a day). On 15 November this
//! laptop carried most of the realm's sessions while the work Mac had
//! room, and nothing said how long that lasted: crowded names a moment,
//! this names the hour, each machine's share of it and the minutes each
//! of balance's flags was held.

use std::collections::BTreeMap;

use serde_json::Value;

use super::{Class, Measure, count, lines, seconds};

const HOUR: i64 = 3600;

/// The definitions read. A newer one may weigh a load differently (the
/// second counts processor use as well as pressure), so an hour holding
/// two takes the newer's minutes alone rather than compare across them.
const DEFINITIONS: [&str; 2] = ["balance/1", "balance/2"];

/// One machine's share of the hour, summed over the minutes it was read.
#[derive(Default)]
struct Share {
    minutes: usize,
    busy: f64,
    idle: f64,
    per_core: f64,
    load: f64,
}

impl Share {
    fn add(&mut self, machine: &Value) {
        let number = |key: &str| machine[key].as_f64().unwrap_or_default();
        self.minutes += 1;
        self.busy += number("busy");
        self.idle += number("idle");
        self.per_core += number("busy_per_core");
        self.load += number("load");
    }

    fn mean(&self, sum: f64) -> f64 {
        sum / count(self.minutes.max(1))
    }
}

/// A flag as the card says it, from balance's own fields.
fn worded(flag: &Value) -> String {
    let field = |key: &str| flag[key].as_str().unwrap_or("a machine");
    match flag["flag"].as_str() {
        Some("hot_while_cool") => format!("{} over while {} had room", field("hot"), field("cool")),
        Some("stranded") => format!(
            "{}'s {} spent while the rest idled",
            field("machine"),
            field("spent")
        ),
        Some("stale") => format!("{}'s facts too old to place by", field("machine")),
        Some("imbalanced") => "the machines' loads spread past balance's line".to_owned(),
        other => format!("{} flagged", other.unwrap_or("something")),
    }
}

/// The measure from `minutes`, the balance service's lines, at `now`: the
/// mean spread of the machines' loads over the last hour, with each
/// machine's share busiest first and the minutes each flag was held; none
/// when no minute of a known definition falls in the hour.
#[must_use]
pub fn balanced(minutes: &str, now: i64, owner: &str) -> Option<Measure> {
    let hour: Vec<(i64, Value)> = lines(minutes)
        .filter(|line| DEFINITIONS.contains(&line["measures"].as_str().unwrap_or_default()))
        .filter_map(|line| Some((seconds(&line["at"])?, line)))
        .filter(|(at, _)| (now - HOUR..=now).contains(at))
        .collect();
    let newest = hour.iter().max_by_key(|(at, _)| *at)?.1["measures"].clone();
    let taken: Vec<&Value> = hour
        .iter()
        .map(|(_, line)| line)
        .filter(|line| line["measures"] == newest)
        .collect();
    let spreads: Vec<f64> = taken
        .iter()
        .filter_map(|line| line["imbalance"].as_f64())
        .collect();
    let mut shares: BTreeMap<String, Share> = BTreeMap::new();
    let mut held: BTreeMap<String, usize> = BTreeMap::new();
    for line in &taken {
        for machine in line["machines"].as_array().into_iter().flatten() {
            if let Some(name) = machine["machine"].as_str() {
                shares.entry(name.to_owned()).or_default().add(machine);
            }
        }
        for flag in line["held"].as_array().into_iter().flatten() {
            *held.entry(worded(flag)).or_default() += 1;
        }
    }
    // The card's first line is the one that matters, so the machine with
    // the most busy sessions a core comes first.
    let mut busiest: Vec<(String, Share)> = shares.into_iter().collect();
    busiest.sort_by(|(_, a), (_, b)| {
        b.mean(b.per_core)
            .total_cmp(&a.mean(a.per_core))
            .then(b.mean(b.busy).total_cmp(&a.mean(a.busy)))
    });
    let mut behind: Vec<String> = busiest
        .iter()
        .map(|(name, share)| {
            format!(
                "{name}: {:.1} busy and {:.1} idle sessions, {:.2} busy a core, load {:.2}",
                share.mean(share.busy),
                share.mean(share.idle),
                share.mean(share.per_core),
                share.mean(share.load),
            )
        })
        .collect();
    let mut flags: Vec<(String, usize)> = held.into_iter().collect();
    flags.sort_by(|(a, x), (b, y)| y.cmp(x).then(a.cmp(b)));
    let of = taken.len();
    behind.extend(
        flags
            .into_iter()
            .map(|(flag, held)| format!("{flag}: held {held} of {of} minutes")),
    );
    if taken.len() < hour.len() {
        let name = newest.as_str().unwrap_or_default();
        behind.push(format!(
            "only {name}'s minutes, since a load under another definition isn't the same"
        ));
    }
    Some(Measure {
        key: "balance".to_owned(),
        class: Class::Ship,
        says: "how far the machines' loads spread over the hour".to_owned(),
        now: spreads.iter().sum::<f64>() / count(spreads.len().max(1)),
        target: None,
        owner: owner.to_owned(),
        behind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_763_298_000;

    const HOT: &str = r#"{"flag":"hot_while_cool","hot":"kestrel","cool":"osprey"}"#;

    /// A machine in a minute line: its name, busy sessions, busy a core
    /// and load.
    type Machine<'a> = (&'a str, u32, f64, f64);

    /// A minute line at `ago` seconds before [`NOW`], under `measures`,
    /// with `machines` and the flags `held`.
    fn minute(
        ago: i64,
        measures: &str,
        imbalance: f64,
        machines: &[Machine],
        held: &str,
    ) -> String {
        let at = jiff::Timestamp::from_second(NOW - ago).expect("a time in range");
        let machines: Vec<Value> = machines
            .iter()
            .map(|(name, busy, per_core, load)| {
                serde_json::json!({
                    "machine": name, "ready": true, "load": load,
                    "busy": busy, "idle": 10, "busy_per_core": per_core,
                })
            })
            .collect();
        let held: Value = serde_json::from_str(&format!("[{held}]")).expect("held flags as JSON");
        serde_json::json!({
            "measures": measures, "at": at.to_string(), "machines": machines,
            "imbalance": imbalance, "flags": [], "held": held,
        })
        .to_string()
    }

    #[test]
    fn the_hour_says_the_mean_spread_and_each_machine_busiest_first() {
        let minutes = [
            minute(
                120,
                "balance/1",
                0.10,
                &[("osprey", 2, 0.2, 0.1), ("kestrel", 20, 1.5, 0.9)],
                HOT,
            ),
            minute(
                60,
                "balance/1",
                0.30,
                &[("osprey", 4, 0.4, 0.3), ("kestrel", 22, 1.7, 0.7)],
                "",
            ),
        ]
        .join("\n");
        let measure = balanced(&minutes, NOW, "OWNER").expect("an hour with minutes in it");
        assert!(
            (measure.now - 0.2).abs() < 1e-9,
            "the spread wasn't the hour's mean: {}",
            measure.now
        );
        assert_eq!(
            measure.behind,
            [
                "kestrel: 21.0 busy and 10.0 idle sessions, 1.60 busy a core, load 0.80",
                "osprey: 3.0 busy and 10.0 idle sessions, 0.30 busy a core, load 0.20",
                "kestrel over while osprey had room: held 1 of 2 minutes",
            ],
            "the shares weren't busiest first, or the flag's minutes were miscounted"
        );
        assert_eq!(
            measure.target, None,
            "a measure without a week of readings was judged"
        );
    }

    #[test]
    fn minutes_outside_the_hour_or_of_an_unknown_definition_are_left_out() {
        let lone = [("kestrel", 1, 0.1, 0.1)];
        let minutes = [
            minute(HOUR + 60, "balance/1", 0.9, &lone, HOT),
            minute(60, "balance/9", 0.9, &lone, HOT),
            "not json".to_owned(),
            minute(30, "balance/1", 0.1, &lone, ""),
        ]
        .join("\n");
        let measure = balanced(&minutes, NOW, "OWNER").expect("one minute in the hour");
        assert!(
            (measure.now - 0.1).abs() < 1e-9,
            "a minute from outside the hour counted: {}",
            measure.now
        );
        assert_eq!(
            measure.behind.len(),
            1,
            "a flag from outside the hour was counted: {:?}",
            measure.behind
        );
    }

    #[test]
    fn an_hour_across_two_definitions_takes_the_newer_alone() {
        let lone = [("kestrel", 1, 0.1, 0.1)];
        let minutes = [
            minute(120, "balance/1", 0.9, &lone, ""),
            minute(60, "balance/2", 0.2, &lone, ""),
        ]
        .join("\n");
        let measure = balanced(&minutes, NOW, "OWNER").expect("an hour with minutes in it");
        assert!(
            (measure.now - 0.2).abs() < 1e-9,
            "the older definition's spread was mixed in: {}",
            measure.now
        );
        assert!(
            measure
                .behind
                .last()
                .is_some_and(|line| line.starts_with("only balance/2's minutes")),
            "the card didn't say the older minutes were left out: {:?}",
            measure.behind
        );
    }

    #[test]
    fn an_hour_with_no_minutes_has_no_measure() {
        assert_eq!(
            balanced("", NOW, "OWNER"),
            None,
            "an hour with no minutes was measured"
        );
    }
}
