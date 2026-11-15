//! What `health` prints when a kind, a view or a thing is asked for by
//! name: its lists set as the terminal kit's tables, fitted to the room.
//! Apart from the day's line, since these change with the kit's tables
//! and the day's line with what Joel reads at a glance.

use std::fmt::Write as _;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use terminal_kit::Pen;
use terminal_kit::layout::{self, Align, Room};

use super::{KINDS, SHOWN, boot_stages, clock, counted, grouped, holding, until, worst_of};
use crate::agents::Died;
use crate::day::{Day, Thing};
use crate::figures::{self, Side};
use crate::memory;

/// A head and a table, set apart by an empty line; a table with no rows
/// says instead, in its view's own words, what it found none of.
fn headed(head: String, table: Vec<String>, rows: usize, none: &str) -> String {
    if rows == 0 {
        return format!("{head}\n{none}\n");
    }
    layout::joined(vec![vec![head], table])
}

/// The dead agents' columns: the session gives way first, then the
/// quest, and a long title is cut last.
const DEAD: [(&str, Align); 5] = [
    ("when", Align::Right),
    ("state", Align::Left),
    ("quest", Align::Left),
    ("session", Align::Left),
    ("title", Align::Left),
];

/// Each agent that died in the day, and each still gone from before it,
/// in the order they died.
fn the_dead_by_name(day: &Day, pen: &Pen, room: Room) -> String {
    let head = format!("agents gone without parking, {}{}", day.date, until(day.to));
    let mut dead: Vec<(&String, &Died)> = day.agents.died.iter().collect();
    let earlier = day
        .agents
        .gone
        .iter()
        .filter(|(session, _)| !day.agents.died.contains_key(*session));
    dead.extend(earlier);
    dead.sort_by_key(|(_, died)| died.at);
    let rows: Vec<Vec<String>> = dead
        .into_iter()
        .map(|(session, died)| {
            vec![
                when(died.at, &day.date),
                died.state.clone(),
                died.quest.clone(),
                session.clone(),
                died.title.clone(),
            ]
        })
        .collect();
    let table = layout::table_in(&DEAD, &rows, pen, room, &[3, 2]);
    headed(head, table, rows.len(), "no agent went without parking")
}

/// A moment's time, and its day when it isn't `date`.
fn when(at: i64, date: &str) -> String {
    Timestamp::from_microsecond(at).map_or_else(
        |_| "--:--".to_owned(),
        |at| {
            let zoned = at.to_zoned(TimeZone::system());
            let day = zoned.strftime("%Y-%m-%d").to_string();
            if day == date {
                zoned.strftime("%H:%M").to_string()
            } else {
                zoned.strftime("%b %d %H:%M").to_string()
            }
        },
    )
}

const AGES: [(&str, Align); 4] = [
    ("age", Align::Left),
    ("copies", Align::Right),
    ("hold", Align::Right),
    ("each", Align::Right),
];

/// Each watched program's copies by age, what each age holds together
/// and each copy on average, so a program that grows as it ages shows.
fn memory_by_age(day: &Day, pen: &Pen, room: Room) -> String {
    let mut blocks = vec![vec![format!("memory on {}{}", day.date, until(day.to))]];
    for (name, held) in day.memory.iter().filter(|(_, held)| held.most_kib > 0) {
        let rows: Vec<Vec<String>> = held
            .by_age
            .iter()
            .enumerate()
            .filter(|(_, aged)| aged.copies > 0)
            .map(|(group, aged)| {
                vec![
                    memory::aged(group),
                    aged.copies.to_string(),
                    memory::size(aged.kib),
                    memory::size(aged.kib / aged.copies),
                ]
            })
            .collect();
        let mut block = vec![format!("{name}: {}", holding(held))];
        block.extend(layout::table_in(&AGES, &rows, pen, room, &[]));
        blocks.push(block);
    }
    layout::joined(blocks)
}

const THINGS: [(&str, Align); 2] = [("thing", Align::Left), ("said", Align::Left)];

/// A thing's signatures' columns; the example is cut last.
const SAYINGS: [(&str, Align); 4] = [
    ("count", Align::Right),
    ("first", Align::Left),
    ("last", Align::Left),
    ("example", Align::Left),
];

/// What was asked for: a kind's things worst first, or the things whose
/// name holds the words, each with what it kept saying. Lists are the
/// terminal kit's tables, fitted to the room.
#[must_use]
pub fn asked(day: &Day, words: &str, pen: &Pen, room: Room) -> String {
    if words == "boot" {
        return boot_stages(day);
    }
    if words == "memory" {
        return memory_by_age(day, pen, room);
    }
    // Not `agents`, which is a daemon's name, whose lines that asks for.
    if words == "died" {
        return the_dead_by_name(day, pen, room);
    }
    if words == "launcher" {
        return launcher_by_part(day);
    }
    if words == "launches" {
        return launches_by_program(day, pen, room);
    }
    if words == "figures" {
        return figures_by_side(day, pen, room);
    }
    if words == "busy" {
        let said = day.busy.said();
        let said = if said.is_empty() {
            "nothing weighed\n".to_owned()
        } else {
            said
        };
        return format!("busy on {}{}\n{said}", day.date, until(day.to));
    }
    if let Some(&kind) = KINDS.iter().find(|kind| kind.name() == words) {
        let head = format!("{} on {}{}", kind.name(), day.date, until(day.to));
        let rows: Vec<Vec<String>> = worst_of(day, kind)
            .into_iter()
            .take(SHOWN)
            .map(|(name, thing)| vec![name.clone(), counted(&thing.badness(), 0)])
            .collect();
        let table = layout::table_in(&THINGS, &rows, pen, room, &[]);
        return headed(head, table, rows.len(), "nothing said a warning");
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
    let blocks = things
        .into_iter()
        .take(SHOWN)
        .map(|(name, thing)| {
            let mut said: Vec<_> = thing.signatures.values().collect();
            said.sort_by_key(|signature| (signature.priority, std::cmp::Reverse(signature.count)));
            let rows: Vec<Vec<String>> = said
                .into_iter()
                .take(SHOWN)
                .map(|signature| {
                    vec![
                        grouped(signature.count),
                        clock(signature.first),
                        clock(signature.last),
                        signature.example.clone(),
                    ]
                })
                .collect();
            let mut block = vec![format!("{name}: {}", counted(&thing.badness(), 0))];
            if !rows.is_empty() {
                block.extend(layout::table_in(&SAYINGS, &rows, pen, room, &[]));
            }
            block
        })
        .collect();
    layout::joined(blocks)
}

/// Each part of the launcher Joel used, how long it took and how many went
/// past its budget.
fn launcher_by_part(day: &Day) -> String {
    let mut out = format!("the launcher on {}{}\n", day.date, until(day.to));
    let said = day.launcher.said();
    if said.is_empty() {
        out.push_str("nothing timed\n");
    }
    for line in said {
        let _ = writeln!(out, "{line}");
    }
    out
}

const LAUNCHES: [(&str, Align); 2] = [("program", Align::Left), ("said", Align::Left)];

/// Each program's openings and ends, the slowest first.
fn launches_by_program(day: &Day, pen: &Pen, room: Room) -> String {
    let head = format!("launches on {}{}", day.date, until(day.to));
    let rows: Vec<Vec<String>> = day
        .launches
        .by_program()
        .into_iter()
        .map(|(name, said)| vec![name, said])
        .collect();
    if rows.is_empty() {
        return format!("{head}\nnothing timed\n");
    }
    layout::joined(vec![
        vec![head],
        layout::table_in(&LAUNCHES, &rows, pen, room, &[]),
    ])
}

const FIGURES: [(&str, Align); 3] = [
    ("side", Align::Left),
    ("figure", Align::Left),
    ("value", Align::Right),
];

/// Each figure cadence judges the day by, Joel's side first: what he
/// waited for or saw, then our stand-ins for it.
fn figures_by_side(day: &Day, pen: &Pen, room: Room) -> String {
    let head = format!("figures on {}{}", day.date, until(day.to));
    let all = figures::of(day);
    let rows: Vec<Vec<String>> = [(Side::His, "his"), (Side::Ours, "ours")]
        .into_iter()
        .flat_map(|(side, called)| {
            all.iter()
                .filter(move |(name, _)| figures::side(name) == side)
                .map(move |(name, value)| vec![called.to_owned(), name.clone(), figure(*value)])
        })
        .collect();
    let table = layout::table_in(&FIGURES, &rows, pen, room, &[]);
    headed(head, table, rows.len(), "nothing measured")
}

/// A figure, whole when it is and to a tenth when not. Adding zero
/// turns a negative zero, as minutes lost on a day with none come to,
/// into zero, so it never reads as -0.
fn figure(value: f64) -> String {
    let value = value + 0.0;
    if value.fract().abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}
