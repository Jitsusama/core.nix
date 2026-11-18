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
/// past its inset, each line without the space it broke at. A line that
/// fits keeps its own spacing, an indent or aligned columns among it. A
/// word too long for a line of its own is broken across lines, as `fold
/// -s` broke it, rather than shortened, since any of its characters, a
/// device's path among them, may be what says why the unit failed. The
/// pieces are laid greedily, each already fitting, rather than through
/// the kit's `wrapped`, which would join a piece that starts with a
/// figure to the next as its unit, and shorten them. Into a pipe it
/// stays one line. A blank line says nothing, so it's left out.
fn under(said: &str, room: Room) -> Vec<String> {
    let said = clean(said);
    let said = said.trim_end();
    if said.trim_start().is_empty() {
        return Vec::new();
    }
    let measure = match room {
        Room::Cells(width) => width.saturating_sub(UNDER.len()).max(1),
        Room::Unbounded => return vec![said.to_owned()],
    };
    if layout::width(said) <= measure {
        return vec![said.to_owned()];
    }
    let mut lines = Vec::new();
    let mut line = String::new();
    for piece in said
        .split_whitespace()
        .flat_map(|word| broken(word, measure))
    {
        if !line.is_empty() && layout::width(&line) + 1 + layout::width(piece) > measure {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(piece);
    }
    lines.push(line);
    lines
}

/// A word in pieces each at most `measure` cells wide, the last holding
/// what's left; a word that fits is one piece.
fn broken(word: &str, measure: usize) -> Vec<&str> {
    let mut pieces = Vec::new();
    let (mut start, mut used) = (0, 0);
    for (at, c) in word.char_indices() {
        let cells = layout::width(c.encode_utf8(&mut [0; 4]));
        if used + cells > measure && at > start {
            pieces.push(&word[start..at]);
            (start, used) = (at, 0);
        }
        used += cells;
    }
    pieces.push(&word[start..]);
    pieces
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
/// be read: an escape sequence whole, so a program that colours its own
/// log or titles a window leaves no `[31m` or `]0;title` behind, and
/// every other control character, a tab read as a space. A sequence cut
/// short by another escape ends there, as a terminal ends it, so the next
/// is dropped whole too.
fn clean(words: &str) -> String {
    let mut out = String::new();
    let mut chars = words.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            // Every sequence ends at the next escape, as a terminal ends
            // it, which then starts one of its own; only the string
            // terminator, an escape and a backslash, is part of one.
            '\u{1b}' => match chars.next_if(|&c| c != '\u{1b}') {
                // A control sequence: parameters and intermediates run
                // until a final byte.
                Some('[') => {
                    while let Some(c) = chars.next_if(|&c| c != '\u{1b}') {
                        if ('\u{40}'..='\u{7e}').contains(&c) {
                            break;
                        }
                    }
                }
                // A string: an operating system command, as a title or a
                // link, or a device control, privacy or application one.
                // It runs until a bell or the string terminator.
                Some(']' | 'P' | 'X' | '^' | '_') => {
                    while let Some(c) = chars.next_if(|&c| c != '\u{1b}') {
                        if c == '\u{7}' {
                            break;
                        }
                    }
                    let mut ahead = chars.clone();
                    if ahead.next() == Some('\u{1b}') && ahead.next() == Some('\\') {
                        chars = ahead;
                    }
                }
                // Any other, such as `ESC ( B`: intermediates, then one
                // final character.
                Some(c) if ('\u{20}'..='\u{2f}').contains(&c) => {
                    while chars
                        .next_if(|c| ('\u{20}'..='\u{2f}').contains(c))
                        .is_some()
                    {}
                    chars.next_if(|&c| c != '\u{1b}');
                }
                _ => {}
            },
            '\t' => out.push(' '),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// Whether `lines` say nothing a person could read once cleaned, as
/// they'd be shown: each blank, or only escapes and controls.
pub fn says_nothing(lines: &[String]) -> bool {
    lines.iter().all(|line| clean(line).trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use terminal_kit::Ink;

    #[test]
    fn lines_of_blanks_escapes_and_controls_say_nothing() {
        let lines = |said: &[&str]| {
            said.iter()
                .map(|line| (*line).to_owned())
                .collect::<Vec<_>>()
        };
        assert!(says_nothing(&[]), "no lines said something");
        assert!(
            says_nothing(&lines(&[
                "",
                "  \t",
                "\u{1b}[0m\u{7}",
                "\u{1b}]0;title\u{7}"
            ])),
            "lines that show nothing were taken as said"
        );
        assert!(
            !says_nothing(&lines(&["", "\u{1b}[31mdisk gone"])),
            "a line with words was taken as nothing"
        );
    }

    #[test]
    fn every_kind_of_escape_goes_whole_even_cut_short() {
        assert_eq!(
            clean("a\u{1b}Pq#0\u{1b}\\b\u{1b}_app\u{7}c\u{1b}^pm\u{1b}\\d\u{1b}Xsos\u{7}e"),
            "abcde",
            "a string sequence's payload reached the console"
        );
        for cut in [
            "red\u{1b}[31\u{1b}[0m plain",
            "red\u{1b}]0;tit\u{1b}[0m plain",
            "red\u{1b}(\u{1b}[0m plain",
            "red\u{1b}\u{1b}[0m plain",
        ] {
            assert_eq!(
                clean(cut),
                "red plain",
                "a sequence cut short by another ate the words after it or left a piece: {cut:?}"
            );
        }
    }

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

    /// Holds the summary to have this line whole among its own.
    fn says(summary: &str, line: &str) {
        assert!(
            summary.lines().any(|said| said == line),
            "no line {line:?} in {summary:?}"
        );
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
        says(
            &summary,
            "     mount: /home: wrong fs type, bad option, bad superblock on",
        );
        says(
            &summary,
            "     /dev/mapper/data, missing codepage or helper program, or other error.",
        );
        let lines = lines(&summary);
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
        says(&summary, &format!("     {}", said.trim_end()));
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
        says(&summary, "  x  nameless.service");
        says(&summary, "  x  long.service");
    }

    #[test]
    fn a_description_sits_beside_its_name_only_while_the_line_holds_it() {
        // Five cells of margin and mark, nine of the name and two between
        // leave 64 of 80 for the description.
        let fits = "d".repeat(64);
        let over = "d".repeat(65);
        let summary = text(
            &[unit("a.service", &fits, &[]), unit("b.service", &over, &[])],
            2,
            &console(),
            Room::Cells(80),
        );
        says(&summary, &format!("  x  a.service  {fits}"));
        says(&summary, "  x  b.service");
    }

    #[test]
    fn a_word_wider_than_the_console_is_broken_across_lines_whole() {
        let path = format!("/dev/disk/by-id/{}", "x".repeat(60));
        let said = format!("cannot open {path} for reading");
        let summary = text(
            &[unit("a.service", "A", &[&said])],
            1,
            &console(),
            Room::Cells(40),
        );
        // What it said is the lines between its heading and the blank
        // before the way out.
        let under: Vec<&str> = lines(&summary)
            .into_iter()
            .skip_while(|line| !line.starts_with("  x  "))
            .skip(1)
            .take_while(|line| !line.is_empty())
            .collect();
        assert!(
            under.iter().all(|line| layout::width(line) <= 40),
            "a line is wider than the console: {summary}"
        );
        assert_eq!(
            under
                .iter()
                .map(|line| line.trim_start())
                .collect::<Vec<_>>()
                .join(" ")
                .replace(' ', ""),
            said.replace(' ', ""),
            "a character of what it said was lost: {summary}"
        );
    }

    #[test]
    fn a_broken_figure_is_never_joined_to_its_unit_and_shortened() {
        // A figure just the width of the line, with its unit after it,
        // which the kit's own wrap would hold together and shorten.
        let said = format!("{} MB", "9".repeat(35));
        let summary = text(
            &[unit("a.service", "A", &[&said])],
            1,
            &console(),
            Room::Cells(40),
        );
        says(&summary, &format!("     {}", "9".repeat(35)));
        says(&summary, "     MB");
    }

    #[test]
    fn a_line_that_fits_keeps_its_own_spacing() {
        let said = "  sda1    ext4    /home";
        let summary = text(
            &[unit("a.service", "A", &[said])],
            1,
            &console(),
            Room::Cells(80),
        );
        says(&summary, &format!("     {said}"));
    }

    #[test]
    fn a_coloured_line_loses_its_sequences_whole_and_a_blank_one_goes() {
        let summary = text(
            &[unit(
                "a.service",
                "A",
                &[
                    "\u{1b}[1;31mred\u{1b}[0m\u{1b}(B\u{1b}]8;;https://x\u{1b}\\",
                    "",
                    "  \u{7}",
                ],
            )],
            1,
            &console(),
            Room::Cells(80),
        );
        let under: Vec<&str> = lines(&summary)
            .into_iter()
            .skip_while(|line| !line.starts_with("  x  "))
            .skip(1)
            .take_while(|line| !line.is_empty())
            .collect();
        assert_eq!(
            under,
            ["     red"],
            "a sequence's words or a blank line reached the console: {summary:?}"
        );
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
        says(&summary, "     and 2 more");
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
        says(&summary, "  x  bell.service  Rings");
        says(&summary, "     reading its label");
        says(&summary, "     a gap");
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
        assert!(summary.contains("\u{1b}[37mA\u{1b}[0m"), "{summary:?}");
        assert!(summary.contains("\u{1b}[90msaid\u{1b}[0m"), "{summary:?}");
        assert!(
            summary.contains("\u{1b}[97mCtrl+Alt+Delete\u{1b}[0m"),
            "{summary:?}"
        );
    }
}
