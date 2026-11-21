//! How a selection moves as Neovim moves one: by character, word, line
//! and paragraph, each step taken over the page's places.

use super::{Class, Layout, Motion, Pos, Selection};

/// Lines whose baselines are this close are one row of the page, as a
/// table's cells or a bullet and its item are.
const ROW_PT: f64 = 2.0;

/// Further across than any line goes: after $, j and k keep to each
/// line's end, as Vim's curswant past every column does.
const LINE_END_PT: f64 = 1.0e6;

impl Layout {
    /// Moves the cursor `count` times.
    pub fn moved(&self, selection: &mut Selection, motion: Motion, count: usize) {
        // f and F with a count go to that many of the character on, or
        // nowhere when the line has fewer, as Vim's fail whole.
        if let Motion::ToLetter(letter) | Motion::ToLetterBack(letter) = motion {
            let on = matches!(motion, Motion::ToLetter(_));
            if let Some(at) = self.letter_from(selection.cursor, letter, on, count.max(1)) {
                selection.put(at);
            }
            return;
        }
        let mut at = selection.cursor;
        let vertical = matches!(motion, Motion::Down | Motion::Up);
        let want = if vertical {
            let want = selection.want_pt.unwrap_or_else(|| self.across(at));
            selection.want_pt = Some(want);
            want
        } else {
            selection.want_pt = None;
            0.0
        };
        for _ in 0..count.max(1) {
            let next = match motion {
                Motion::Left => (at.ch > 0).then(|| Pos {
                    ch: at.ch - 1,
                    ..at
                }),
                Motion::Right => (at.ch + 1 < self.len(at.line)).then(|| Pos {
                    ch: at.ch + 1,
                    ..at
                }),
                Motion::Down => self
                    .row_from(at.line, true, want)
                    .map(|line| Pos { line, ch: 0 }),
                Motion::Up => self
                    .row_from(at.line, false, want)
                    .map(|line| Pos { line, ch: 0 }),
                Motion::WordOn => self.word_on(at),
                Motion::WordBack => self.word_back(at),
                Motion::WordEnd => self.word_end(at),
                Motion::LineStart => Some(Pos { ch: 0, ..at }),
                Motion::LineEnd => Some(Pos {
                    ch: self.len(at.line).saturating_sub(1),
                    ..at
                }),
                Motion::ParagraphOn => self.paragraph_on(at),
                Motion::ParagraphBack => self.paragraph_back(at),
                Motion::ToLetter(_) | Motion::ToLetterBack(_) => None,
                Motion::First => (0..self.lines.len())
                    .find(|&line| self.len(line) > 0)
                    .map(|line| Pos { line, ch: 0 }),
                Motion::Last => self.end().map(|end| Pos { ch: 0, ..end }),
            };
            match next {
                Some(next) => at = next,
                None => break,
            }
        }
        if let (true, Some(want)) = (vertical, selection.want_pt) {
            at.ch = self.nearest(at.line, want);
        }
        selection.cursor = at;
        selection.taken = None;
        if matches!(motion, Motion::LineEnd) {
            selection.want_pt = Some(LINE_END_PT);
        }
    }

    /// j and k: the line of the next row down the page that has any
    /// characters, or up, and of its lines the one across from `want_pt`
    /// or nearest it, so a table's j goes to the cell under, never the
    /// one beside.
    fn row_from(&self, line: usize, down: bool, want_pt: f64) -> Option<usize> {
        let baseline = |line: usize| self.lines[line].baseline_pt;
        let here = baseline(line);
        let filled = || (0..self.lines.len()).filter(|&line| self.len(line) > 0);
        let rows = filled().map(baseline);
        let row = if down {
            rows.filter(|&row| row > here + ROW_PT)
                .min_by(f64::total_cmp)
        } else {
            rows.filter(|&row| row < here - ROW_PT)
                .max_by(f64::total_cmp)
        }?;
        filled()
            .filter(|&line| (baseline(line) - row).abs() < ROW_PT)
            .min_by(|&a, &b| self.off(a, want_pt).total_cmp(&self.off(b, want_pt)))
    }

    /// How far across a line is from a place, none when the place is on it.
    fn off(&self, line: usize, x_pt: f64) -> f64 {
        let spans = &self.lines[line].spans;
        match (spans.first(), spans.last()) {
            (Some(&(from, _)), Some(&(_, to))) => (from - x_pt).max(x_pt - to).max(0.0),
            _ => f64::INFINITY,
        }
    }

    /// f and F: the `count`th of a character on along the line from a
    /// place, or back, never past the line's ends.
    fn letter_from(&self, from: Pos, letter: char, on: bool, count: usize) -> Option<Pos> {
        let chars = &self.chars[from.line];
        let found = |&ch: &usize| chars[ch] == letter;
        let ch = if on {
            (from.ch + 1..chars.len()).filter(found).nth(count - 1)
        } else {
            (0..from.ch).rev().filter(found).nth(count - 1)
        }?;
        Some(Pos { ch, ..from })
    }

    /// w: the next word's start, a line's end counting as a blank.
    fn word_on(&self, from: Pos) -> Option<Pos> {
        let kind = self.class_at(from);
        let mut at = from;
        loop {
            let next = self.next(at)?;
            let crossed = next.line != at.line;
            at = next;
            if crossed || self.class_at(at) != kind {
                break;
            }
        }
        while self.class_at(at) == Class::Blank {
            at = self.next(at)?;
        }
        Some(at)
    }

    /// e: the end of this word, or the next one's from its end.
    fn word_end(&self, from: Pos) -> Option<Pos> {
        let mut at = self.next(from)?;
        while self.class_at(at) == Class::Blank {
            at = self.next(at)?;
        }
        let kind = self.class_at(at);
        while let Some(next) = self.next(at) {
            if next.line != at.line || self.class_at(next) != kind {
                break;
            }
            at = next;
        }
        Some(at)
    }

    /// b: the start of this word, or the one before's from its start.
    fn word_back(&self, from: Pos) -> Option<Pos> {
        let mut at = self.previous(from)?;
        while self.class_at(at) == Class::Blank {
            at = self.previous(at)?;
        }
        let kind = self.class_at(at);
        while let Some(before) = self.previous(at) {
            if before.line != at.line || self.class_at(before) != kind {
                break;
            }
            at = before;
        }
        Some(at)
    }

    /// Which paragraph a line is in: its piece's, or its own for a line no
    /// piece holds.
    pub(super) fn paragraph_key(&self, line: usize) -> (bool, usize) {
        match self.of_line[line] {
            Some(unit) => (true, self.paragraph[unit]),
            None => (false, line),
        }
    }

    /// The first line of the paragraph a line is in.
    pub(super) fn paragraph_start(&self, line: usize) -> usize {
        let key = self.paragraph_key(line);
        (0..=line)
            .rev()
            .take_while(|&l| self.paragraph_key(l) == key)
            .last()
            .unwrap_or(line)
    }

    /// }: the next paragraph's start, or the page's end. A line no piece
    /// holds, as a block of code's language over it, is passed over where
    /// a piece's line follows.
    fn paragraph_on(&self, from: Pos) -> Option<Pos> {
        let key = self.paragraph_key(from.line);
        let starts = |line: &usize| self.paragraph_key(*line) != key && self.len(*line) > 0;
        let next = (from.line + 1..self.lines.len())
            .filter(starts)
            .find(|&line| self.of_line[line].is_some())
            .or_else(|| (from.line + 1..self.lines.len()).find(starts));
        match next {
            Some(line) => Some(Pos { line, ch: 0 }),
            None => self.end().filter(|end| *end != from),
        }
    }

    /// {: this paragraph's start, or the one before's from its start.
    fn paragraph_back(&self, from: Pos) -> Option<Pos> {
        let start = self.paragraph_start(from.line);
        if from.ch > 0 || from.line != start {
            return Some(Pos { line: start, ch: 0 });
        }
        let before = (0..start)
            .rev()
            .find(|&line| self.len(line) > 0 && self.of_line[line].is_some())
            .or_else(|| (0..start).rev().find(|&line| self.len(line) > 0))?;
        Some(Pos {
            line: self.paragraph_start(before),
            ch: 0,
        })
    }
}
