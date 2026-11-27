//! The system's screen: a line for each part, then one for each machine,
//! each wearing its health as a mark and a word, then where each surface
//! goes deeper. What a line says is the point of it, so in a terminal it
//! wraps under itself rather than giving way. Every line that starts a
//! record names it, as docs/commands.md asks, so `system | grep network`
//! reads alone, and into a pipe each record is one line. A thing with a home
//! on the local sites is a link to it, where the terminal draws links.
//! Under each machine go its ties to the other three parts, a line each
//! naming the machine; `system machine <name>` draws one machine whole.

use serde_json::Value;
use terminal_kit::Pen;
use terminal_kit::layout::{self, Align, GAP, ITEMS, MARK, Room};

use system::record::{Health, System, Thing};

use crate::said::{self, Said};

/// Where the local sites are served from (modules/local-sites).
pub const SITES: &str = "http://sites.localhost:8484/";

/// The mark a health wears, and the role it and the health's word take.
fn marked(health: Health) -> (&'static str, &'static str) {
    match health {
        Health::Good => ("done", "success"),
        Health::Watch => ("caution", "warning"),
        Health::Bad => ("failed", "alert"),
        Health::Unread => ("", "muted"),
    }
}

/// A record's line: its name, how it stands (none for a machine's tie,
/// which stands as its machine does), what it says, and its home on the
/// local sites when it has one.
type Record = (String, Option<Health>, String, Option<String>);

/// Records as lines: the mark, the name and the health's word each in a
/// column of their own, then what it says, wrapped under itself. A name
/// with a home links to it.
fn records(records: &[Record], pen: &Pen, room: Room) -> Vec<String> {
    let names = records
        .iter()
        .map(|(name, ..)| layout::width(name))
        .max()
        .unwrap_or(0);
    let word = |health: &Option<Health>| health.map_or("", Health::word);
    let words = records
        .iter()
        .map(|(_, health, ..)| layout::width(word(health)))
        .max()
        .unwrap_or(0);
    let lead = MARK + names + GAP + words + GAP;
    records
        .iter()
        .map(|(name, health, says, home)| {
            let (mark, role) = marked(health.unwrap_or(Health::Unread));
            let mark = if mark.is_empty() {
                " ".repeat(MARK)
            } else {
                pen.marked(mark, role)
            };
            let pad = |words: &str, to: usize| " ".repeat(to - layout::width(words) + GAP);
            let drawn = pen.role("text", name);
            format!(
                "{mark}{}{}{}{}{}",
                home.as_ref()
                    .map_or_else(|| drawn.clone(), |home| pen.link(home, &drawn)),
                pad(name, names),
                pen.role(role, word(health)),
                pad(word(health), words),
                pen.role("muted", &layout::hung(says, lead, room)),
            )
        })
        .collect()
}

/// The whole system, laid out for the room.
pub fn system(system: &System, pen: &Pen, room: Room) -> String {
    let head = vec![format!(
        "{}{}",
        pen.title("system"),
        pen.role("muted", &format!("{ITEMS}read on {}", system.machine))
    )];
    let parts: Vec<Record> = system
        .parts
        .iter()
        .map(|part| {
            (
                part.name.to_owned(),
                Some(part.health),
                part.says.clone(),
                Some(format!("{SITES}{}", part.deeper.site)),
            )
        })
        .collect();
    let machines: Vec<Record> = machines(system)
        .flat_map(|thing| {
            let line = (
                format!("machine {}", thing.name),
                Some(thing.health),
                thing.says.clone(),
                home(thing),
            );
            std::iter::once(line).chain(tied(thing, &system.machine))
        })
        .collect();
    let further: Vec<Vec<String>> = system
        .parts
        .iter()
        .map(|part| {
            vec![
                part.name.to_owned(),
                part.deeper.command.to_owned(),
                format!("{SITES}{}", part.deeper.site),
                part.deeper.launcher.to_owned(),
            ]
        })
        .collect();
    layout::joined(vec![
        head,
        records(&parts, pen, room),
        records(&machines, pen, room),
        layout::table_in(
            &[
                ("deeper", Align::Left),
                ("command", Align::Left),
                ("site", Align::Left),
                ("launcher", Align::Left),
            ],
            &further,
            pen,
            room,
            &[3, 2],
        ),
    ])
}

/// Every machine the parts list.
fn machines(system: &System) -> impl Iterator<Item = &Thing> {
    system
        .parts
        .iter()
        .flat_map(|part| &part.things)
        .filter(|thing| thing.kind == "machine")
}

/// A thing's home on the local sites, when a site is home to it.
fn home(thing: &Thing) -> Option<String> {
    thing
        .address
        .as_ref()
        .map(|address| format!("{SITES}{address}"))
}

/// A machine's three ties to the other parts, a line each naming it, in
/// their main words, read on the machine `here` names; none for a machine
/// with no ties.
fn tied(thing: &Thing, here: &str) -> Vec<Record> {
    if thing.ties.is_null() {
        return Vec::new();
    }
    [
        ("cells", said::cells(&thing.ties)),
        ("network", said::network(&thing.ties)),
        (
            "code",
            said::code(&thing.ties, &thing.name, thing.name == here),
        ),
    ]
    .into_iter()
    .map(|(tie, said)| (format!("{} {tie}", thing.name), None, said.said, None))
    .collect()
}

/// A tie whole, its main words and what goes beside them, when anything
/// does.
fn told(said: &Said) -> String {
    if said.aside.is_empty() {
        return said.said.clone();
    }
    format!("{}{ITEMS}{}", said.said, said.aside)
}

/// One machine, whole: how it stands, its kind, its ties to the other
/// parts, and what can be done with it; none when no part lists it by
/// that name. `now` is the moment it's read at, in seconds since 1970.
pub fn machine(system: &System, name: &str, pen: &Pen, room: Room, now: i64) -> Option<String> {
    let thing = machines(system).find(|thing| thing.name == name)?;
    let head = vec![format!(
        "{}{}",
        pen.title(&format!("machine {name}")),
        pen.role("muted", &format!("{ITEMS}read on {}", system.machine))
    )];
    let stands = (
        name.to_owned(),
        Some(thing.health),
        thing.says.clone(),
        None,
    );
    let ties = &thing.ties;
    let acts = ties.get("acts").filter(|acts| acts.is_object());
    let commands = acts
        .and_then(|acts| acts.get("commands")?.as_array())
        .map(|commands| {
            commands
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(ITEMS)
        });
    let facts = [
        ("kind", said::kind(ties, name, &system.machine)),
        ("cells", Some(told(&said::cells(ties)))),
        ("network", Some(told(&said::network(ties)))),
        (
            "code",
            Some(told(&said::code(ties, name, name == system.machine))),
        ),
        ("itself", Some(told(&said::itself(ties, now)))),
        ("home", home(thing)),
        (
            "launcher",
            acts.and_then(|acts| acts.get("link")?.as_str().map(str::to_owned)),
        ),
        ("commands", commands),
    ];
    let shown: Vec<(&str, Option<&str>)> = facts
        .iter()
        .filter_map(|(label, value)| value.as_deref().map(|value| (*label, Some(value))))
        .collect();
    Some(layout::joined(vec![
        head,
        records(&[stands], pen, room),
        layout::facts(&shown, "", pen, room),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use system::record::{Deeper, Part, Thing};
    use terminal_kit::Ink;

    /// 2026-11-15T19:20:00Z, 20 minutes after the laptop's update.
    const NOW: i64 = 1_794_770_400;

    fn ties() -> serde_json::Value {
        json!({
            "cells": {"busy": 3, "idle": 1, "ended": 0},
            "network": [{"by": "5 GHz", "point": "Hall", "bars": 5, "room_mbps": 183}],
            "code": {"repositories": 12, "checkouts": 14, "trees": 3, "behind": 1,
                     "changes": 2},
            "figures": {"model": "Example 14", "system": "linux",
                        "disk": 1_023_000_000_000_u64, "disk_free": 248_000_000_000_u64,
                        "updated_at": "2026-11-15T19:00:00Z", "update_step": "done"},
            "acts": {"link": "mycelium:machine/laptop?from=system",
                      "commands": ["mycelium sprout --on laptop", "mycelium update laptop"]},
        })
    }

    fn whole() -> System {
        let deeper = |command, site, launcher| Deeper {
            command,
            site,
            launcher,
        };
        System {
            machine: "laptop".to_owned(),
            read_at: "2026-11-15T19:00:00Z".to_owned(),
            parts: vec![
                Part {
                    name: "machines",
                    health: Health::Watch,
                    says: "laptop is short of memory and processor, has a filling disk, \
                           248 GB free, runs 70 busy and 20 idle sessions"
                        .to_owned(),
                    side: Vec::new(),
                    holds: Some("1 machine".to_owned()),
                    things: vec![Thing {
                        kind: "machine",
                        name: "laptop".to_owned(),
                        health: Health::Watch,
                        says: "is short of memory, runs 3 busy sessions".to_owned(),
                        side: Vec::new(),
                        looks: Vec::new(),
                        address: Some("mycelium/#machine=laptop".to_owned()),
                        ties: ties(),
                    }],
                    deeper: deeper(
                        "mycelium machines",
                        "mycelium/",
                        "Agents › Networks › Machines",
                    ),
                },
                Part {
                    name: "network",
                    health: Health::Unread,
                    says: "net isn't on this machine".to_owned(),
                    side: Vec::new(),
                    holds: None,
                    things: Vec::new(),
                    deeper: deeper("net", "network/", "Network"),
                },
            ],
        }
    }

    #[test]
    fn every_record_into_a_pipe_is_one_line_naming_it() {
        let said = system(&whole(), &Pen::plain(), Room::Unbounded);
        let lines: Vec<&str> = said.lines().collect();
        assert_eq!(lines[0], "system · read on laptop");
        assert!(
            lines.contains(
                &"! machines  watch     laptop is short of memory and processor, has a filling \
                  disk, 248 GB free, runs 70 busy and 20 idle sessions"
            ),
            "{said}"
        );
        assert!(
            lines.contains(&"  network   not read  net isn't on this machine"),
            "a part not read wears no mark and says why: {said}"
        );
        assert!(
            lines.contains(&"! machine laptop  watch  is short of memory, runs 3 busy sessions"),
            "each machine has its own line: {said}"
        );
        assert!(
            lines.iter().any(|line| line.starts_with("network")
                && line.contains("http://sites.localhost:8484/network/")
                && line.ends_with("Network")),
            "where to go deeper names the command, the site and the launcher: {said}"
        );
    }

    /// A line's words, its spacing aside.
    fn words(line: &str) -> String {
        line.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn each_machine_says_its_three_ties_each_on_a_line_naming_it() {
        let said = system(&whole(), &Pen::plain(), Room::Unbounded);
        let lines: Vec<String> = said.lines().map(words).collect();
        let at = lines
            .iter()
            .position(|line| line.starts_with("! machine laptop"))
            .expect("the machine's line");
        assert_eq!(
            lines[at + 1..at + 4],
            [
                "laptop cells 3 busy and 1 idle",
                "laptop network Wi-Fi to the Hall point, on 5 GHz",
                "laptop code 14 checkouts of 12 repositories",
            ],
            "{said}"
        );
        let mut bare = whole();
        bare.parts[0].things[0].ties = serde_json::Value::Null;
        let said = system(&bare, &Pen::plain(), Room::Unbounded);
        assert!(
            !said.contains("laptop cells"),
            "a machine with no ties says none: {said}"
        );
    }

    #[test]
    fn one_machine_is_drawn_whole_with_what_can_be_done_with_it() {
        let said = machine(&whole(), "laptop", &Pen::plain(), Room::Unbounded, NOW)
            .expect("the laptop is listed");
        let lines: Vec<String> = said.lines().map(words).collect();
        assert_eq!(lines[0], "machine laptop · read on laptop");
        for line in [
            "! laptop watch is short of memory, runs 3 busy sessions",
            "kind Example 14, Linux, this machine",
            "cells 3 busy and 1 idle · none has ended",
            "network Wi-Fi to the Hall point, on 5 GHz · a strong signal, with 183 Mb/s of room",
            "code 14 checkouts of 12 repositories · 3 trees; 1 checkout behind its upstream; \
             2 open changes",
            "itself 248 GB of 1 TB free · updated 20 m ago",
            "home http://sites.localhost:8484/mycelium/#machine=laptop",
            "launcher mycelium:machine/laptop?from=system",
            "commands mycelium sprout --on laptop · mycelium update laptop",
        ] {
            assert!(lines.contains(&line.to_owned()), "{line:?} in {said}");
        }
        assert_eq!(
            machine(&whole(), "Laptop", &Pen::plain(), Room::Unbounded, NOW),
            None,
            "a machine is named as the node names it"
        );
    }

    #[test]
    fn a_machine_with_no_acts_offers_none() {
        let mut bare = whole();
        bare.parts[0].things[0].ties["acts"] = serde_json::Value::Null;
        bare.parts[0].things[0].address = None;
        let said = machine(&bare, "laptop", &Pen::plain(), Room::Unbounded, NOW)
            .expect("the laptop is listed");
        for label in ["launcher", "commands", "home"] {
            assert!(
                !said.lines().any(|line| line.starts_with(label)),
                "{label} isn't offered: {said}"
            );
        }
    }

    #[test]
    fn a_tie_with_nothing_aside_ends_with_its_words() {
        let mut bare = whole();
        bare.parts[0].things[0].ties["figures"] = serde_json::json!({});
        let said = machine(&bare, "laptop", &Pen::plain(), Room::Unbounded, NOW)
            .expect("the laptop is listed");
        let itself = said
            .lines()
            .find(|line| line.starts_with("itself"))
            .expect("itself is drawn");
        assert!(itself.ends_with("its disk not said"), "{itself:?}");
    }

    #[test]
    fn what_a_record_says_wraps_under_itself_rather_than_going() {
        let said = system(&whole(), &Pen::plain(), Room::Cells(60));
        let at = said.find("machines  watch").expect("the machines' line");
        let lines: Vec<&str> = said[at..].lines().take(3).collect();
        // Two cells of mark, the widest name and the widest health's word,
        // each with its gap: what it says hangs 22 cells in.
        assert!(lines[1].starts_with(&" ".repeat(22)), "{said}");
        assert!(lines[1].chars().nth(22).is_some_and(|c| c != ' '), "{said}");
        assert!(said.contains("filling disk"), "nothing it says is lost");
    }

    #[test]
    fn a_thing_with_a_home_links_to_it_where_links_are_drawn() {
        let said = system(&whole(), &Pen::new(Ink::sixteen(), None), Room::Unbounded);
        assert!(
            said.contains("\u{1b}]8;;http://sites.localhost:8484/mycelium/#machine=laptop\u{1b}\\"),
            "the machine links to its home: {said:?}"
        );
        assert!(
            said.contains("\u{1b}]8;;http://sites.localhost:8484/network/\u{1b}\\"),
            "a part links to its site: {said:?}"
        );
        let plain = system(&whole(), &Pen::plain(), Room::Unbounded);
        assert!(
            !plain.contains('\u{1b}'),
            "into a pipe, the words alone: {plain:?}"
        );
    }

    #[test]
    fn every_line_fits_a_narrow_terminal() {
        for cells in [40, 60, 80, 120] {
            let said = system(&whole(), &Pen::plain(), Room::Cells(cells));
            let one = machine(&whole(), "laptop", &Pen::plain(), Room::Cells(cells), NOW)
                .expect("the laptop is listed");
            for line in said.lines().chain(one.lines()) {
                assert!(
                    layout::width(line) <= cells,
                    "{line:?} is wider than {cells} cells"
                );
            }
        }
    }
}
