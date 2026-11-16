//! The system's screen: a line for each part, then one for each machine,
//! each wearing its health as a mark and a word, then where each surface
//! goes deeper. What a line says is the point of it, so in a terminal it
//! wraps under itself rather than giving way. Every line that starts a
//! record names it, as docs/commands.md asks, so `system | grep network`
//! reads alone, and into a pipe each record is one line. A thing with a home
//! on the local sites is a link to it, where the terminal draws links.

use terminal_kit::Pen;
use terminal_kit::layout::{self, Align, GAP, ITEMS, MARK, Room};

use system::record::{Health, System};

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

/// A record's line: its name, how it stands, what it says, and its home on
/// the local sites when it has one.
type Record<'a> = (String, Health, &'a str, Option<String>);

/// Records as lines: the mark, the name and the health's word each in a
/// column of their own, then what it says, wrapped under itself. A name
/// with a home links to it.
fn records(records: &[Record], pen: &Pen, room: Room) -> Vec<String> {
    let names = records
        .iter()
        .map(|(name, ..)| layout::width(name))
        .max()
        .unwrap_or(0);
    let words = records
        .iter()
        .map(|(_, health, ..)| layout::width(health.word()))
        .max()
        .unwrap_or(0);
    let lead = MARK + names + GAP + words + GAP;
    records
        .iter()
        .map(|(name, health, says, home)| {
            let (mark, role) = marked(*health);
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
                pen.role(role, health.word()),
                pad(health.word(), words),
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
                part.health,
                part.says.as_str(),
                Some(format!("{SITES}{}", part.deeper.site)),
            )
        })
        .collect();
    let machines: Vec<Record> = system
        .parts
        .iter()
        .flat_map(|part| &part.things)
        .filter(|thing| thing.kind == "machine")
        .map(|thing| {
            (
                format!("machine {}", thing.name),
                thing.health,
                thing.says.as_str(),
                thing
                    .address
                    .as_ref()
                    .map(|address| format!("{SITES}{address}")),
            )
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

#[cfg(test)]
mod tests {
    use super::*;
    use system::record::{Deeper, Part, Thing};
    use terminal_kit::Ink;

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
                    things: vec![Thing {
                        kind: "machine",
                        name: "laptop".to_owned(),
                        health: Health::Watch,
                        says: "is short of memory, runs 3 busy sessions".to_owned(),
                        address: Some("mycelium/#machine=laptop".to_owned()),
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
            for line in said.lines() {
                assert!(
                    layout::width(line) <= cells,
                    "{line:?} is wider than {cells} cells"
                );
            }
        }
    }
}
