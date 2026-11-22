//! What `health` prints: the day in one line a kind, worst first, and one
//! kind's or one thing's lines when it's asked for by name. Written for
//! Joel to read at a glance, so counts are said in words and the worst
//! thing of each kind is named. What's asked for by name is `asked`'s.

mod asked;

use std::fmt::Write as _;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use terminal_kit::Pen;
use terminal_kit::layout::{self, Room};

pub use asked::asked;

use crate::agents::{self, Agents};
use crate::day::{Badness, Boot, Day, Kind, Thing};
use crate::disk;
use crate::feel;
use crate::memory::{self, Memory};
use crate::typing;

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
pub(super) const SHOWN: usize = 10;

/// How many of the day's most painful interactions open `health`.
const PAINS: usize = 3;

/// The day, a line a kind, the worst kind first. Each line is a fact, its
/// label muted beside it, and one too long for the room goes on under its
/// own words; into a pipe each stays one line.
#[must_use]
pub fn day(day: &Day, pen: &Pen, room: Room) -> String {
    let mut out = format!("health, {}{}\n", day.date, until(day.to));
    for line in layout::facts(&as_facts(&said_of(day)), "", pen, room) {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// The facts as `layout::facts` takes them, every one known.
fn as_facts<'a>(said: &'a [(&'static str, String)]) -> Vec<(&'static str, Option<&'a str>)> {
    said.iter()
        .map(|(label, line)| (*label, Some(line.as_str())))
        .collect()
}

/// What the day says, as labelled lines, in the order they're shown.
fn said_of(day: &Day) -> Vec<(&'static str, String)> {
    let mut said = Vec::new();
    for pain in day.feel.iter().take(PAINS) {
        said.push((
            "feel",
            format!("{}, {}", clock(pain.at * 1_000_000), feel::said(pain)),
        ));
    }
    if let Some(line) = day.launcher.summed() {
        said.push(("feel", format!("the launcher: {line}")));
    }
    if let Some(line) = typing::said(&day.typing) {
        said.push(("feel", line));
    }
    let mut kinds: Vec<(Kind, Vec<(&String, &Thing)>)> = KINDS
        .iter()
        .map(|&kind| (kind, worst_of(day, kind)))
        .filter(|(_, things)| !things.is_empty())
        .collect();
    kinds.sort_by_key(|(_, things)| std::cmp::Reverse(total(things)));
    if kinds.is_empty() {
        said.push(("warnings", "none".to_owned()));
    }
    for (kind, things) in kinds {
        let (name, worst) = things[0];
        said.push((
            kind.name(),
            format!(
                "{}; worst: {name}, {}",
                counted(&total(&things), things.len()),
                counted(&worst.badness(), 0)
            ),
        ));
    }
    for boot in boots_of(day) {
        if let Some(took) = boot.shutdown() {
            let at = clock(boot.stopping.unwrap_or_default());
            said.push(("shutdown", format!("{}, asked at {at}", seconds(took))));
        }
        if let Some(took) = boot.to_ready() {
            said.push((
                "boot",
                format!(
                    "{} to a ready launcher, at {}; your password's {} aside",
                    seconds(took),
                    clock(boot.started),
                    seconds(boot.password())
                ),
            ));
        }
    }
    let lines = [
        ("agents", the_dead(&day.agents)),
        ("asks", day.asks.said()),
        ("launches", day.launches.said()),
        ("disks", disk::said(&day.disks)),
    ];
    said.extend(
        lines
            .into_iter()
            .filter_map(|(label, line)| line.map(|line| (label, line))),
    );
    for (name, held) in &day.memory {
        if held.most_kib > 0 {
            said.push((
                "memory",
                format!(
                    "{name}: {}; largest {}, oldest {}; {}at most {} at {}",
                    holding(held),
                    memory::size(held.largest_kib),
                    memory::age(held.oldest),
                    typical(held),
                    memory::size(held.most_kib),
                    clock(held.most_at)
                ),
            ));
        }
    }
    said
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

/// What a watched program's copies held at the last look.
/// A typical and a heavy copy at the last look, as cadence holds them,
/// when any was running then.
fn typical(held: &Memory) -> String {
    if held.copies == 0 {
        return String::new();
    }
    format!(
        "typical {}, heavy {}; ",
        memory::size(held.median_kib),
        memory::size(held.p95_kib)
    )
}

pub(super) fn holding(held: &Memory) -> String {
    format!(
        "{} hold {}, {} in swap",
        held.copies,
        memory::size(held.kib + held.swap_kib),
        memory::size(held.swap_kib)
    )
}

/// The day's boots, in the order they happened.
fn boots_of(day: &Day) -> Vec<&Boot> {
    let mut boots: Vec<&Boot> = day.boots.values().collect();
    boots.sort_by_key(|boot| boot.stopping.unwrap_or(boot.started));
    boots
}

/// Each boot's stages as systemd timed them, and its shutdown.
pub(super) fn boot_stages(day: &Day) -> String {
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
pub(super) fn worst_of(day: &Day, kind: Kind) -> Vec<(&String, &Thing)> {
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
/// (1 yours), 12 errors (7 agents'), 1,470 warnings from 21".
pub(super) fn counted(bad: &Badness, things: usize) -> String {
    let Badness {
        his_crashes: his,
        crashes,
        traces,
        errors,
        restarts,
        lines,
        events,
        agents_errors: agents,
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
    // Agents' errors are out of both errors and lines, so what's left of
    // the lines past his errors is warnings.
    let warnings = lines.saturating_sub(errors);
    // A crash is said at an error's priority, so it's counted once, as a
    // crash.
    let errors = errors + agents;
    let other_errors = errors.saturating_sub(crashes);
    if other_errors > 0 {
        let agents = if agents > 0 {
            format!(" ({} agents')", grouped(agents))
        } else {
            String::new()
        };
        parts.push(format!(
            "{}{agents}",
            plural(other_errors, "error", "errors")
        ));
    }
    if restarts > 0 {
        parts.push(plural(restarts, "restart", "restarts"));
    }
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
pub(super) fn grouped(count: u64) -> String {
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
pub(super) fn until(to: i64) -> String {
    if to == 0 {
        String::new()
    } else {
        format!(", to {}", clock(to))
    }
}

/// A time of day on the machine's clock, from microseconds since the epoch.
pub(super) fn clock(at: i64) -> String {
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
    use terminal_kit::Pen;
    use terminal_kit::layout::{self, Room};

    use super::*;
    use crate::agents::Died;
    use crate::day::Signature;

    /// What was asked for, uninked, in a terminal 100 cells wide.
    fn asked(day: &Day, words: &str) -> String {
        super::asked(day, words, &Pen::plain(), Room::Cells(100))
    }

    /// The day, uninked, into a pipe, where no line is wrapped.
    fn day(day: &Day) -> String {
        super::day(day, &Pen::plain(), Room::Unbounded)
    }

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
    fn errors_say_how_many_were_agents() {
        let tec = Thing {
            agents_errors: 1_200,
            agents_lines: 1_200,
            ..thing(Kind::Programs, 2_000, 1_205, 0, 0)
        };
        assert_eq!(
            counted(&tec.badness(), 0),
            "1,205 errors (1,200 agents'), 795 warnings",
            "what agents' commands said is told apart, as Joel's crashes are"
        );
    }

    #[test]
    fn agents_errors_rank_below_his() {
        let mut day = Day::new("2026-11-16");
        day.things.insert(
            "tec".into(),
            Thing {
                agents_errors: 60,
                agents_lines: 90,
                ..thing(Kind::Programs, 90, 60, 0, 0)
            },
        );
        day.things
            .insert("kitty".into(), thing(Kind::Programs, 5, 0, 0, 0));
        let worst: Vec<&str> = worst_of(&day, Kind::Programs)
            .into_iter()
            .map(|(name, _)| name.as_str())
            .collect();
        assert_eq!(
            worst,
            ["kitty", "tec"],
            "five warnings of his are worse than ninety lines of agents'"
        );
    }

    #[test]
    fn the_day_fits_its_width_and_each_line_hangs_under_its_words() {
        let mut felt = a_day();
        felt.launcher.took(
            "launcher: 4 drawers in home presented a median 42 ms from the key, the slowest \
             134 ms, the p99 134 ms, 3 within 50 ms",
        );
        let narrow = super::day(&felt, &Pen::plain(), Room::Cells(60));
        let lines: Vec<&str> = narrow.lines().collect();
        assert!(
            lines.iter().all(|line| layout::width(line) <= 60),
            "a line passes 60 cells: {narrow}"
        );
        let launcher = lines
            .iter()
            .position(|line| line.starts_with("feel      the launcher: "))
            .unwrap_or_else(|| panic!("the launcher has a line: {narrow}"));
        assert!(
            lines[launcher + 1].starts_with("          ")
                && !lines[launcher + 1][10..].starts_with(' '),
            "the launcher's words don't go on under their own: {narrow}"
        );
        assert_eq!(
            lines[0], "health, 2026-11-12",
            "the day's head isn't one of its facts: {narrow}"
        );
    }

    #[test]
    fn an_empty_day_says_so() {
        assert_eq!(
            day(&Day::new("2026-11-12")),
            "health, 2026-11-12\nwarnings  none\n"
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
        assert!(
            shown.contains("\nCOUNT  FIRST  LAST   EXAMPLE\n    4  "),
            "its count, under its heading: {shown}"
        );
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
    fn the_launcher_and_typing_follow_the_pains_and_each_figure_says_its_side() {
        let mut felt = a_day();
        felt.launcher.took(
            "launcher: 4 drawers in home presented a median 42 ms from the key, the slowest \
             134 ms, the p99 134 ms, 3 within 50 ms",
        );
        felt.typing = vec![crate::typing::Sample {
            keys: 400,
            p99_ms: 21.0,
            ..crate::typing::Sample::default()
        }];
        let shown = day(&felt);
        let lines: Vec<&str> = shown.lines().collect();
        assert_eq!(
            lines[1..3],
            [
                "feel      the launcher: 1 of 4 timed past their budgets; worst drawers, 1 of 4 \
                 past 50 ms, the slowest 134 ms",
                "feel      typing in pi, modelled: 400 keys in 1 sample, the worst p99 21 ms",
            ],
            "the launcher or typing in pi doesn't follow the pains: {shown}"
        );
        let figures = asked(&felt, "figures");
        let his = figures
            .lines()
            .position(|line| line.contains("launcher.drawers.p99-ms"));
        let ours = figures
            .lines()
            .position(|line| line.contains("typing.worst-p99-ms"));
        assert!(
            his < ours
                && figures.contains("his   launcher.drawers.p99-ms")
                && figures.contains("ours  typing.worst-p99-ms"),
            "a figure doesn't say its side, or Joel's don't come first: {figures}"
        );
        assert!(
            asked(&felt, "launcher").contains("4 drawers: a median 42 ms"),
            "the launcher asked for doesn't say each part"
        );
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
        assert_eq!(lines.len(), 5, "each once, the earlier gone first: {named}");
        assert!(lines[3].contains("  busy       QEST-1  a  "), "{named}");
        assert!(lines[4].ends_with("  b        π - Git upkeep"), "{named}");
        assert!(!day(&a_day()).contains("agents"), "none died");
        let narrow = super::asked(&mourned, "died", &Pen::plain(), Room::Cells(40));
        assert!(
            narrow.lines().all(|line| layout::width(line) <= 40),
            "a line passes 40 cells: {narrow}"
        );
        assert!(
            narrow.contains("busy") && !narrow.contains("SESSION"),
            "the session doesn't give way first: {narrow}"
        );
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
                 typical 2.0 GiB, heavy 4.0 GiB; at most 6.0 GiB at "
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
            aged.contains("\nunder 1 h         1  4.0 GiB  4.0 GiB\n"),
            "{aged}"
        );
        assert!(
            aged.contains("\n6 h to 24 h       1  2.0 GiB  2.0 GiB\n"),
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
    fn an_empty_view_says_what_it_found_none_of() {
        let quiet = Day::new("2026-11-12");
        for (words, none) in [
            ("services", "nothing said a warning"),
            ("died", "no agent went without parking"),
            ("launches", "nothing timed"),
        ] {
            let shown = asked(&quiet, words);
            assert_eq!(
                shown.lines().nth(1),
                Some(none),
                "{words} on a quiet day doesn't say it found none: {shown}"
            );
        }
        let figures = asked(&quiet, "figures");
        assert!(
            !figures.contains("-0"),
            "a figure of nothing reads as negative: {figures}"
        );
    }

    #[test]
    fn counts_are_grouped_by_thousands() {
        assert_eq!(grouped(18_216), "18,216", "thousands");
        assert_eq!(grouped(999), "999", "no group under a thousand");
        assert_eq!(grouped(1_000_000), "1,000,000", "millions");
    }
}
