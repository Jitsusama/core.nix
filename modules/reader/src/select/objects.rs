//! Text objects, as Vim's iw, aw, is, ap and the rest take them: the word,
//! sentence, paragraph or code a selection's cursor is in.

use std::ops::Range;

use super::{Class, Kind, Layout, Object, Pos, Selection, Taken, class};

impl Layout {
    /// Selects a text object round the cursor.
    pub fn object(&self, selection: &mut Selection, object: Object) {
        let at = selection.cursor;
        if self.len(at.line) == 0 {
            return;
        }
        let taken = match object {
            Object::InWord | Object::AWord => Some(self.word(at, object == Object::AWord)),
            Object::InSentence | Object::ASentence => {
                self.sentence(at, object == Object::ASentence)
            }
            Object::InCode | Object::ACode => {
                let Some((from, to, bytes)) = self.code_at(at, object == Object::ACode) else {
                    return;
                };
                selection.kind = Kind::Chars;
                selection.anchor = from;
                selection.cursor = to;
                selection.want_pt = None;
                selection.taken = Some(Taken::Bytes(bytes));
                return;
            }
            Object::InParagraph | Object::AParagraph => {
                self.paragraph(selection, at, object == Object::AParagraph);
                return;
            }
        };
        if let Some((from, to)) = taken {
            selection.kind = Kind::Chars;
            selection.anchor = from;
            selection.cursor = to;
            selection.want_pt = None;
            selection.taken = None;
        }
    }

    /// iw and aw: the run of one class round the cursor on its line, and
    /// for aw the blanks after it, or before it where none follow.
    fn word(&self, at: Pos, around: bool) -> (Pos, Pos) {
        let line = &self.chars[at.line];
        let run = |ch: usize| -> (usize, usize) {
            let kind = class(line[ch]);
            let mut from = ch;
            while from > 0 && class(line[from - 1]) == kind {
                from -= 1;
            }
            let mut to = ch;
            while to + 1 < line.len() && class(line[to + 1]) == kind {
                to += 1;
            }
            (from, to)
        };
        let (mut from, mut to) = run(at.ch);
        if around {
            if class(line[at.ch]) == Class::Blank {
                if to + 1 < line.len() {
                    to = run(to + 1).1;
                }
            } else if to + 1 < line.len() && class(line[to + 1]) == Class::Blank {
                to = run(to + 1).1;
            } else if from > 0 && class(line[from - 1]) == Class::Blank {
                from = run(from - 1).0;
            }
        }
        (
            Pos {
                line: at.line,
                ch: from,
            },
            Pos {
                line: at.line,
                ch: to,
            },
        )
    }

    /// The places of the piece a line is in, in reading order: its lines,
    /// or the line alone where no piece holds it.
    fn piece_places(&self, line: usize) -> Vec<Pos> {
        let (first, last) = self.of_line[line]
            .and_then(|unit| self.unit_lines[unit])
            .unwrap_or((line, line));
        (first..=last)
            .filter(|&l| self.of_line[l] == self.of_line[line])
            .flat_map(|l| (0..self.len(l)).map(move |ch| Pos { line: l, ch }))
            .collect()
    }

    /// is and as: the sentence round the cursor in its piece, from its
    /// first word to its stop, and for as the blanks after it.
    fn sentence(&self, at: Pos, around: bool) -> Option<(Pos, Pos)> {
        let places = self.piece_places(at.line);
        let here = places.iter().position(|p| *p == at)?;
        let c = |i: usize| self.at(places[i]);
        // Where a sentence ends: a stop, any closing marks after it, then
        // a blank or a line's end.
        let ends = |i: usize| {
            if !matches!(c(i), '.' | '!' | '?') {
                return None;
            }
            let mut end = i;
            while end + 1 < places.len()
                && matches!(c(end + 1), ')' | ']' | '"' | '\'' | '\u{201d}' | '\u{2019}')
            {
                end += 1;
            }
            let followed = end + 1 == places.len()
                || c(end + 1).is_whitespace()
                || places[end + 1].line != places[end].line;
            followed.then_some(end)
        };
        let mut start = 0;
        let mut i = 0;
        while i < here {
            if let Some(end) = ends(i)
                && end < here
            {
                start = end + 1;
                i = end + 1;
            } else {
                i += 1;
            }
        }
        while start < here && c(start).is_whitespace() {
            start += 1;
        }
        let mut stop = places.len() - 1;
        for i in here..places.len() {
            if let Some(end) = ends(i) {
                stop = end;
                break;
            }
        }
        if around {
            while stop + 1 < places.len()
                && c(stop + 1).is_whitespace()
                && places[stop + 1].line == places[stop].line
            {
                stop += 1;
            }
        }
        Some((places[start], places[stop]))
    }

    /// `` i` ``: the words of the code span the cursor is in, and the
    /// file's bytes of them. `` a` `` takes its backticks too, and as
    /// Vim's does, the blanks after it, or before it where none follow.
    fn code_at(&self, at: Pos, around: bool) -> Option<(Pos, Pos, Range<usize>)> {
        let unit = self.of_line[at.line]?;
        let matched = self.matched(unit);
        let byte = matched.iter().find(|(pos, _)| *pos == at)?.1;
        let (inside, whole) = self
            .code
            .iter()
            .find(|(inside, _)| inside.contains(&byte))?;
        let shown: Vec<Pos> = matched
            .iter()
            .filter(|(_, byte)| inside.contains(byte))
            .map(|(pos, _)| *pos)
            .collect();
        let (mut from, mut to) = (*shown.first()?, *shown.last()?);
        if !around {
            return Some((from, to, inside.clone()));
        }
        let bytes = self.source.as_bytes();
        let blank = |b: &u8| *b == b' ' || *b == b'\t';
        let (mut start, mut end) = (whole.start, whole.end);
        let after = bytes[end..].iter().take_while(|b| blank(b)).count();
        if after > 0 {
            end += after;
            while self.chars[to.line]
                .get(to.ch + 1)
                .is_some_and(|c| c.is_whitespace())
            {
                to.ch += 1;
            }
        } else {
            start -= bytes[..start].iter().rev().take_while(|b| blank(b)).count();
            while from.ch > 0 && self.chars[from.line][from.ch - 1].is_whitespace() {
                from.ch -= 1;
            }
        }
        Some((from, to, start..end))
    }

    /// ip and ap: the paragraph the cursor is in, by its pieces, and for ap
    /// the blank lines after it in the file, or before it where none
    /// follow.
    fn paragraph(&self, selection: &mut Selection, at: Pos, around: bool) {
        let key = self.paragraph_key(at.line);
        let start = self.paragraph_start(at.line);
        let end = (at.line..self.lines.len())
            .take_while(|&line| self.paragraph_key(line) == key)
            .last()
            .unwrap_or(at.line);
        selection.kind = Kind::Pieces;
        selection.anchor = Pos { line: start, ch: 0 };
        selection.cursor = Pos { line: end, ch: 0 };
        selection.want_pt = None;
        let taken = self.file_lines(start, end).map(|(mut first, mut last)| {
            // The lines either side that aren't blank and no piece holds,
            // a fence or a table's rule, are the paragraph's too.
            while first > 1 && !self.blank(first - 1) {
                first -= 1;
            }
            while last < self.source_lines.len() && !self.blank(last + 1) {
                last += 1;
            }
            if around {
                let after = last;
                while last < self.source_lines.len() && self.blank(last + 1) {
                    last += 1;
                }
                if last == after {
                    while first > 1 && self.blank(first - 1) {
                        first -= 1;
                    }
                }
            }
            (first, last)
        });
        selection.taken = taken.map(|(first, last)| Taken::Lines(first, last));
    }

    /// Whether a line of the file, from 1, is blank.
    fn blank(&self, line: usize) -> bool {
        self.source_lines
            .get(line.wrapping_sub(1))
            .is_none_or(|range| self.source[range.clone()].trim().is_empty())
    }
}
