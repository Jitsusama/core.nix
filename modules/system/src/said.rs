//! What a machine's ties to the other three parts (parts/ties.rs) are
//! said in, in a terminal: its cells, its network, its code and itself,
//! each as its main words and what goes beside them. The system page's
//! machines say the same ties in the same words, each opening with a
//! capital there and lower case here, where each follows its label; a
//! word changed here is changed there in the same change.

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
    let none_or = |count: u64| match count {
        0 => "none".to_owned(),
        count => grouped(count),
    };
    let said = match (of("busy"), of("idle")) {
        (0, 0) => "none busy or idle".to_owned(),
        (busy, idle) => format!("{} busy and {} idle", none_or(busy), none_or(idle)),
    };
    let aside = match of("ended") {
        0 => "none has ended".to_owned(),
        ended => format!("and {} that have ended", grouped(ended)),
    };
    Said::new(said, aside)
}

/// Where a machine meets the network, a clause for each place, and how
/// well each reaches, in the same order: its radio's signal and its room,
/// so no place's figure is said as another's.
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
    let reaches: Vec<String> = places.iter().filter_map(reach).collect();
    let aside = if reaches.is_empty() {
        "how much room it has isn't measured".to_owned()
    } else {
        reaches.join("; ")
    };
    Said::new(said, aside)
}

/// How well one place reaches, as far as it's measured: a radio's signal,
/// then its room.
fn reach(place: &Value) -> Option<String> {
    let how = (place.get("by").and_then(Value::as_str) != Some("wired"))
        .then(|| place.get("bars").and_then(Value::as_u64).map(signal))
        .flatten();
    let room = place
        .get("room_mbps")
        .and_then(Value::as_f64)
        .map(|mbps| format!("{} Mb/s of room", mbps.round()));
    match (how, room) {
        (Some(how), Some(room)) => Some(format!("{how}, with {room}")),
        (Some(how), None) => Some(how.to_owned()),
        (None, room) => room,
    }
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
/// another, as `repos` reads only the machine it runs on; on the machine
/// it's read on, `here`, no code is a read that failed.
pub fn code(ties: &Value, name: &str, here: bool) -> Said {
    let Some(code) = ties.get("code").filter(|code| code.is_object()) else {
        if here {
            return Said::new(NOT_READ, "repos couldn't be had");
        }
        return Said::new(
            "not read from here",
            format!("its code is read on {name} itself"),
        );
    };
    let of = |key: &str| code.get(key).and_then(Value::as_u64).unwrap_or(0);
    let behind = match of("behind") {
        behind @ (0 | 1) => format!(
            "{} behind its upstream",
            many(behind, "checkout", "checkouts")
        ),
        behind => format!(
            "{} behind their upstreams",
            many(behind, "checkout", "checkouts")
        ),
    };
    Said::new(
        format!(
            "{} of {}",
            many(of("checkouts"), "checkout", "checkouts"),
            many(of("repositories"), "repository", "repositories")
        ),
        format!(
            "{}; {behind}; {}",
            many(of("trees"), "tree", "trees"),
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
        let full = charge >= FULL;
        clauses.push(
            match (figures.get("on_battery").and_then(Value::as_bool), full) {
                (Some(true), _) => format!("{charge}% charged, on battery"),
                (Some(false), true) => "fully charged and plugged in".to_owned(),
                (Some(false), false) => format!("{charge}% charged and plugged in"),
                (None, true) => "fully charged".to_owned(),
                (None, false) => format!("{charge}% charged"),
            },
        );
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
/// ago it was, where its time reads and isn't ahead of this clock, and
/// how it ended.
fn updated(figures: &Value, now: i64) -> Option<String> {
    let at = figures.get("updated_at").and_then(Value::as_str)?;
    let ago = moment(at)
        .filter(|then| *then <= now)
        .map(|then| format!("{} ago", span(now.saturating_sub(then).unsigned_abs())));
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

/// `count` of a thing, `one` or `more` as it's one or not, and no `one`
/// for none.
fn many(count: u64, one: &str, more: &str) -> String {
    match count {
        0 => format!("no {one}"),
        1 => format!("1 {one}"),
        count => format!("{} {more}", grouped(count)),
    }
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
/// seconds since 1970, as the kit's `time` says one; none for any other
/// form, or for a field out of its range, which the kit wouldn't say back
/// as it was given.
fn moment(at: &str) -> Option<i64> {
    civil(at).filter(|seconds| terminal_kit::time::rfc3339(*seconds) == at)
}

/// The seconds since 1970 an RFC 3339 time's fields add up to, by Howard
/// Hinnant's days-from-civil arithmetic, whatever their range.
fn civil(at: &str) -> Option<i64> {
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
        assert_eq!(of(4, 0, 0), said("4 busy and none idle", "none has ended"));
        assert_eq!(
            of(0, 0, 1),
            said("none busy or idle", "and 1 that have ended")
        );
        assert_eq!(
            cells(&json!({"cells": null})),
            said("not read", "the mycelium's health couldn't be had")
        );
    }

    #[test]
    fn the_network_says_each_place_and_how_well_each_reaches() {
        let at = |places: Value| network(&json!({"network": places}));
        let wifi = json!({"by": "5 GHz", "point": "Hall", "bars": 5, "room_mbps": 183});
        let wired = json!({"by": "wired", "point": "Den", "bars": null, "room_mbps": 941});
        assert_eq!(
            at(json!([wired, wifi.clone()])),
            said(
                "wired to the Den point, and Wi-Fi to the Hall point, on 5 GHz",
                "941 Mb/s of room; a strong signal, with 183 Mb/s of room"
            ),
            "each place's figures stay with it, in the order the places are said"
        );
        assert_eq!(
            at(json!([wifi])),
            said(
                "Wi-Fi to the Hall point, on 5 GHz",
                "a strong signal, with 183 Mb/s of room"
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
        let read = json!({"code": {"repositories": 12, "checkouts": 14, "trees": 3,
                                   "behind": 1, "changes": 0}});
        assert_eq!(
            code(&read, "kestrel", true),
            said(
                "14 checkouts of 12 repositories",
                "3 trees; 1 checkout behind its upstream; no open change"
            )
        );
        let more = json!({"code": {"repositories": 1, "checkouts": 1, "trees": 0,
                                   "behind": 2, "changes": 1}});
        assert_eq!(
            code(&more, "kestrel", true),
            said(
                "1 checkout of 1 repository",
                "no tree; 2 checkouts behind their upstreams; 1 open change"
            )
        );
        assert_eq!(
            code(&json!({"code": null}), "osprey", false),
            said("not read from here", "its code is read on osprey itself")
        );
        assert_eq!(
            code(&json!({"code": null}), "kestrel", true),
            said("not read", "repos couldn't be had"),
            "on the machine it's read on, an unread code is a read that failed"
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
        let unsaid = json!({"figures": {"disk_free": 2_000_000_000_000_u64, "charge": 40}});
        assert_eq!(
            itself(&unsaid, now),
            said("2 TB free", "40% charged"),
            "a charge whose node doesn't say its plug says no plug"
        );
        let ahead = json!({"figures": {"disk_free": 2_000_000_000_000_u64,
                                       "updated_at": "2025-11-17T23:25:18Z",
                                       "update_step": "done"}});
        assert_eq!(
            itself(&ahead, now),
            said("2 TB free", "updated"),
            "an update stamped ahead of this clock says no age rather than a false one"
        );
    }

    #[test]
    fn a_moment_is_read_back_from_the_kits_own_times_and_no_other_form() {
        for seconds in [
            0,
            951_782_400,   // 2000-02-29, a leap day of a century leap year.
            1_709_251_199, // 2024-02-29T23:59:59Z.
            1_735_689_599, // 2024-12-31T23:59:59Z.
            1_763_420_718,
            4_102_444_800, // 2100-01-01, after a century that isn't a leap year.
        ] {
            let at = terminal_kit::time::rfc3339(seconds);
            assert_eq!(moment(&at), Some(seconds), "{at}");
        }
        for odd in [
            "2025-11-17T22:45:18.5Z",
            "2025-11-17T22:45:18+00:00",
            "2025-13-17T22:45:18Z",
            "2025-02-30T22:45:18Z",
            "2025-11-17T24:45:18Z",
            "soon",
        ] {
            assert_eq!(moment(odd), None, "{odd}");
        }
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
