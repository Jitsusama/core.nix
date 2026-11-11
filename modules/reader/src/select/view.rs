//! What the window shows of a selection: its lines' extents, keeping its
//! cursor in view, and what it covers on each line, to draw it.

use super::{Class, Kind, Layout, Pos, Selection, class};
use crate::text::Piece;

impl Layout {
    /// Where a line's top and foot are, in points down the page.
    pub fn line_extent(&self, line: usize) -> (f64, f64) {
        let line = &self.lines[line];
        (
            line.baseline_pt - line.size_pt,
            line.baseline_pt + line.size_pt * 0.3,
        )
    }

    /// The first word wholly in a view from `top_pt` to `foot_pt`, where v
    /// starts, given the cursor's line's top for `top_pt`: the first letter
    /// or digit of the first line wholly in it that has one, so never a
    /// list's bullet.
    pub fn first_in_view(&self, top_pt: f64, foot_pt: f64) -> Option<Pos> {
        // The file's own words first: the eyebrow over a title, an alert's
        // kind and a caption are the page's, not the file's, so a copy from
        // them would be no markdown.
        let first = |written: bool| {
            (0..self.lines.len()).find_map(|line| {
                let (top, foot) = self.line_extent(line);
                if top < top_pt || foot > foot_pt || (written && self.of_line[line].is_none()) {
                    return None;
                }
                let ch = self.chars[line]
                    .iter()
                    .position(|c| class(*c) == Class::Word)?;
                Some(Pos { line, ch })
            })
        };
        first(true).or_else(|| first(false))
    }

    /// Keeps the cursor in a view from `top_pt` to `foot_pt`, when the view
    /// moved without it: to the line nearest it that shows whole, keeping
    /// across, as Neovim keeps its cursor on the screen.
    pub fn keep_in_view(&self, selection: &mut Selection, top_pt: f64, foot_pt: f64) {
        let at = selection.cursor;
        let (top, foot) = self.line_extent(at.line);
        if top >= top_pt && foot <= foot_pt {
            return;
        }
        let mut shown = (0..self.lines.len()).filter(|&line| {
            let (top, foot) = self.line_extent(line);
            self.len(line) > 0 && top >= top_pt && foot <= foot_pt
        });
        let line = if top < top_pt {
            shown.next()
        } else {
            shown.next_back()
        };
        if let Some(line) = line {
            let want = selection.want_pt.unwrap_or_else(|| self.across(at));
            selection.want_pt = Some(want);
            selection.cursor = Pos {
                line,
                ch: self.nearest(line, want),
            };
            selection.taken = None;
        }
    }

    /// Whether a line is the same on another page of the file: its words
    /// and where it sits, so a selection on it holds.
    #[expect(
        clippy::float_cmp,
        reason = "a line set the same way sits on the same baseline, copied, never computed"
    )]
    pub fn same_line(&self, other: &Layout, line: usize) -> bool {
        match (self.lines.get(line), other.lines.get(line)) {
            (Some(mine), Some(theirs)) => {
                mine.text == theirs.text && mine.baseline_pt == theirs.baseline_pt
            }
            _ => false,
        }
    }

    /// What a selection shows, as the page sets its words: for the key
    /// log, which says what each key selected.
    pub fn shown(&self, selection: &Selection) -> String {
        let (from, to) = selection.ends();
        match selection.kind {
            Kind::Chars => self.page_text(from, to),
            Kind::Pieces => {
                let (first, last) = self.covered_lines(selection);
                let end = Pos {
                    line: last,
                    ch: self.len(last),
                };
                self.page_text(Pos { line: first, ch: 0 }, end)
            }
        }
    }

    /// What a selection covers on each of its lines, to draw it.
    pub fn pieces(&self, selection: &Selection) -> Vec<Piece> {
        let (from, to) = selection.ends();
        let (first, last) = self.covered_lines(selection);
        (first..=last)
            .filter(|&line| self.len(line) > 0)
            .map(|line| {
                let spans = &self.lines[line].spans;
                let chars = selection.kind == Kind::Chars;
                let start = if chars && line == from.line {
                    from.ch
                } else {
                    0
                };
                let end = if chars && line == to.line {
                    to.ch.min(spans.len() - 1)
                } else {
                    spans.len() - 1
                };
                self.piece(line, start, end)
            })
            .collect()
    }

    /// The cursor, as the character it's on.
    pub fn cursor(&self, selection: &Selection) -> Option<Piece> {
        let at = selection.cursor;
        (at.ch < self.len(at.line)).then(|| self.piece(at.line, at.ch, at.ch))
    }

    fn piece(&self, line: usize, from: usize, to: usize) -> Piece {
        let line = &self.lines[line];
        Piece {
            baseline_pt: line.baseline_pt,
            size_pt: line.size_pt,
            cap_pt: line.cap_pt,
            from_pt: line.spans[from].0,
            to_pt: line.spans[to].1,
        }
    }
}
