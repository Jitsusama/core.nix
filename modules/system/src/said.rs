//! What a machine's ties to the other three parts (parts/ties.rs) are
//! said in, in a terminal: its cells, its network, its code and itself,
//! each as its main words and what goes beside them. The system page says
//! the same ties in the same words (site/machine.ts), each opening with a
//! capital there and lower case here, where each follows its label.

use serde_json::Value;
use terminal_kit::time::span;

/// A gigabyte and a terabyte, as a node counts bytes.
const GIGABYTE: f64 = 1e9;
const TERABYTE: f64 = 1e12;

/// The bars at or over which a radio's signal is said strong, and fair.
const STRONG_BARS: u64 = 4;
const FAIR_BARS: u64 = 3;

/// A full charge, and the whole of a share.
const FULL: f64 = 100.0;

/// A day, in seconds.
const DAY: i64 = 86_400;

/// How each system a node names is said.
const SYSTEMS: [(&str, &str); 2] = [("linux", "Linux"), ("macos", "macOS")];

const NOT_READ: &str = "not read";

/// A tie said: its main words, and what goes beside them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Said {
    pub said: String,
    pub aside: String,
}

impl Said {
    fn new(said: impl Into<String>, aside: impl Into<String>) -> Self {
        Self {
            said: said.into(),
            aside: aside.into(),
        }
    }
}

/// A machine's cells, busy and idle, with those that have ended aside.
pub fn cells(ties: &Value) -> Said {
    let Some(cells) = ties.get("cells").filter(|cells| cells.is_object()) else {
        return Said::new(NOT_READ, "the mycelium's health couldn't be had");
    };
    let of = |key: &str| cells.get(key).and_then(Value::as_u64).unwrap_or(0);
    let busy = match of("busy") {
        0 => "none".to_owned(),
        busy => grouped(busy),
    };
    let aside = match of("ended") {
        0 => "none has ended".to_owned(),
        ended => format!("and {} that have ended", grouped(ended)),
    };
    Said::new(
        format!("{busy} busy and {} idle", grouped(of("idle"))),
        aside,
    )
}

/// Where a machine meets the network, a clause for each place, and how
/// well: its radio's signal, and the most room any place has.
pub fn network(ties: &Value) -> Said {
    let Some(places) = ties.get("network").and_then(Value::as_array) else {
        return Said::new(NOT_READ, "the network's answer couldn't be had");
    };
    if places.is_empty() {
        return Said::new(
            "not seen on the home network",
            "no device on it carries this machine's name",
        );
    }
    let said = places.iter().map(meets).collect::<Vec<_>>().join(", and ");
    let how = places
        .iter()
        .filter(|place| place.get("by").and_then(Value::as_str) != Some("wired"))
        .find_map(|place| place.get("bars").and_then(Value::as_u64))
        .map(signal);
    let room = places
        .iter()
        .filter_map(|place| place.get("room_mbps").and_then(Value::as_f64))
        .reduce(f64::max)
        .map(|mbps| format!("{} Mb/s of room", mbps.round()));
    let aside = match (how, room) {
        (Some(how), Some(room)) => format!("{how}, with {room}"),
        (Some(how), None) => how.to_owned(),
        (None, Some(room)) => format!("with {room}"),
        (None, None) => "how much room it has isn't measured".to_owned(),
    };
    Said::new(said, aside)
}

/// Where one place meets the network, as a clause. Wi-Fi is a name, so it
/// keeps its capitals.
fn meets(place: &Value) -> String {
    let point = place.get("point").and_then(Value::as_str).map_or_else(
        || "an unnamed point".to_owned(),
        |point| format!("the {point} point"),
    );
    match place.get("by").and_then(Value::as_str) {
        Some("wired") => format!("wired to {point}"),
        Some(band) => format!("Wi-Fi to {point}, on {band}"),
        None => format!("on {point}"),
    }
}

/// A radio's signal, said from its bars.
fn signal(bars: u64) -> &'static str {
    if bars >= STRONG_BARS {
        "a strong signal"
    } else if bars >= FAIR_BARS {
        "a fair signal"
    } else {
        "a weak signal"
    }
}

/// A machine's code, counted where it was read, and said not read from
/// another, as `repos` reads only the machine it runs on.
pub fn code(ties: &Value, name: &str) -> Said {
    let Some(code) = ties.get("code").filter(|code| code.is_object()) else {
        return Said::new(
            "not read from here",
            format!("its code is read on {name} itself"),
        );
    };
    let of = |key: &str| code.get(key).and_then(Value::as_u64).unwrap_or(0);
    Said::new(
        format!(
            "{} in {}",
            many(of("repositories"), "repository", "repositories"),
            many(of("checkouts"), "checkout", "checkouts")
        ),
        format!(
            "{}; {} behind their upstream; {}",
            many(of("trees"), "tree", "trees"),
            many(of("behind"), "checkout", "checkouts"),
            many(of("changes"), "open change", "open changes")
        ),
    )
}

/// A machine itself: its disk's room, then its processor, memory and
/// power, then how long ago it was updated, each said only when its node
/// says it. `now` is the moment it's said at, in seconds since 1970.
pub fn itself(ties: &Value, now: i64) -> Said {
    let figures = ties.get("figures").unwrap_or(&Value::Null);
    let number = |key: &str| figures.get(key).and_then(Value::as_f64);
    let said = match (number("disk_free"), number("disk")) {
        (None, _) => "its disk not said".to_owned(),
        (Some(free), None) => format!("{} free", size(free)),
        (Some(free), Some(disk)) => format!("{} of {} free", size(free), size(disk)),
    };
    let mut clauses = Vec::new();
    if let Some(busy) = number("cpu_busy") {
        clauses.push(format!("processor {}% busy", (busy * FULL).round()));
    }
    if let (Some(free), Some(memory)) = (number("memory_available"), number("memory")) {
        clauses.push(format!(
            "{} of {} of memory free",
            (free / GIGABYTE).round(),
            size(memory)
        ));
    }
    if let Some(charge) = number("charge") {
        let on_battery = figures.get("on_battery").and_then(Value::as_bool) == Some(true);
        clauses.push(if on_battery {
            format!("{charge}% charged, on battery")
        } else if charge >= FULL {
            "fully charged and plugged in".to_owned()
        } else {
            format!("{charge}% charged and plugged in")
        });
    }
    let parts = [
        (!clauses.is_empty()).then(|| clauses.join(", ")),
        updated(figures, now),
    ];
    Said::new(
        said,
        parts.into_iter().flatten().collect::<Vec<_>>().join("; "),
    )
}

/// A machine's last update, as a clause, when its node says one: how long
/// ago it was, where its time reads, and how it ended.
fn updated(figures: &Value, now: i64) -> Option<String> {
    let at = figures.get("updated_at").and_then(Value::as_str)?;
    let ago =
        moment(at).map(|then| format!("{} ago", span(now.saturating_sub(then).unsigned_abs())));
    let step = figures.get("update_step").and_then(Value::as_str);
    Some(match (step, ago) {
        (Some("done"), Some(ago)) => format!("updated {ago}"),
        (Some("done"), None) => "updated".to_owned(),
        (Some("unchanged"), Some(ago)) => format!("unchanged at its update {ago}"),
        (Some("unchanged"), None) => "unchanged at its last update".to_owned(),
        (None, Some(ago)) => format!("last updated {ago}"),
        (None, None) => return None,
        (Some(step), Some(ago)) => format!("its update {ago} says {step}"),
        (Some(step), None) => format!("its last update says {step}"),
    })
}

/// What kind of machine it is: its model and its system, and whether it's
/// the one `here` names.
pub fn kind(ties: &Value, name: &str, here: &str) -> Option<String> {
    let figures = ties.get("figures")?;
    let model = figures.get("model").and_then(Value::as_str);
    let system = figures.get("system").and_then(Value::as_str).map(|system| {
        SYSTEMS
            .iter()
            .find(|(named, _)| *named == system)
            .map_or(system, |(_, said)| said)
    });
    let words: Vec<&str> = [model, system].into_iter().flatten().collect();
    if words.is_empty() {
        return None;
    }
    let this = (name == here).then_some("this machine");
    Some(words.into_iter().chain(this).collect::<Vec<_>>().join(", "))
}

/// `count` of a thing, `one` or `more` as it's one or not.
fn many(count: u64, one: &str, more: &str) -> String {
    format!("{} {}", grouped(count), if count == 1 { one } else { more })
}

/// A count with its thousands set apart by commas, as Canada writes one.
fn grouped(count: u64) -> String {
    let digits = count.to_string();
    let mut out = String::new();
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// A size in bytes, in gigabytes, or terabytes to a tenth from a thousand
/// of them.
fn size(bytes: f64) -> String {
    if bytes >= TERABYTE {
        let tenths = (bytes / TERABYTE * 10.0).round() / 10.0;
        return format!("{tenths} TB");
    }
    format!("{} GB", (bytes / GIGABYTE).round())
}

/// The moment an RFC 3339 time in UTC names, `2026-11-17T22:45:18Z`, in
/// seconds since 1970, by Howard Hinnant's days-from-civil arithmetic, as
/// the kit's `time` says one; none for any other form.
fn moment(at: &str) -> Option<i64> {
    let at = at.strip_suffix('Z')?;
    let (date, time) = at.split_once('T')?;
    let numbers = |text: &str, sep: char| -> Option<Vec<i64>> {
        text.split(sep).map(|part| part.parse().ok()).collect()
    };
    let (&[year, month, day], &[hour, minute, second]) = (
        numbers(date, '-')?.as_slice(),
        numbers(time, ':')?.as_slice(),
    ) else {
        return None;
    };
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let of_era_days = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    let days = era * 146_097 + of_era_days - 719_468;
    Some(days * DAY + hour * 3600 + minute * 60 + second)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn said(said: &str, aside: &str) -> Said {
        Said {
            said: said.to_owned(),
            aside: aside.to_owned(),
        }
    }

    #[test]
    fn cells_say_the_busy_and_the_idle_with_the_ended_aside() {
        let of = |busy: u64, idle: u64, ended: u64| {
            cells(&json!({"cells": {"busy": busy, "idle": idle, "ended": ended}}))
        };
        assert_eq!(
            of(2, 1_204, 0),
            said("2 busy and 1,204 idle", "none has ended")
        );
        assert_eq!(
            of(0, 3, 5),
            said("none busy and 3 idle", "and 5 that have ended")
        );
        assert_eq!(
            cells(&json!({"cells": null})),
            said("not read", "the mycelium's health couldn't be had")
        );
    }

    #[test]
    fn the_network_says_each_place_and_how_well_it_reaches() {
        let at = |places: Value| network(&json!({"network": places}));
        let wifi = json!({"by": "5 GHz", "point": "Hall", "bars": 5, "room_mbps": 183});
        let wired = json!({"by": "wired", "point": "Den", "bars": null, "room_mbps": 941});
        assert_eq!(
            at(json!([wired, wifi])),
            said(
                "wired to the Den point, and Wi-Fi to the Hall point, on 5 GHz",
                "a strong signal, with 941 Mb/s of room"
            )
        );
        let weak = json!({"by": "2.4 GHz", "point": null, "bars": 2, "room_mbps": null});
        assert_eq!(
            at(json!([weak])),
            said("Wi-Fi to an unnamed point, on 2.4 GHz", "a weak signal")
        );
        let unknown = json!({"by": null, "point": "Hall", "bars": null, "room_mbps": null});
        assert_eq!(
            at(json!([unknown])),
            said("on the Hall point", "how much room it has isn't measured")
        );
        assert_eq!(
            at(json!([])),
            said(
                "not seen on the home network",
                "no device on it carries this machine's name"
            )
        );
        assert_eq!(
            at(Value::Null),
            said("not read", "the network's answer couldn't be had")
        );
    }

    #[test]
    fn code_is_counted_where_it_was_read_and_said_unread_elsewhere() {
        let read = json!({"code": {"repositories": 12, "checkouts": 1, "trees": 3,
                                   "behind": 1, "changes": 0}});
        assert_eq!(
            code(&read, "kestrel"),
            said(
                "12 repositories in 1 checkout",
                "3 trees; 1 checkout behind their upstream; 0 open changes"
            )
        );
        assert_eq!(
            code(&json!({"code": null}), "osprey"),
            said("not read from here", "its code is read on osprey itself")
        );
    }

    #[test]
    fn a_machine_itself_says_only_what_its_node_says() {
        let now = 1_763_420_718; // 2025-11-17T23:05:18Z, 20 minutes after the update.
        let whole = json!({"figures": {
            "disk": 1_023_000_000_000_u64, "disk_free": 169_000_000_000_u64,
            "cpu_busy": 0.9, "memory": 67_000_000_000_u64,
            "memory_available": 30_000_000_000_u64, "charge": 79, "on_battery": false,
            "updated_at": "2025-11-17T22:45:18Z", "update_step": "done"}});
        assert_eq!(
            itself(&whole, now),
            said(
                "169 GB of 1 TB free",
                "processor 90% busy, 30 of 67 GB of memory free, 79% charged and plugged in; \
                 updated 20 m ago"
            )
        );
        let bare = json!({"figures": {"disk": null, "disk_free": null, "charge": 100,
                                      "on_battery": false}});
        assert_eq!(
            itself(&bare, now),
            said("its disk not said", "fully charged and plugged in")
        );
        let draining = json!({"figures": {"disk_free": 2_000_000_000_000_u64, "charge": 40,
                                          "on_battery": true, "updated_at": "soon",
                                          "update_step": "building"}});
        assert_eq!(
            itself(&draining, now),
            said(
                "2 TB free",
                "40% charged, on battery; its last update says building"
            )
        );
    }

    #[test]
    fn a_machine_is_its_model_and_system_and_says_when_its_this_one() {
        let ties = json!({"figures": {"model": "Example 14", "system": "macos"}});
        assert_eq!(
            kind(&ties, "osprey", "kestrel").as_deref(),
            Some("Example 14, macOS")
        );
        assert_eq!(
            kind(&ties, "kestrel", "kestrel").as_deref(),
            Some("Example 14, macOS, this machine")
        );
        let odd = json!({"figures": {"model": null, "system": "plan9"}});
        assert_eq!(kind(&odd, "tern", "kestrel").as_deref(), Some("plan9"));
        assert_eq!(kind(&json!({"figures": {}}), "tern", "tern"), None);
    }
}
