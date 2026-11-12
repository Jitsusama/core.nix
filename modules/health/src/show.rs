//! What `health` prints: the day in one line a kind, worst first, and one
//! kind's or one thing's lines when it's asked for by name. Written for
//! Joel to read at a glance, so counts are said in words and the worst
//! thing of each kind is named.

use std::fmt::Write as _;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::agents::{self, Died};
use crate::day::{Agents, Badness, Boot, Day, Kind, Thing};
use crate::feel;
use crate::memory::{self, Memory};

/// The kinds, in the order they're shown when as bad as each other.
pub const KINDS: [Kind; 6] = [
    Kind::Words,
    Kind::Programs,
    Kind::Services,
    Kind::Devices,
    Kind::Security,
    Kind::Tools,
];

/// How many of a kind's things, or a thing's signatures, are shown.
const SHOWN: usize = 10;

/// How many of the day's most painful interactions open `health`.
const PAINS: usize = 3;

/// The day, a line a kind, the worst kind first.
#[must_use]
pub fn day(day: &Day) -> String {
    let mut out = format!("health, {}{}\n", day.date, until(day.to));
    for pain in day.feel.iter().take(PAINS) {
        let _ = writeln!(
            out,
            "feel      {}, {}",
            clock(pain.at * 1_000_000),
            feel::said(pain)
        );
    }
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
    for boot in boots_of(day) {
        if let Some(took) = boot.shutdown() {
            let at = clock(boot.stopping.unwrap_or_default());
            let _ = writeln!(out, "shutdown  {}, asked at {at}", seconds(took));
        }
        if let Some(took) = boot.to_ready() {
            let _ = writeln!(
                out,
                "boot      {} to a ready launcher, at {}; your password's {} aside",
                seconds(took),
                clock(boot.started),
                seconds(boot.password())
            );
        }
    }
    if let Some(line) = the_dead(&day.agents) {
        let _ = writeln!(out, "agents    {line}");
    }
    for (name, held) in &day.memory {
        if held.most_kib > 0 {
            let _ = writeln!(
                out,
                "memory    {name}: {}; largest {}, oldest {}; at most {} at {}",
                holding(held),
                memory::size(held.largest_kib),
                memory::age(held.oldest),
                memory::size(held.most_kib),
                clock(held.most_at)
            );
        }
    }
    out
}

/// The agents gone without parking, how many mid-turn and how long the
/// oldest has been gone, then those that died in the day and the most in
/// any ten minutes, which is when dying has become a pattern.
fn the_dead(agents: &Agents) -> Option<String> {
    if agents.gone.is_empty() && agents.died.is_empty() {
        return None;
    }
    let mid_turn = agents
        .gone
        .values()
        .filter(|died| died.state == "busy")
        .count();
    let oldest = agents
        .gone
        .values()
        .map(|died| died.at)
        .min()
        .unwrap_or(agents.looked);
    let times: Vec<i64> = agents.died.values().map(|died| died.at).collect();
    let gone = u64::try_from(agents.gone.len()).unwrap_or(u64::MAX);
    let age = u64::try_from((agents.looked - oldest) / 1_000_000).unwrap_or(0);
    Some(format!(
        "{} gone without parking ({mid_turn} mid-turn), oldest {}; {} died today, \
         at most {} in ten minutes",
        grouped(gone),
        memory::age(age),
        agents.died.len(),
        agents::most_in_ten_minutes(&times)
    ))
}

/// Each agent that died in the day, and each still gone from before it,
/// in the order they died.
fn the_dead_by_name(day: &Day) -> String {
    let mut out = format!(
        "agents gone without parking, {}{}\n",
        day.date,
        until(day.to)
    );
    let mut dead: Vec<(&String, &Died)> = day.agents.died.iter().collect();
    let earlier = day
        .agents
        .gone
        .iter()
        .filter(|(session, _)| !day.agents.died.contains_key(*session));
    dead.extend(earlier);
    dead.sort_by_key(|(_, died)| died.at);
    for (session, died) in dead {
        let _ = writeln!(
            out,
            "{} {:<9} {:<20} {} ({session})",
            when(died.at, &day.date),
            died.state,
            died.quest,
            died.title
        );
    }
    out
}

/// A moment's time, and its day when it isn't `date`.
fn when(at: i64, date: &str) -> String {
    Timestamp::from_microsecond(at).map_or_else(
        |_| "--:--".to_owned(),
        |at| {
            let zoned = at.to_zoned(TimeZone::system());
            let day = zoned.strftime("%Y-%m-%d").to_string();
            if day == date {
                zoned.strftime("      %H:%M").to_string()
            } else {
                zoned.strftime("%b %d %H:%M").to_string()
            }
        },
    )
}

/// What a watched program's copies held at the last look.
fn holding(held: &Memory) -> String {
    format!(
        "{} hold {}, {} in swap",
        held.copies,
        memory::size(held.kib + held.swap_kib),
        memory::size(held.swap_kib)
    )
}

/// Each watched program's copies by age, what each age holds together
/// and each copy on average, so a program that grows as it ages shows.
fn memory_by_age(day: &Day) -> String {
    let mut out = format!("memory on {}{}\n", day.date, until(day.to));
    for (name, held) in day.memory.iter().filter(|(_, held)| held.most_kib > 0) {
        let _ = writeln!(out, "{name}: {}", holding(held));
        for (group, aged) in held.by_age.iter().enumerate() {
            if aged.copies > 0 {
                let _ = writeln!(
                    out,
                    "  {:<12} {:>3} hold {}, {} each",
                    memory::aged(group),
                    aged.copies,
                    memory::size(aged.kib),
                    memory::size(aged.kib / aged.copies)
                );
            }
        }
    }
    out
}

/// What was asked for: a kind's things worst first, or the things whose
/// name holds the words, each with what it kept saying.
#[must_use]
pub fn asked(day: &Day, words: &str) -> String {
    if words == "boot" {
        return boot_stages(day);
    }
    if words == "memory" {
        return memory_by_age(day);
    }
    // Not `agents`, which is a daemon's name, whose lines that asks for.
    if words == "died" {
        return the_dead_by_name(day);
    }
    if let Some(&kind) = KINDS.iter().find(|kind| kind.name() == words) {
        let mut out = format!("{} on {}{}\n", kind.name(), day.date, until(day.to));
        for (name, thing) in worst_of(day, kind).into_iter().take(SHOWN) {
            let _ = writeln!(out, "{name:<24} {}", counted(&thing.badness(), 0));
        }
        return out;
    }
    let mut things: Vec<(&String, &Thing)> = day
        .things
        .iter()
        .filter(|(name, _)| name.contains(words))
        .collect();
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

/// The day's boots, in the order they happened.
fn boots_of(day: &Day) -> Vec<&Boot> {
    let mut boots: Vec<&Boot> = day.boots.values().collect();
    boots.sort_by_key(|boot| boot.stopping.unwrap_or(boot.started));
    boots
}

/// Each boot's stages as systemd timed them, and its shutdown.
fn boot_stages(day: &Day) -> String {
    let mut out = format!("boots on {}\n", day.date);
    let boots = boots_of(day);
    if boots.is_empty() {
        out.push_str("the machine neither started nor stopped\n");
    }
    for boot in boots {
        if let Some(took) = boot.shutdown() {
            let at = clock(boot.stopping.unwrap_or_default());
            let _ = writeln!(out, "shut down at {at} in {}", seconds(took));
        }
        if boot.started > 0 {
            let password = boot.password();
            let _ = writeln!(
                out,
                "started at {}: firmware {}, loader {}, kernel {}, initrd {} with your \
                 password's {} aside, services {}",
                clock(boot.started),
                seconds(boot.firmware),
                seconds(boot.loader),
                seconds(boot.kernel),
                seconds(boot.initrd - password),
                seconds(password),
                seconds(boot.userspace)
            );
        }
        if let Some(took) = boot.to_ready() {
            let _ = writeln!(out, "  {} to a ready launcher", seconds(took));
        }
    }
    out
}

/// Microseconds as seconds to a tenth: "27.0 s".
fn seconds(micros: i64) -> String {
    let tenths = (micros + 50_000) / 100_000;
    format!("{}.{} s", tenths / 10, tenths % 10)
}

/// A kind's things, worst first.
fn worst_of(day: &Day, kind: Kind) -> Vec<(&String, &Thing)> {
    let mut things: Vec<(&String, &Thing)> = day
        .things
        .iter()
        .filter(|(_, thing)| thing.kind == Some(kind))
        .collect();
    things.sort_by_key(|(_, thing)| std::cmp::Reverse(thing.badness()));
    things
}

/// The badness of things together.
fn total(things: &[(&String, &Thing)]) -> Badness {
    things
        .iter()
        .fold(Badness::default(), |sum, (_, thing)| sum + thing.badness())
}

/// Badness in words, gravest first, leaving out what's none: "3 crashes
/// (1 yours), 12 errors, 1,470 warnings from 21".
fn counted(bad: &Badness, things: usize) -> String {
    let Badness {
        his_crashes: his,
        crashes,
        traces,
        errors,
        restarts,
        lines,
        events,
    } = *bad;
    let mut parts = Vec::new();
    if crashes > 0 {
        let yours = if his > 0 {
            format!(" ({his} yours)")
        } else {
            String::new()
        };
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
    if events > 0 {
        parts.push(plural(events, "event", "events"));
    }
    let from = if things > 1 {
        format!(" from {things}")
    } else {
        String::new()
    };
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
        if place > 0 && (digits.len() - place).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// How far into the day it was read, ", to 22:41", or nothing for a day
/// with no entry.
fn until(to: i64) -> String {
    if to == 0 {
        String::new()
    } else {
        format!(", to {}", clock(to))
    }
}

/// A time of day on the machine's clock, from microseconds since the epoch.
fn clock(at: i64) -> String {
    Timestamp::from_microsecond(at).map_or_else(
        |_| "--:--".to_owned(),
        |at| {
            at.to_zoned(TimeZone::system())
                .strftime("%H:%M")
                .to_string()
        },
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
        day.things
            .insert("pebble-lights".into(), thing(Kind::Services, 3306, 0, 0, 0));
        day.things
            .insert("people".into(), thing(Kind::Services, 118, 4, 0, 0));
        day.things
            .insert("chrome".into(), thing(Kind::Programs, 28, 28, 28, 1));
        day.things
            .insert("iwlwifi".into(), thing(Kind::Devices, 12, 12, 0, 0));
        day.things.insert(
            "usb".into(),
            Thing {
                events: 22,
                ..thing(Kind::Devices, 0, 0, 0, 0)
            },
        );
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
            lines[2], "devices   12 errors, 22 events from 2; worst: iwlwifi, 12 errors",
            "errors come before warnings, and a device's events last"
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
        assert_eq!(
            day(&Day::new("2026-11-12")),
            "health, 2026-11-12\nnothing said a warning\n"
        );
    }

    #[test]
    fn a_kind_asked_for_lists_its_things() {
        let shown = asked(&a_day(), "services");
        assert!(shown.starts_with("services on 2026-11-12\n"), "{shown}");
        let people = shown.find("people").expect("people is listed");
        let lights = shown
            .find("pebble-lights")
            .expect("pebble-lights is listed");
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
        assert!(
            shown.starts_with("people: 4 errors, 114 warnings\n"),
            "{shown}"
        );
        assert!(shown.contains("     4  "), "its count: {shown}");
        assert!(
            shown.contains("lookup 12 failed"),
            "an example as said: {shown}"
        );
        assert_eq!(
            asked(&day, "nobody"),
            "nothing called nobody said a warning on 2026-11-12\n",
            "a name nothing has says so"
        );
    }

    #[test]
    fn the_three_worst_pains_open_the_day() {
        let mut felt = a_day();
        felt.feel = (1..=4)
            .map(|rank| feel::Pain {
                trying: format!("program:p{rank}"),
                seconds: 9.0,
                rank,
                ..feel::Pain::default()
            })
            .collect();
        let shown = day(&felt);
        let lines: Vec<&str> = shown.lines().collect();
        assert!(lines[1].starts_with("feel      "), "{shown}");
        assert!(lines[1].ends_with(", finding P1 took 9 s"), "{shown}");
        assert!(lines[3].ends_with(", finding P3 took 9 s"), "{shown}");
        assert!(!shown.contains("P4"), "three at most: {shown}");
        assert!(!lines[4].starts_with("feel"), "then the kinds: {shown}");
    }

    #[test]
    fn the_dead_agents_have_a_line_and_are_named() {
        let mut mourned = a_day();
        let died = |state: &str, at: i64| Died {
            state: state.into(),
            at,
            quest: "QEST-1".into(),
            title: "π - Git upkeep".into(),
        };
        let hour = 3_600_000_000;
        mourned.agents.looked = 30 * hour;
        mourned
            .agents
            .gone
            .insert("a".into(), died("busy", 8 * hour));
        mourned
            .agents
            .gone
            .insert("b".into(), died("listening", 29 * hour));
        mourned
            .agents
            .died
            .insert("b".into(), died("listening", 29 * hour));
        let shown = day(&mourned);
        assert!(
            shown.contains(
                "\nagents    2 gone without parking (1 mid-turn), oldest 22 h; 1 died today, \
                 at most 1 in ten minutes\n"
            ),
            "{shown}"
        );
        let named = asked(&mourned, "died");
        let lines: Vec<&str> = named.lines().collect();
        assert_eq!(lines.len(), 3, "each once, the earlier gone first: {named}");
        assert!(lines[1].contains(" busy      QEST-1"), "{named}");
        assert!(lines[2].ends_with("π - Git upkeep (b)"), "{named}");
        assert!(!day(&a_day()).contains("agents"), "none died");
    }

    #[test]
    fn each_watched_programs_memory_has_a_line_and_its_ages() {
        let mut minded = a_day();
        let copy = |kib, age| memory::Held {
            kib,
            swap_kib: 1 << 20,
            age,
        };
        let mut pi = Memory::default();
        pi.saw(0, &[copy(3 << 20, 600), copy(1 << 20, 79_200)]);
        minded.memory.insert("pi".into(), pi);
        minded.memory.insert("quickshell".into(), Memory::default());
        let shown = day(&minded);
        let line = shown
            .lines()
            .find(|line| line.starts_with("memory    pi: "))
            .unwrap_or_else(|| panic!("a line for pi: {shown}"));
        assert!(
            line.starts_with(
                "memory    pi: 2 hold 6.0 GiB, 2.0 GiB in swap; largest 4.0 GiB, oldest 22 h; \
                 at most 6.0 GiB at "
            ),
            "{shown}"
        );
        assert!(
            !shown.contains("quickshell"),
            "not running all day: {shown}"
        );
        let aged = asked(&minded, "memory");
        assert!(!aged.contains("quickshell"), "not running all day: {aged}");
        assert!(
            aged.contains("\n  under 1 h      1 hold 4.0 GiB, 4.0 GiB each\n"),
            "{aged}"
        );
        assert!(
            aged.contains("\n  6 h to 24 h    1 hold 2.0 GiB, 2.0 GiB each\n"),
            "{aged}"
        );
    }

    #[test]
    fn a_boot_and_a_shutdown_each_have_a_line() {
        let mut booted = a_day();
        booted.boots.insert(
            "b".into(),
            Boot {
                started: 1,
                firmware: 9_700_000,
                loader: 500_000,
                kernel: 400_000,
                initrd: 24_400_000,
                userspace: 20_700_000,
                asked: Some(2_900_000),
                opened: Some(23_900_000),
                ready: Some(37_700_000),
                stopping: Some(1),
                ended: Some(30_000_001),
            },
        );
        let shown = day(&booted);
        assert!(shown.contains("\nshutdown  30.0 s, asked at "), "{shown}");
        assert!(
            shown.contains("\nboot      26.9 s to a ready launcher, at "),
            "{shown}"
        );
        assert!(
            shown.ends_with("; your password's 21.0 s aside\n"),
            "{shown}"
        );
        let stages = asked(&booted, "boot");
        assert!(
            stages.contains("initrd 3.4 s with your password's 21.0 s aside, services 20.7 s"),
            "{stages}"
        );
        assert_eq!(
            asked(&a_day(), "boot"),
            "boots on 2026-11-12\nthe machine neither started nor stopped\n",
            "a day without one says so"
        );
    }

    #[test]
    fn counts_are_grouped_by_thousands() {
        assert_eq!(grouped(18_216), "18,216", "thousands");
        assert_eq!(grouped(999), "999", "no group under a thousand");
        assert_eq!(grouped(1_000_000), "1,000,000", "millions");
    }
}
