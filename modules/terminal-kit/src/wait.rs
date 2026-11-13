//! A wait said on stderr (docs/commands.md, Waiting): into a terminal,
//! one line rewriting in place, the running mark, what's being done, and
//! how far along where that's known, as a ten-cell braille meter over a
//! faint track; when it's over, its outcome stays in its place. Into a
//! pipe each event is a plain line of its own, and nothing moves.

use std::io::{self, Write};

use crate::ink::Pen;
use crate::layout::{self, Room};

/// The cells a meter takes, and the half cells it moves by.
const CELLS: usize = 10;
const HALVES: f64 = 20.0;

/// A cell full, half full from the left over the track, and the track.
const FULL: char = '\u{28ff}';
const HALF: char = '\u{28c7}';
const TRACK: char = '\u{28c0}';

/// How far along something is, as a meter's filled and empty cells.
fn meter(fraction: f64) -> (String, String) {
    // Half cells, so a meter moves on every twentieth.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=20 first"
    )]
    let halves = (fraction.clamp(0.0, 1.0) * HALVES).round() as usize;
    let mut filled: String = std::iter::repeat_n(FULL, halves / 2).collect();
    if halves % 2 == 1 {
        filled.push(HALF);
    }
    let empty = std::iter::repeat_n(TRACK, CELLS - layout::width(&filled)).collect();
    (filled, empty)
}

/// A wait being said.
pub struct Wait<'a, W: Write> {
    out: W,
    pen: &'a Pen,
    /// Whether it's said into a terminal, where it rewrites its line.
    terminal: bool,
    room: Room,
    words: String,
}

impl<'a, W: Write> Wait<'a, W> {
    /// A wait begun, said at once: what's being done.
    pub fn begin(mut out: W, pen: &'a Pen, room: Room, words: &str) -> io::Result<Self> {
        let terminal = room.cells().is_some();
        if !terminal {
            writeln!(out, "{}{words}", pen.marked("running", "muted"))?;
        }
        let mut wait = Self {
            out,
            pen,
            terminal,
            room,
            words: words.to_owned(),
        };
        if terminal {
            wait.draw(None)?;
        }
        Ok(wait)
    }

    /// How far along it is, and what it's doing now. Into a pipe a change
    /// of words is a line, and a fraction alone says nothing.
    pub fn step(&mut self, fraction: Option<f64>, words: Option<&str>) -> io::Result<()> {
        let changed = words.is_some_and(|words| words != self.words);
        if let Some(words) = words {
            words.clone_into(&mut self.words);
        }
        if self.terminal {
            self.draw(fraction)
        } else if changed {
            writeln!(
                self.out,
                "{}{}",
                self.pen.marked("running", "muted"),
                self.words
            )
        } else {
            Ok(())
        }
    }

    fn draw(&mut self, fraction: Option<f64>) -> io::Result<()> {
        let lead = layout::MARK;
        let gauge = fraction.map(meter);
        let used = lead + gauge.as_ref().map_or(0, |_| CELLS + layout::GAP);
        let words = match self.room.cells() {
            Some(cells) => layout::cut(&self.words, cells.saturating_sub(used + 1)),
            None => self.words.clone(),
        };
        let mut line = format!(
            "\r\u{1b}[2K{}{}",
            self.pen.marked("running", "muted"),
            self.pen.role("text", &words)
        );
        if let Some((filled, empty)) = gauge {
            line.push_str(&" ".repeat(layout::GAP));
            line.push_str(&self.pen.role("text", &filled));
            line.push_str(&self.pen.faint("rule", &empty));
        }
        write!(self.out, "{line}")?;
        self.out.flush()
    }

    /// It finished well: the done mark and what happened, in its place.
    pub fn done(self, words: &str) -> io::Result<()> {
        self.end("done", "success", words)
    }

    /// It failed: the failed mark and what happened, in its place, so the
    /// error after it reads on.
    pub fn failed(self, words: &str) -> io::Result<()> {
        self.end("failed", "alert", words)
    }

    fn end(mut self, mark: &str, role: &str, words: &str) -> io::Result<()> {
        let clear = if self.terminal { "\r\u{1b}[2K" } else { "" };
        writeln!(
            self.out,
            "{clear}{}{}",
            self.pen.marked(mark, role),
            self.pen.role("text", words)
        )?;
        self.out.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_meter_fills_by_half_cells_over_its_track() {
        assert_eq!(meter(0.0), (String::new(), "\u{28c0}".repeat(10)));
        assert_eq!(
            meter(0.25),
            ("\u{28ff}\u{28ff}\u{28c7}".into(), "\u{28c0}".repeat(7))
        );
        assert_eq!(meter(1.0), ("\u{28ff}".repeat(10), String::new()));
        assert_eq!(meter(7.0).0, "\u{28ff}".repeat(10));
    }

    #[test]
    fn into_a_pipe_each_event_is_a_plain_line() {
        let pen = Pen::plain();
        let mut out = Vec::new();
        let mut wait = Wait::begin(&mut out, &pen, Room::Unbounded, "building").expect("said");
        wait.step(Some(0.5), None).expect("said");
        wait.step(Some(0.6), Some("testing")).expect("said");
        wait.done("built example-nixos").expect("said");
        assert_eq!(
            String::from_utf8(out).expect("words"),
            "\u{2192} building\n\u{2192} testing\n\u{2713} built example-nixos\n"
        );
    }

    #[test]
    fn into_a_terminal_it_rewrites_one_line_and_leaves_its_outcome() {
        let pen = Pen::plain();
        let mut out = Vec::new();
        let mut wait = Wait::begin(&mut out, &pen, Room::Cells(40), "building").expect("said");
        wait.step(Some(0.5), None).expect("said");
        wait.failed("couldn't build").expect("said");
        assert_eq!(
            String::from_utf8(out).expect("words"),
            "\r\u{1b}[2K\u{2192} building\
             \r\u{1b}[2K\u{2192} building  \u{28ff}\u{28ff}\u{28ff}\u{28ff}\u{28ff}\
             \u{28c0}\u{28c0}\u{28c0}\u{28c0}\u{28c0}\
             \r\u{1b}[2K\u{2715} couldn't build\n"
        );
    }
}
