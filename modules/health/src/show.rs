//! What `health` prints: the day in one line a kind, worst first, and one
//! kind's or one thing's lines when it's asked for by name. Written for
//! Joel to read at a glance, so counts are said in words and the worst
//! thing of each kind is named.

use std::fmt::Write as _;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::day::{Day, Kind, Thing};

/// The kinds, in the order they're shown when as bad as each other.
const KINDS: [Kind; 4] = [Kind::Programs, Kind::Services, Kind::Devices, Kind::Security];

/// How many of a kind's things, or a thing's signatures, are shown.
const SHOWN: usize = 10;

/// The day, a line a kind, the worst kind first.
#[must_use]
pub fn day(day: &Day) -> String {
    let mut out = format!("health, {}{}\n", day.date, until(day.to));
    let mut kinds: Vec<(Kind, Vec<(&String, &Thing)>)> = KINDS
        .iter()
        .map(|&kind| (kind, worst_of(day, kind)))
        .filter(|(_, things)| !things.is_empty())
        .collect();
    kinds.sort_by_key(|(_, things)| std::cmp::Reverse(total(things)));
    if kinds.is_empty() {
        out.push_str("nothing said a warning\n");
    }
    for (kind, things) in kinds {
        let (name, worst) = things[0];
        let _ = writeln!(
            out,
            "{:<9} {}; worst: {name}, {}",
            kind.name(),
            counted(&total(&things), things.len()),
            counted(&worst.badness(), 0)
        );
    }
    out
}

/// What was asked for: a kind's things worst first, or the things whose
/// name holds the words, each with what it kept saying.
#[must_use]
pub fn asked(day: &Day, words: &str) -> String {
    if let Some(&kind) = KINDS.iter().find(|kind| kind.name() == words) {
        let mut out = format!("{} on {}{}\n", kind.name(), day.date, until(day.to));
        for (name, thing) in worst_of(day, kind).into_iter().take(SHOWN) {
            let _ = writeln!(out, "{name:<24} {}", counted(&thing.badness(), 0));
        }
        return out;
    }
    let mut things: Vec<(&String, &Thing)> =
        day.things.iter().filter(|(name, _)| name.contains(words)).collect();
    if things.is_empty() {
        return format!("nothing called {words} said a warning on {}\n", day.date);
    }
    things.sort_by_key(|(_, thing)| std::cmp::Reverse(thing.badness()));
    let mut out = String::new();
    for (name, thing) in things.into_iter().take(SHOWN) {
        let _ = writeln!(out, "{name}: {}", counted(&thing.badness(), 0));
        let mut said: Vec<_> = thing.signatures.values().collect();
        said.sort_by_key(|signature| (signature.priority, std::cmp::Reverse(signature.count)));
        for signature in said.into_iter().take(SHOWN) {
            let _ = writeln!(
                out,
                "  {:>6}  {} to {}  {}",
                grouped(signature.count),
                clock(signature.first),
                clock(signature.last),
                signature.example
            );
        }
    }
    out
}

/// A kind's things, worst first.
fn worst_of(day: &Day, kind: Kind) -> Vec<(&String, &Thing)> {
    let mut things: Vec<(&String, &Thing)> =
        day.things.iter().filter(|(_, thing)| thing.kind == Some(kind)).collect();
    things.sort_by_key(|(_, thing)| std::cmp::Reverse(thing.badness()));
    things
}

/// The badness of things together.
fn total(things: &[(&String, &Thing)]) -> (u64, u64, u64, u64, u64, u64) {
    things.iter().fold((0, 0, 0, 0, 0, 0), |sum, (_, thing)| {
        let bad = thing.badness();
        (
            sum.0 + bad.0,
            sum.1 + bad.1,
            sum.2 + bad.2,
            sum.3 + bad.3,
            sum.4 + bad.4,
            sum.5 + bad.5,
        )
    })
}

/// Badness in words, gravest first, leaving out what's none: "3 crashes
/// (1 yours), 12 errors, 1,470 warnings from 21".
fn counted(bad: &(u64, u64, u64, u64, u64, u64), things: usize) -> String {
    let (his, crashes, traces, errors, restarts, lines) = *bad;
    let mut parts = Vec::new();
    if crashes > 0 {
        let yours = if his > 0 { format!(" ({his} yours)") } else { String::new() };
        parts.push(format!("{}{yours}", plural(crashes, "crash", "crashes")));
    }
    if traces > 0 {
        parts.push(plural(traces, "trace", "traces"));
    }
    // A crash is said at an error's priority, so it's counted once, as a
    // crash.
    let other_errors = errors.saturating_sub(crashes);
    if other_errors > 0 {
        parts.push(plural(other_errors, "error", "errors"));
    }
    if restarts > 0 {
        parts.push(plural(restarts, "restart", "restarts"));
    }
    let warnings = lines.saturating_sub(errors);
    if warnings > 0 {
        parts.push(plural(warnings, "warning", "warnings"));
    }
    let from = if things > 1 { format!(" from {things}") } else { String::new() };
    format!("{}{from}", parts.join(", "))
}

/// A count and its noun, as one or many.
fn plural(count: u64, one: &str, many: &str) -> String {
    format!("{} {}", grouped(count), if count == 1 { one } else { many })
}

/// A count with its thousands set apart: 18,216.
fn grouped(count: u64) -> String {
    let digits = count.to_string();
    let mut out = String::new();
    for (place, digit) in digits.chars().enumerate() {
        if place > 0 && (digits.len() - place) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// How far into the day it was read, ", to 22:41", or nothing for a day
/// with no entry.
fn until(to: i64) -> String {
    if to == 0 { String::new() } else { format!(", to {}", clock(to)) }
}

/// A time of day on the machine's clock, from microseconds since the epoch.
fn clock(at: i64) -> String {
    Timestamp::from_microsecond(at).map_or_else(
        |_| "--:--".to_owned(),
        |at| at.to_zoned(TimeZone::system()).strftime("%H:%M").to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::day::Signature;

    fn thing(kind: Kind, lines: u64, errors: u64, crashes: u64, his: u64) -> Thing {
        Thing {
            kind: Some(kind),
            lines,
            errors,
            crashes,
            his_crashes: his,
            ..Thing::default()
        }
    }

    fn a_day() -> Day {
        let mut day = Day::new("2026-11-12");
        day.things.insert("pebble-lights".into(), thing(Kind::Services, 3306, 0, 0, 0));
        day.things.insert("people".into(), thing(Kind::Services, 118, 4, 0, 0));
        day.things.insert("chrome".into(), thing(Kind::Programs, 28, 28, 28, 1));
        day.things.insert("iwlwifi".into(), thing(Kind::Devices, 12, 12, 0, 0));
        day
    }

    #[test]
    fn the_day_is_a_line_a_kind_the_worst_first() {
        let shown = day(&a_day());
        let lines: Vec<&str> = shown.lines().collect();
        assert_eq!(lines[0], "health, 2026-11-12", "it opens with the day");
        assert_eq!(
            lines[1], "programs  28 crashes (1 yours); worst: chrome, 28 crashes (1 yours)",
            "a crash is worse than any number of warnings"
        );
        assert_eq!(
            lines[2],
            "devices   12 errors; worst: iwlwifi, 12 errors",
            "errors come before warnings"
        );
        assert_eq!(
            lines[3],
            "services  4 errors, 3,420 warnings from 2; worst: people, 4 errors, 114 warnings",
            "a kind's worst is its gravest, not its loudest"
        );
        assert_eq!(lines.len(), 4, "a kind with nothing isn't shown");
    }

    #[test]
    fn an_empty_day_says_so() {
        assert_eq!(day(&Day::new("2026-11-12")), "health, 2026-11-12\nnothing said a warning\n");
    }

    #[test]
    fn a_kind_asked_for_lists_its_things() {
        let shown = asked(&a_day(), "services");
        assert!(shown.starts_with("services on 2026-11-12\n"), "{shown}");
        let people = shown.find("people").expect("people is listed");
        let lights = shown.find("pebble-lights").expect("pebble-lights is listed");
        assert!(people < lights, "the gravest first: {shown}");
    }

    #[test]
    fn a_thing_asked_for_says_what_it_kept_saying() {
        let mut day = a_day();
        let people = day.things.get_mut("people").expect("people is in the day");
        people.signatures.insert(
            "lookup # failed".into(),
            Signature {
                count: 4,
                priority: 3,
                example: "lookup 12 failed".into(),
                ..Signature::default()
            },
        );
        let shown = asked(&day, "peop");
        assert!(shown.starts_with("people: 4 errors, 114 warnings\n"), "{shown}");
        assert!(shown.contains("     4  "), "its count: {shown}");
        assert!(shown.contains("lookup 12 failed"), "an example as said: {shown}");
        assert_eq!(
            asked(&day, "nobody"),
            "nothing called nobody said a warning on 2026-11-12\n",
            "a name nothing has says so"
        );
    }

    #[test]
    fn counts_are_grouped_by_thousands() {
        assert_eq!(grouped(18_216), "18,216", "thousands");
        assert_eq!(grouped(999), "999", "no group under a thousand");
        assert_eq!(grouped(1_000_000), "1,000,000", "millions");
    }
}
