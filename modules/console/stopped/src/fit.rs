//! A failed boot's summary as the rows it takes, fitted to the console's
//! height. The console doesn't scroll back, so whatever runs past the top
//! is lost, and four units with long lines, the files the journal
//! skipped and the way out can pass a screen's rows with the shell's own
//! words under them. When they would, what's least needed goes first:
//! each unit's earlier lines, from whichever unit takes the most rows,
//! since its last line is most often why it stopped; then the example of
//! a skipped file, leaving the sentence that says lines may be missing;
//! then the start of a long line, keeping its end after the `more` mark;
//! then that sentence; then the last units, counted with the rest. Each
//! unit left keeps its name and the way out stays, however short the
//! screen.

use terminal_kit::Pen;
use terminal_kit::layout::Room;

use crate::summary::{UNDER, under, way_out};

/// The rows the emergency shell's own words take under the summary:
/// systemd's three saying what emergency mode is, sulogin's four on a
/// locked root account, and the row its cursor waits on.
pub const LEFT_FOR_THE_SHELL: usize = 8;

/// The summary, laid out but not yet drawn, so it can be fitted.
pub struct Screen {
    /// The units shown, in the order they're shown.
    pub units: Vec<Told>,
    /// How many units failed, those shown among them.
    pub all: usize,
    /// The files journalctl skipped, where it skipped any.
    pub skipped: Option<Skipped>,
}

/// A unit as it's shown: its heading, drawn, and the lines under it.
pub struct Told {
    pub heading: String,
    pub lines: Vec<Line>,
}

/// A line under a unit, whole or cut to its last rows.
pub struct Line {
    words: String,
    rows: Option<usize>,
}

/// The files journalctl skipped: whether there were several, the first
/// it said, and whether that first is shown as an example.
pub struct Skipped {
    pub several: bool,
    pub first: String,
    pub example: bool,
}

impl Line {
    /// A line shown whole.
    pub fn whole(words: String) -> Self {
        Self { words, rows: None }
    }

    /// Its rows in `room`: every row it wraps to, or, where it's cut to
    /// fewer, as many of its last words as fit them after the `more`
    /// mark, since the end of a line is most often where a program says
    /// why it stopped. A line cut to one row whose last word fills it
    /// alone takes two, the mark wrapping on its own.
    fn laid(&self, room: Room, more: &str) -> Vec<String> {
        let whole = under(&self.words, room);
        let Some(rows) = self.rows.filter(|rows| whole.len() > *rows) else {
            return whole;
        };
        // A line that wraps is a run of single spaces between its pieces,
        // so its rows split back into them.
        let pieces: Vec<&str> = whole.iter().flat_map(|row| row.split(' ')).collect();
        let tail = |from: usize| under(&format!("{more} {}", pieces[from..].join(" ")), room);
        (1..pieces.len())
            .map(tail)
            .find(|laid| laid.len() <= rows)
            .unwrap_or_else(|| tail(pieces.len() - 1))
    }
}

impl Skipped {
    /// What's said of the skipped files, a line each: the sentence, and
    /// the example under it while it's shown.
    fn told(&self) -> Vec<&str> {
        let sentence = match (self.several, self.example) {
            (false, true) => "journalctl skipped a journal file, so lines may be missing:",
            (true, true) => "journalctl skipped journal files, so lines may be missing, such as:",
            (false, false) => "journalctl skipped a journal file, so lines may be missing.",
            (true, false) => "journalctl skipped journal files, so lines may be missing.",
        };
        let mut told = vec![sentence];
        if self.example {
            told.push(&self.first);
        }
        told
    }
}

impl Screen {
    /// The summary drawn: each unit with what it said under it, how many
    /// more failed, the files the journal skipped, and the way out.
    pub fn drawn(&self, pen: &Pen, room: Room) -> String {
        let more = pen.mark("more");
        let mut out = String::new();
        let muted = |out: &mut String, part: &str| {
            out.push_str(UNDER);
            out.push_str(&pen.role("muted", part));
            out.push('\n');
        };
        for unit in &self.units {
            out.push('\n');
            out.push_str(&unit.heading);
            out.push('\n');
            for line in &unit.lines {
                for part in line.laid(room, more) {
                    muted(&mut out, &part);
                }
            }
        }
        if self.all > self.units.len() {
            out.push('\n');
            muted(
                &mut out,
                &format!("and {} more", self.all - self.units.len()),
            );
        }
        // A file the journal skipped may hold any unit's lines, or only an
        // earlier boot's, so it's said once, for them all, rather than under
        // a unit as its reason. The first says what kind of trouble it is. A
        // flaky disk can word one file two ways, so they're never counted.
        if let Some(skipped) = &self.skipped {
            out.push('\n');
            for part in skipped
                .told()
                .into_iter()
                .flat_map(|line| under(line, room))
            {
                muted(&mut out, &part);
            }
        }
        out.push('\n');
        out.push_str(UNDER);
        out.push_str(&way_out(pen));
        out.push_str("\n\n");
        out
    }

    /// Fitted to `rows` of a console, the shell's own words under it, as
    /// the module says; what can't go stays, however few the rows.
    pub fn fit(&mut self, rows: usize, pen: &Pen, room: Room) {
        let rows = rows.saturating_sub(LEFT_FOR_THE_SHELL);
        while self.drawn(pen, room).matches('\n').count() > rows
            && self.trimmed(room, pen.mark("more"))
        {}
    }

    /// Whether the least needed of what's left could go, and it went.
    fn trimmed(&mut self, room: Room, more: &str) -> bool {
        let taken = |unit: &Told| -> usize {
            unit.lines
                .iter()
                .map(|line| line.laid(room, more).len())
                .sum()
        };
        if let Some(unit) = self
            .units
            .iter_mut()
            .filter(|unit| unit.lines.len() > 1)
            .max_by_key(|unit| taken(unit))
        {
            unit.lines.remove(0);
            return true;
        }
        if let Some(skipped) = self.skipped.as_mut().filter(|skipped| skipped.example) {
            skipped.example = false;
            return true;
        }
        // A line is cut a row at a time, the longest first, each cut to
        // fewer rows than the last, so the cutting ends.
        let longest = self
            .units
            .iter_mut()
            .flat_map(|unit| unit.lines.iter_mut())
            .map(|line| (line.laid(room, more).len(), line))
            .filter(|(laid, line)| *laid > 1 && line.rows.is_none_or(|rows| *laid <= rows))
            .max_by_key(|(laid, _)| *laid);
        if let Some((laid, line)) = longest {
            line.rows = Some(laid - 1);
            return true;
        }
        if self.skipped.take().is_some() {
            return true;
        }
        if self.units.len() > 1 {
            self.units.pop();
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use terminal_kit::layout::Room;
    use terminal_kit::{Ink, Pen};

    use crate::summary::{Failed, text};

    fn unit(name: &str, said: &[&str]) -> Failed {
        Failed {
            name: name.to_owned(),
            about: name.to_owned(),
            said: said.iter().map(|line| (*line).to_owned()).collect(),
            unread: None,
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

    /// The rows a summary takes, as the console's cursor moves down them.
    fn rows(summary: &str) -> usize {
        summary.matches('\n').count()
    }

    /// A line of `words` words, each its number.
    fn long(words: usize) -> String {
        (1..=words)
            .map(|word| format!("w{word}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn four() -> Vec<Failed> {
        ["a", "b", "c", "d"]
            .iter()
            .map(|name| {
                unit(
                    &format!("{name}.service"),
                    &[
                        &long(30),
                        &long(30),
                        &format!("{name} stopped: {}", long(40)),
                    ],
                )
            })
            .collect()
    }

    #[test]
    fn a_summary_with_room_is_left_whole() {
        let failed = four();
        let skipped = ["Journal file /j/a.journal~ is truncated, ignoring file.".to_owned()];
        let unbounded = text(&failed, 6, &skipped, &console(), Room::Cells(80), None);
        let roomy = text(&failed, 6, &skipped, &console(), Room::Cells(80), Some(500));
        assert_eq!(roomy, unbounded, "a summary with rows to spare was cut");
    }

    #[test]
    fn every_console_is_given_no_more_rows_than_it_has() {
        let failed = four();
        let skipped = ["Journal file /j/a.journal~ is truncated, ignoring file.".to_owned()];
        // Four headings, "and 2 more" and the way out take 14 rows, which
        // stay whatever the height; from there up, the summary fits.
        for height in 22..80 {
            let summary = text(
                &failed,
                6,
                &skipped,
                &console(),
                Room::Cells(80),
                Some(height),
            );
            assert!(
                rows(&summary) <= height - super::LEFT_FOR_THE_SHELL,
                "{height} rows were given {} of summary: {summary}",
                rows(&summary)
            );
            assert!(
                summary.contains("Ctrl+Alt+Delete"),
                "{height} rows lost the way out: {summary}"
            );
        }
    }

    #[test]
    fn earlier_lines_go_first_from_the_unit_taking_most_rows() {
        let failed = [
            unit("a.service", &["a one", "a two", "a three"]),
            unit("b.service", &["b one", "b two"]),
        ];
        let whole = text(&failed, 2, &[], &console(), Room::Cells(80), None);
        let fitted = text(
            &failed,
            2,
            &[],
            &console(),
            Room::Cells(80),
            Some(rows(&whole) + 7),
        );
        assert!(
            !fitted.contains("a one") && fitted.contains("a two") && fitted.contains("b one"),
            "one row too few didn't cost the busiest unit its first line: {fitted}"
        );
    }

    #[test]
    fn a_long_last_line_keeps_its_end_after_the_mark() {
        let said = format!("{} because the disk went away", long(60));
        let failed = [unit("a.service", &[&said])];
        let fitted = text(&failed, 1, &[], &console(), Room::Cells(40), Some(17));
        assert!(
            fitted.contains("     ~ ") && fitted.contains("because the disk went away"),
            "a cut line lost its end or its mark: {fitted}"
        );
        assert!(
            !fitted.contains("w1 "),
            "a cut line kept its start: {fitted}"
        );
        assert!(
            rows(&fitted) <= 17 - super::LEFT_FOR_THE_SHELL,
            "a cut line still ran past the screen: {fitted}"
        );
    }

    #[test]
    fn the_skipped_files_example_goes_before_any_units_line() {
        let failed = [unit("a.service", &["a one"])];
        let skipped = ["Journal file /j/a.journal~ is truncated, ignoring file.".to_owned()];
        let whole = text(&failed, 1, &skipped, &console(), Room::Cells(80), None);
        let fitted = text(
            &failed,
            1,
            &skipped,
            &console(),
            Room::Cells(80),
            Some(rows(&whole) + 7),
        );
        assert!(
            fitted.contains("a one")
                && fitted
                    .contains("     journalctl skipped a journal file, so lines may be missing.\n")
                && !fitted.contains("truncated"),
            "the example didn't go first, or its sentence wasn't ended: {fitted}"
        );
    }

    #[test]
    fn units_past_a_short_screen_are_counted_with_the_rest() {
        let fitted = text(&four(), 6, &[], &console(), Room::Cells(80), Some(19));
        assert!(
            fitted.contains("a.service") && !fitted.contains("d.service"),
            "a short screen didn't keep the first units and count the last: {fitted}"
        );
        assert!(
            fitted.contains("     and 4 more\n") || fitted.contains("     and 3 more\n"),
            "the units left out weren't counted: {fitted}"
        );
        assert!(
            rows(&fitted) <= 19 - super::LEFT_FOR_THE_SHELL,
            "a short screen was given too many rows: {fitted}"
        );
    }

    #[test]
    fn a_unit_and_the_way_out_stay_however_short_the_screen() {
        let fitted = text(&four(), 6, &[], &console(), Room::Cells(80), Some(0));
        assert!(
            fitted.contains("a.service") && fitted.contains("Ctrl+Alt+Delete"),
            "a screen with no rows lost the first unit or the way out: {fitted}"
        );
        assert!(
            fitted.contains("     and 5 more\n"),
            "the units left out weren't counted: {fitted}"
        );
    }
}
