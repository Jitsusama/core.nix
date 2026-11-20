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
use terminal_kit::layout::{self, Room};

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
        // Fewer pieces never wrap to more rows, so the most that fit is
        // found by halving rather than by laying every tail, since a
        // journal line runs to 48K and the shell waits on this. Where
        // none fit, as the mark wrapping on its own, the last piece stays.
        let (mut from, mut last) = (1, pieces.len() - 1);
        while from < last {
            let half = from + (last - from) / 2;
            if tail(half).len() <= rows {
                last = half;
            } else {
                from = half + 1;
            }
        }
        tail(from)
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
        loop {
            let taken = rows_drawn(&self.drawn(pen, room), room);
            if taken <= rows || !self.trimmed(room, pen.mark("more"), taken - rows) {
                break;
            }
        }
    }

    /// Whether the least needed of what's left could go, and it went,
    /// with `over` rows still to find.
    fn trimmed(&mut self, room: Room, more: &str, over: usize) -> bool {
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
        // The longest line is cut by the rows still over, to one at
        // least, each cut to fewer rows than the last, so the cutting
        // ends; cut a row at a time, a 48K line would be laid hundreds
        // of times.
        let longest = self
            .units
            .iter_mut()
            .flat_map(|unit| unit.lines.iter_mut())
            .map(|line| (line.laid(room, more).len(), line))
            .filter(|(laid, line)| *laid > 1 && line.rows.is_none_or(|rows| *laid <= rows))
            .max_by_key(|(laid, _)| *laid);
        if let Some((laid, line)) = longest {
            line.rows = Some(laid.saturating_sub(over).max(1));
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

/// The rows `drawn` takes on a console `room` wide, as its cursor moves
/// down them: a line wider than the console, as a long unit's name or
/// the way out on a narrow one, wraps to more than one.
fn rows_drawn(drawn: &str, room: Room) -> usize {
    let Some(cells) = room.cells() else {
        return drawn.matches('\n').count();
    };
    drawn
        .split_inclusive('\n')
        .filter(|line| line.ends_with('\n'))
        .map(|line| {
            shown(line.trim_end_matches('\n'))
                .div_ceil(cells.max(1))
                .max(1)
        })
        .sum()
}

/// The cells a drawn line shows, without the colours the pen inks it in.
fn shown(line: &str) -> usize {
    let mut bare = String::new();
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // The pen inks with select-graphic sequences alone, which end
            // at their final byte.
            for c in chars.by_ref() {
                if ('\u{40}'..='\u{7e}').contains(&c) && c != '[' {
                    break;
                }
            }
        } else {
            bare.push(c);
        }
    }
    layout::width(&bare)
}

#[cfg(test)]
mod tests {
    use terminal_kit::layout::Room;
    use terminal_kit::{Ink, Pen};

    use crate::summary::tests::console;
    use crate::summary::{Failed, text};

    fn unit(name: &str, said: &[&str]) -> Failed {
        Failed {
            name: name.to_owned(),
            about: name.to_owned(),
            said: said.iter().map(|line| (*line).to_owned()).collect(),
            unread: None,
        }
    }

    /// The rows a summary takes, as the console's cursor moves down them,
    /// where none of its lines is wider than the console.
    fn rows(summary: &str) -> usize {
        summary.matches('\n').count()
    }

    /// The rows a summary drawn uninked takes on a console `cells` wide,
    /// each line wrapping to as many rows as its characters fill.
    fn rows_at(summary: &str, cells: usize) -> usize {
        summary
            .split_inclusive('\n')
            .map(|line| line.trim_end().chars().count().div_ceil(cells).max(1))
            .sum()
    }

    /// A summary without the colours a pen inks it in.
    fn bare(summary: &str) -> String {
        let mut bare = String::new();
        let mut chars = summary.chars();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                chars.by_ref().find(|c| *c == 'm');
            } else {
                bare.push(c);
            }
        }
        bare
    }

    /// The units a summary shows, by their headings.
    fn units_shown(summary: &str) -> usize {
        summary
            .lines()
            .filter(|line| line.starts_with("  ") && !line.starts_with("     "))
            .count()
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
        // However short the screen, one unit with a row of its last line,
        // the count of the rest and the way out stay, each with the blank
        // row above it, and the blank below: eight rows. Past that the
        // summary fits, units going at the shortest.
        let least = 8;
        assert_eq!(
            rows(&text(
                &failed,
                6,
                &skipped,
                &console(),
                Room::Cells(80),
                Some(0)
            )),
            least,
            "a screen with no rows kept more than a unit, the count and the way out"
        );
        for height in least + super::LEFT_FOR_THE_SHELL..80 {
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
            let first = summary
                .split("a.service")
                .nth(1)
                .and_then(|after| after.split("\n\n").next())
                .unwrap_or_default();
            assert!(
                first.contains("w40"),
                "{height} rows lost the first unit or its last line's end: {summary}"
            );
        }
    }

    #[test]
    fn a_tie_for_most_rows_costs_the_later_unit_its_line() {
        let failed = [
            unit("a.service", &["a one", "a two"]),
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
            fitted.contains("a one") && !fitted.contains("b one") && fitted.contains("b two"),
            "a tie cost the first unit, which most often says why, its line: {fitted}"
        );
    }

    #[test]
    fn a_cut_line_keeps_every_row_it_has_room_for() {
        let failed = [unit("a.service", &[&long(400)])];
        // A blank row, the heading, the blank row and the way out, and
        // the blank under it take five; the line has the rest.
        for height in 14..36 {
            let fitted = text(&failed, 1, &[], &console(), Room::Cells(80), Some(height));
            assert_eq!(
                rows(&fitted),
                height - super::LEFT_FOR_THE_SHELL,
                "at {height} rows a cut line didn't fill the rows left it: {fitted}"
            );
        }
    }

    #[test]
    fn an_inked_summary_fits_as_its_words_would_uninked() {
        let inked = Pen::coded(
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
        let failed = four();
        for height in 16..60 {
            let plain = text(&failed, 6, &[], &console(), Room::Cells(40), Some(height));
            let coloured = text(&failed, 6, &[], &inked, Room::Cells(40), Some(height));
            assert!(coloured.contains('\u{1b}'), "the inked pen drew no colour");
            assert_eq!(
                bare(&coloured),
                plain,
                "at {height} rows a colour's escapes were counted as cells they don't take"
            );
        }
    }

    #[test]
    fn a_mark_that_wraps_alone_stops_the_cutting() {
        // Fifteen cells fill a row of a console twenty wide under its five
        // spaces, so the mark before the last word wraps to a row of its
        // own, and the line can't be cut to fewer than two.
        let word = "x".repeat(15);
        let said = format!("{} {word}", long(10));
        let failed = [unit("a.service", &[&said])];
        let fitted = text(&failed, 1, &[], &console(), Room::Cells(20), Some(0));
        assert!(
            fitted.contains(&format!("     ~\n     {word}\n")),
            "a line whose last word fills a row lost it or its mark: {fitted}"
        );
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
            rows_at(&fitted, 40) <= 17 - super::LEFT_FOR_THE_SHELL,
            "a cut line still ran past the screen: {fitted}"
        );
    }

    #[test]
    fn the_skipped_files_example_goes_before_a_line_is_cut() {
        let failed = [unit("a.service", &[&long(30)])];
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
            fitted.contains("w1 ")
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
            fitted.contains(&format!("     and {} more\n", 6 - units_shown(&fitted))),
            "the units left out weren't counted as those not shown: {fitted}"
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

    #[test]
    fn a_journal_length_line_is_fitted_at_once() {
        // journald keeps a line of up to 48K; laid a tail at a time and
        // cut a row at a time, this took minutes the shell waits on.
        let said = format!("{} because the disk went away", long(8000));
        let failed = [unit("a.service", &[&said])];
        let started = std::time::Instant::now();
        let fitted = text(&failed, 1, &[], &console(), Room::Cells(80), Some(24));
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "a 48K line took {:?} to fit",
            started.elapsed()
        );
        assert!(
            fitted.contains("because the disk went away") && rows(&fitted) <= 16,
            "a 48K line lost its end or ran past the screen: {fitted}"
        );
    }

    #[test]
    fn four_units_of_journal_length_lines_are_fitted_in_time() {
        // The most the summary reads: four units of three lines, each as
        // long as journald keeps one.
        let failed: Vec<_> = ["a", "b", "c", "d"]
            .iter()
            .map(|name| {
                let said = format!("{} {name} stopped", long(8000));
                unit(&format!("{name}.service"), &[&said, &said, &said])
            })
            .collect();
        // The old cut took 28 s on one such line in a release build, so
        // either bound catches it: five seconds as the package builds it,
        // and thirty for a debug `cargo test`, which took 5.6 s here.
        let bound = if cfg!(debug_assertions) { 30 } else { 5 };
        let started = std::time::Instant::now();
        let fitted = text(&failed, 4, &[], &console(), Room::Cells(80), Some(24));
        assert!(
            started.elapsed() < std::time::Duration::from_secs(bound),
            "four units of 48K lines took {:?} to fit",
            started.elapsed()
        );
        assert!(
            fitted.contains("a stopped") && rows(&fitted) <= 16,
            "four units of 48K lines lost the first's end or ran past the screen: {fitted}"
        );
    }

    #[test]
    fn a_heading_wider_than_the_console_is_counted_as_it_wraps() {
        let name = format!("{}.mount", "a".repeat(100));
        let failed = [
            unit(&name, &["one", "two", "three"]),
            unit("b.service", &["b one"]),
        ];
        let fitted = text(&failed, 2, &[], &console(), Room::Cells(40), Some(20));
        // The console's pen inks nothing here, so each row is its
        // characters, forty to a row.
        assert!(
            rows_at(&fitted, 40) <= 20 - super::LEFT_FOR_THE_SHELL,
            "a wrapped heading ran the summary past the screen: {fitted}"
        );
        assert!(
            fitted.contains(&name) && fitted.contains("three"),
            "the first unit lost its name or its last line: {fitted}"
        );
    }
}
