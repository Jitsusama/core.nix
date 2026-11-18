//! What a failed boot's console says, laid out from what systemd said:
//! each unit that failed, a step in from the edge, with what it said
//! under it, at most four of them so the screen holds them above the
//! shell's own words, and the way out last.

use terminal_kit::Pen;
use terminal_kit::layout::{self, Room};

/// How many units are shown, so the summary fits a screen a person can
/// read before the shell's own words.
pub const SHOWN: usize = 4;

/// The edge a unit's line starts from.
const MARGIN: &str = "  ";

/// What a unit said, set under its name, past its mark.
const UNDER: &str = "     ";

/// A unit that failed, as systemd tells of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failed {
    /// Its name, as `home.mount`.
    pub name: String,
    /// Its description, which is its name where it has none.
    pub about: String,
    /// The last its own programs said, or, when they said nothing, the
    /// last systemd said of it, a line each.
    pub said: Vec<String>,
}

/// The summary: the first `SHOWN` of `failed`, how many more there are
/// of `all`, and the way out, for a room `room` wide. Nothing failed
/// says nothing.
pub fn text(failed: &[Failed], all: usize, pen: &Pen, room: Room) -> String {
    if all == 0 {
        return String::new();
    }
    let mut out = String::new();
    for unit in failed.iter().take(SHOWN) {
        out.push('\n');
        out.push_str(&heading(unit, pen, room));
        out.push('\n');
        for line in &unit.said {
            for part in under(line, room) {
                out.push_str(UNDER);
                out.push_str(&pen.role("muted", &part));
                out.push('\n');
            }
        }
    }
    let shown = failed.len().min(SHOWN);
    if all > shown {
        out.push('\n');
        out.push_str(UNDER);
        out.push_str(&pen.role("muted", &format!("and {} more", all - shown)));
        out.push('\n');
    }
    out.push('\n');
    out.push_str(UNDER);
    out.push_str(&way_out(pen));
    out.push_str("\n\n");
    out
}

/// A unit's line: its mark in the alert colour, its name, and its
/// description beside it. A unit described only by its name has none,
/// and a description too long to sit beside the name goes, since the
/// name says which unit it is.
fn heading(unit: &Failed, pen: &Pen, room: Room) -> String {
    let name = clean(&unit.name);
    let about = clean(&unit.about);
    let mut line = format!(
        "{MARGIN}{} {}",
        pen.marked("failed", "alert"),
        pen.role("strong", &name)
    );
    let fits = match room {
        Room::Cells(width) => {
            UNDER.len() + layout::width(&name) + 2 + layout::width(&about) <= width
        }
        Room::Unbounded => true,
    };
    if about != name && !about.is_empty() && fits {
        line.push_str("  ");
        line.push_str(&pen.role("text", &about));
    }
    line
}

/// A line a unit said, wrapped between words to what's left of the room
/// past its inset, each line without the space it broke at. Into a pipe
/// it stays one line.
fn under(said: &str, room: Room) -> Vec<String> {
    let said = clean(said);
    let said = said.trim_end();
    match room {
        Room::Cells(width) => layout::wrapped(said, width.saturating_sub(UNDER.len()).max(1))
            .into_iter()
            .map(|line| line.trim_end().to_owned())
            .collect(),
        Room::Unbounded => vec![said.to_owned()],
    }
}

/// The keys out of a failed boot, as the console's greeting draws keys.
fn way_out(pen: &Pen) -> String {
    format!(
        "{} {} {} {}",
        pen.role("strong", "Ctrl+Alt+Delete"),
        pen.role("text", "restarts; hold"),
        pen.role("strong", "Space"),
        pen.role("text", "as it starts for an older generation.")
    )
}

/// Words from a unit, without what would steer the console rather than
/// be read: every control character, a tab read as a space.
fn clean(words: &str) -> String {
    words
        .chars()
        .filter_map(|c| match c {
            '\t' => Some(' '),
            c if c.is_control() => None,
            c => Some(c),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use terminal_kit::Ink;

    fn unit(name: &str, about: &str, said: &[&str]) -> Failed {
        Failed {
            name: name.to_owned(),
            about: about.to_owned(),
            said: said.iter().map(|line| (*line).to_owned()).collect(),
        }
    }

    fn console() -> Pen {
        Pen::new(
            Ink {
                console: true,
                ..Ink::plain()
            },
            None,
        )
    }

    fn lines(text: &str) -> Vec<&str> {
        text.lines().collect()
    }

    #[test]
    fn nothing_failed_says_nothing() {
        assert_eq!(text(&[], 0, &console(), Room::Cells(80)), "");
    }

    #[test]
    fn a_unit_is_named_and_described_with_what_it_said_under_it() {
        let failed = [unit(
            "home.mount",
            "Mount /home",
            &["probing the disk", "reading its label"],
        )];
        assert_eq!(
            text(&failed, 1, &console(), Room::Cells(80)),
            "\n  x  home.mount  Mount /home\n     probing the disk\n     reading its label\n\n     \
             Ctrl+Alt+Delete restarts; hold Space as it starts for an older generation.\n\n"
        );
    }

    #[test]
    fn what_a_unit_said_wraps_between_words_to_the_console() {
        let said = "mount: /home: wrong fs type, bad option, bad superblock on /dev/mapper/data, \
                    missing codepage or helper program, or other error.";
        let summary = text(
            &[unit("home.mount", "Mount /home", &[said])],
            1,
            &console(),
            Room::Cells(80),
        );
        let lines = lines(&summary);
        assert!(lines.contains(&"     mount: /home: wrong fs type, bad option, bad superblock on"));
        assert!(lines.contains(
            &"     /dev/mapper/data, missing codepage or helper program, or other error."
        ));
        assert!(
            lines.iter().all(|line| layout::width(line) <= 80),
            "a line is wider than the console: {summary}"
        );
        assert!(
            lines.iter().all(|line| !line.ends_with(' ')),
            "a line ends in a space: {summary}"
        );
    }

    #[test]
    fn into_a_pipe_what_a_unit_said_stays_one_line() {
        let said = "word ".repeat(40);
        let summary = text(
            &[unit("a.service", "a", &[&said])],
            1,
            &console(),
            Room::Unbounded,
        );
        assert!(summary.contains(&format!("     {}\n", said.trim_end())));
    }

    #[test]
    fn a_unit_without_a_description_or_with_one_too_long_is_named_alone() {
        let long =
            "A description far too long to sit beside its unit's name on a console eighty wide";
        let summary = text(
            &[
                unit("nameless.service", "nameless.service", &[]),
                unit("long.service", long, &[]),
            ],
            2,
            &console(),
            Room::Cells(80),
        );
        let lines = lines(&summary);
        assert!(lines.contains(&"  x  nameless.service"));
        assert!(lines.contains(&"  x  long.service"));
    }

    #[test]
    fn four_units_are_shown_and_the_rest_counted() {
        let failed: Vec<Failed> = ["a", "b", "c", "d", "e", "f"]
            .iter()
            .map(|name| unit(&format!("{name}.service"), name, &[]))
            .collect();
        let summary = text(&failed[..SHOWN], failed.len(), &console(), Room::Cells(80));
        assert_eq!(
            lines(&summary)
                .iter()
                .filter(|line| line.starts_with("  x  "))
                .count(),
            SHOWN
        );
        assert!(lines(&summary).contains(&"     and 2 more"));
    }

    #[test]
    fn a_unit_passes_no_control_character_to_the_console() {
        let summary = text(
            &[unit(
                "bell\u{7}.service",
                "Rings\u{1b}[2J",
                &["reading its label\u{7}", "a\tgap\u{1b}]0;title\u{7}"],
            )],
            1,
            &console(),
            Room::Cells(80),
        );
        assert!(
            !summary.chars().any(|c| c.is_control() && c != '\n'),
            "a control character reached the console: {summary:?}"
        );
        let lines = lines(&summary);
        assert!(lines.contains(&"  x  bell.service  Rings[2J"));
        assert!(lines.contains(&"     reading its label"));
        assert!(lines.contains(&"     a gap]0;title"));
    }

    #[test]
    fn the_mark_is_drawn_in_the_alert_colour_and_words_in_theirs() {
        let pen = Pen::coded(
            Ink {
                console: true,
                ..Ink::sixteen()
            },
            [
                ("alert", "31"),
                ("strong", "97"),
                ("text", "37"),
                ("muted", "90"),
            ],
        );
        let summary = text(
            &[unit("a.service", "A", &["said"])],
            1,
            &pen,
            Room::Cells(80),
        );
        assert!(summary.contains("\u{1b}[31mx\u{1b}[0m"), "{summary:?}");
        assert!(
            summary.contains("\u{1b}[97ma.service\u{1b}[0m"),
            "{summary:?}"
        );
        assert!(summary.contains("\u{1b}[90msaid\u{1b}[0m"), "{summary:?}");
        assert!(
            summary.contains("\u{1b}[97mCtrl+Alt+Delete\u{1b}[0m"),
            "{summary:?}"
        );
    }
}
