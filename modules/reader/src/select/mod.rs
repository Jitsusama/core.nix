//! Visual mode, kept apart from the window so it can be tested: a
//! selection on the page's words, moved as Neovim moves one, and what it
//! copies, which is the file's own markdown for what it covers.
//!
//! A selection's cursor starts on the first word of the page's cursor's
//! line, or the first after it. It moves over the page's lines (text.rs), each character
//! a place. The walk marks each piece of the file that V takes whole, a
//! paragraph, a heading, a list's item, a table's row or a line of code,
//! with the lines it was written on (walk.rs), and those marks, read off
//! the page here, say which lines of the page are which piece. A piece's
//! characters are matched to the file's own in order, over the bytes
//! comrak says are words, so a selection of characters copies the bytes
//! between its ends, taking in a span's markup, `**` or a link's
//! address, only when the selection holds all of the span's words.

mod file;
mod motions;
mod objects;
mod view;

use std::ops::Range;

use typst::foundations::Value;
use typst::introspection::{MetadataElem, Tag};
use typst::layout::{Frame, FrameItem, Point};

use crate::text::{Line, Piece};

use file::markup;

/// A place among the page's words: a line, and a character on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pos {
    pub line: usize,
    pub ch: usize,
}

/// How a selection takes the page: by characters, as v, or by the pieces
/// of the file it touches, whole, as V.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Chars,
    Pieces,
}

/// A selection: where it was started, where its cursor is, and the
/// across a vertical move keeps to, as Neovim's curswant.
#[derive(Clone, Debug, PartialEq)]
pub struct Selection {
    pub kind: Kind,
    pub anchor: Pos,
    pub cursor: Pos,
    want_pt: Option<f64>,
    /// What of the file an object took, which copying takes as it is.
    taken: Option<Taken>,
}

/// What of the file an object took, beyond the words the page shows.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Taken {
    /// A paragraph's lines, blank lines and all.
    Lines(usize, usize),
    /// A code span's bytes: its words alone for i`, and for a` its
    /// backticks too, which the page doesn't show.
    Bytes(Range<usize>),
}

impl Selection {
    pub fn new(kind: Kind, at: Pos) -> Self {
        Selection {
            kind,
            anchor: at,
            cursor: at,
            want_pt: None,
            taken: None,
        }
    }

    /// Its two ends, the first first.
    pub fn ends(&self) -> (Pos, Pos) {
        (self.anchor.min(self.cursor), self.anchor.max(self.cursor))
    }

    /// o: the cursor to the other end.
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.anchor, &mut self.cursor);
        self.want_pt = None;
    }

    /// The cursor to a place, as a search's match puts it.
    pub fn put(&mut self, at: Pos) {
        self.cursor = at;
        self.want_pt = None;
        self.taken = None;
    }
}

/// The cell a line of the page stands in, its top and foot down the page
/// in points: the page's pitch tall, or the line's size and a fifth where
/// that is taller, as the title's is, centred on its capitals, so it
/// stands as far over them as under the baseline. A selection's ground
/// and the cursor are each line's cell, so a paragraph's lines, a pitch
/// apart, join into one ground as Vim's Visual does (VKCF3Z, 2026-11-05).
pub fn line_cell(baseline_pt: f64, cap_pt: f64, size_pt: f64) -> (f64, f64) {
    let tall = crate::page::pitch_pt().max(size_pt * 1.2);
    let top = baseline_pt - cap_pt - (tall - cap_pt) / 2.0;
    (top, top + tall)
}

/// Each piece of a selection as the ground drawn behind it: its top and
/// foot down the page and its ends across, in points. A line's ground is
/// its cell, reaching down to the next line's where the two are nearly
/// touching, as a larger line's beside a smaller one is, so a selection
/// across them is one ground, as a terminal's is, and apart across a gap
/// between paragraphs.
pub fn grounds(pieces: &[Piece]) -> Vec<(f64, f64, f64, f64)> {
    pieces
        .iter()
        .enumerate()
        .map(|(i, piece)| {
            let (top, mut foot) = line_cell(piece.baseline_pt, piece.cap_pt, piece.size_pt);
            if let Some(next) = pieces.get(i + 1) {
                let (next_top, _) = line_cell(next.baseline_pt, next.cap_pt, next.size_pt);
                if next_top > foot && next_top - foot < piece.size_pt * 0.5 {
                    foot = next_top;
                }
            }
            (top, foot, piece.from_pt, piece.to_pt)
        })
        .collect()
}

/// How the cursor moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    Left,
    Right,
    Down,
    Up,
    /// w, b and e.
    WordOn,
    WordBack,
    WordEnd,
    /// 0 and $.
    LineStart,
    LineEnd,
    /// { and }.
    ParagraphBack,
    ParagraphOn,
    /// gg and G.
    First,
    Last,
}

/// Whether the file's lines hold every word of a line of the page, in
/// any case, since a table's head and a label are set in capitals.
fn holds_its_words(written: &str, text: &str) -> bool {
    let written = written.to_lowercase();
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .all(|word| written.contains(&word.to_lowercase()))
}

/// A text object: what iw, aw, is, as, ip, ap, `` i` `` and `` a` ``
/// select.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Object {
    InWord,
    AWord,
    InSentence,
    ASentence,
    InParagraph,
    AParagraph,
    InCode,
    ACode,
}

/// A piece of the file the page sets as one, and where its mark is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Unit {
    pub top_pt: f64,
    /// The lines of the file it was written on, from 1.
    pub first: usize,
    pub last: usize,
}

/// The page's words for selecting, with the file they were set from.
pub struct Layout {
    pub lines: Vec<Line>,
    chars: Vec<Vec<char>>,
    units: Vec<Unit>,
    /// Each line's piece, by its place in `units`.
    of_line: Vec<Option<usize>>,
    /// Each piece's first and last line on the page, if it shows any.
    unit_lines: Vec<Option<(usize, usize)>>,
    /// Each piece's paragraph: pieces with no blank line between them in
    /// the file are one, as Neovim's paragraphs are.
    paragraph: Vec<usize>,
    source: String,
    /// Each line of the file, as bytes of it, without its newline.
    source_lines: Vec<Range<usize>>,
    /// Which bytes of the file are its words, rather than markup.
    words: Vec<bool>,
    /// Each span of markup round words, emphasis, a link or code: the
    /// bytes it takes and the first and last of its words.
    spans: Vec<(Range<usize>, Range<usize>)>,
    /// Each span of code: its words, and the bytes it takes with its
    /// backticks.
    code: Vec<(Range<usize>, Range<usize>)>,
}

/// Lines this close to a mark are under it: the mark sits on its first
/// line's baseline, or at the top of the block holding it.
const NEAR_PT: f64 = 0.5;

/// A soft hyphen, set where hyphenation broke a word.
const BREAK: char = '\u{ad}';

/// The marks a list's items are set with: bullets, a dash and a task's
/// boxes.
const MARKS: &str = "\u{2022}\u{25e6}\u{25aa}\u{2023}\u{2043}\u{2013}-*+\u{2610}\u{2611}\u{2612}";

/// Whether a line's words are a list's marker: a bullet or a task's box,
/// or a number or letter and the point or bracket after it.
fn marker(text: &str) -> bool {
    let text = text.trim();
    let numbered = text
        .strip_suffix(['.', ')'])
        .is_some_and(|n| !n.is_empty() && n.len() <= 4 && n.chars().all(char::is_alphanumeric));
    !text.is_empty() && (text.chars().all(|c| MARKS.contains(c)) || numbered)
}

/// The piece each line is of. A line is its piece's when the piece's mark
/// is the last above it and no block starts between the two: the line a
/// block starts with but no piece of it marks, an alert's kind or a
/// picture's caption, is no piece's.
fn pieces_of(lines: &[Line], units: &[Unit], starts: &[f64]) -> Vec<Option<usize>> {
    lines
        .iter()
        .map(|line| {
            let below = line.baseline_pt + NEAR_PT;
            let unit = units.iter().rposition(|unit| unit.top_pt <= below)?;
            let block = starts
                .iter()
                .copied()
                .filter(|top| *top <= below)
                .fold(f64::MIN, f64::max);
            (units[unit].top_pt + NEAR_PT >= block).then_some(unit)
        })
        .collect()
}

/// Where each of the file's lines is in it, by its bytes, without its
/// line's end.
fn ranges_of_lines(source: &str) -> Vec<std::ops::Range<usize>> {
    let mut at = 0;
    source
        .split('\n')
        .map(|line| {
            let range = at..at + line.len();
            at += line.len() + 1;
            range
        })
        .collect()
}

/// Takes an item's marker off its first line, and a task's box after its
/// bullet, where both are drawn.
fn unmark_item(line: &mut Line, chars: &mut Vec<char>) {
    if unmark(line, chars, marker) {
        unmark(line, chars, |mark| mark.chars().all(|c| MARKS.contains(c)));
    }
}

/// Takes a list's marker and the blank after it off the front of a line
/// of words, so the line starts where its words do.
fn unmark(line: &mut Line, chars: &mut Vec<char>, is_mark: impl Fn(&str) -> bool) -> bool {
    let Some(blank) = chars.iter().position(|c| c.is_whitespace()) else {
        return false;
    };
    let words = chars[blank..]
        .iter()
        .position(|c| !c.is_whitespace())
        .map_or(chars.len(), |at| blank + at);
    let mark: String = chars[..blank].iter().collect();
    if blank == 0 || words == chars.len() || !is_mark(&mark) || line.spans.len() != chars.len() {
        return false;
    }
    chars.drain(..words);
    line.spans.drain(..words);
    line.text = chars.iter().collect();
    line.x_pt = line.spans[0].0;
    line.tail_x_pt = line.tail_x_pt.max(line.x_pt);
    true
}

/// Whether a line of the file starts a list's item: a mark or a number,
/// then a blank.
fn listed(line: &str) -> bool {
    let line = line.trim_start().trim_start_matches('>').trim_start();
    let Some((mark, rest)) = line.split_once(' ') else {
        return false;
    };
    let numbered = mark
        .strip_suffix(['.', ')'])
        .is_some_and(|n| !n.is_empty() && n.len() <= 9 && n.chars().all(|c| c.is_ascii_digit()));
    !rest.is_empty() && (matches!(mark, "-" | "*" | "+") || numbered)
}

/// How vi tells words apart: blanks between them, letters and digits, and
/// everything else.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Blank,
    Word,
    Other,
}

fn class(c: char) -> Class {
    if c.is_whitespace() {
        Class::Blank
    } else if c.is_alphanumeric() || c == '_' || c == BREAK {
        Class::Word
    } else {
        Class::Other
    }
}

/// Every piece's mark on a page, top to bottom.
pub fn units(frame: &Frame) -> Vec<Unit> {
    fn walk(frame: &Frame, at: Point, found: &mut Vec<Unit>) {
        for (place, item) in frame.items() {
            let here = at + *place;
            match item {
                FrameItem::Group(group) => {
                    let moved = Point::new(group.transform.tx, group.transform.ty);
                    walk(&group.frame, here + moved, found);
                }
                FrameItem::Tag(Tag::Start(content, _)) => {
                    if let Some(metadata) = content.to_packed::<MetadataElem>()
                        && let Value::Str(said) = &metadata.value
                        && let Some((first, last)) = said
                            .strip_prefix("s ")
                            .and_then(|lines| lines.split_once('-'))
                        && let (Ok(first), Ok(last)) = (first.parse(), last.parse())
                    {
                        found.push(Unit {
                            top_pt: here.y.to_pt(),
                            first,
                            last,
                        });
                    }
                }
                _ => {}
            }
        }
    }
    let mut found = Vec::new();
    walk(frame, Point::zero(), &mut found);
    found.sort_by(|a, b| a.top_pt.total_cmp(&b.top_pt));
    found
}

impl Layout {
    /// The page's words, read off its frame, with the file it was set from.
    pub fn read(frame: &Frame, source: String) -> Self {
        let lines = crate::text::lines(crate::text::runs(frame));
        let starts: Vec<f64> = crate::place::starts(frame)
            .into_iter()
            .map(|(_, top)| top)
            .collect();
        Self::new(lines, units(frame), &starts, source)
    }

    /// The page's lines and the pieces' marks, each block's start, in
    /// points, and the file.
    pub fn new(mut lines: Vec<Line>, mut units: Vec<Unit>, starts: &[f64], source: String) -> Self {
        let mut chars: Vec<Vec<char>> = lines
            .iter()
            .map(|line| line.text.chars().collect())
            .collect();
        let mut of_line = pieces_of(&lines, &units, starts);
        let source_lines = ranges_of_lines(&source);
        let blank = |line: usize| {
            line == 0
                || source_lines
                    .get(line - 1)
                    .is_none_or(|range| source[range.clone()].trim().is_empty())
        };
        // An item's lines run on over the blank line after it, which is
        // the gap between it and what follows, not its own.
        for unit in &mut units {
            while unit.last > unit.first && blank(unit.last) {
                unit.last -= 1;
            }
        }
        // A label set beside a piece's line, as a block of code's language
        // is beside its first, is the page's, not the piece's: a line on
        // another of the piece's baselines whose words the piece's lines in
        // the file don't hold, as a table's cells beside each other do.
        let labels: Vec<usize> = (0..lines.len())
            .filter(|&line| {
                let Some(unit) = of_line[line] else {
                    return false;
                };
                let level = |other: usize| {
                    other != line
                        && of_line[other] == Some(unit)
                        && (lines[other].baseline_pt - lines[line].baseline_pt).abs()
                            < lines[line].size_pt * 0.5
                };
                let beside = (0..lines.len()).any(level);
                let before = (0..lines.len())
                    .any(|other| level(other) && lines[other].x_pt > lines[line].x_pt);
                let (first, last) = (units[unit].first, units[unit].last);
                let written = source_lines
                    .get(first.saturating_sub(1)..last.min(source_lines.len()))
                    .unwrap_or_default()
                    .iter()
                    .map(|range| &source[range.clone()])
                    .collect::<Vec<_>>()
                    .join("\n");
                let item = written.lines().next().is_some_and(listed);
                (beside && !holds_its_words(&written, &lines[line].text))
                    || (before && item && marker(&lines[line].text))
            })
            .collect();
        // Taken off the lines a selection goes over, since one standing
        // between a piece's lines would part the piece. A list's marker,
        // beside its item's first line, goes too: it's the margin's, as
        // Vim's number column is, so nothing stands on it or grounds it.
        // A marker set close enough to its words is read as their line's
        // start rather than a line of its own, so it comes off the front
        // of the item's first line instead, unless it stood apart.
        let mut seen = vec![false; units.len()];
        for &line in labels.iter().rev() {
            if let Some(unit) = of_line[line].filter(|_| marker(&lines[line].text)) {
                seen[unit] = true;
            }
            lines.remove(line);
            chars.remove(line);
            of_line.remove(line);
        }
        for line in 0..lines.len() {
            let Some(unit) = of_line[line] else {
                continue;
            };
            if !std::mem::replace(&mut seen[unit], true) {
                let first = source_lines
                    .get(units[unit].first.saturating_sub(1))
                    .map(|range| &source[range.clone()]);
                if first.is_some_and(listed) {
                    unmark_item(&mut lines[line], &mut chars[line]);
                }
            }
        }
        let mut unit_lines: Vec<Option<(usize, usize)>> = vec![None; units.len()];
        for (line, unit) in of_line.iter().enumerate() {
            if let Some(unit) = *unit {
                let span = unit_lines[unit].get_or_insert((line, line));
                span.1 = line;
            }
        }
        let mut paragraph = Vec::with_capacity(units.len());
        for (i, unit) in units.iter().enumerate() {
            let joins = i > 0 && {
                let before = &units[i - 1];
                unit.first <= before.last || !(before.last + 1..unit.first).any(blank)
            };
            let next = paragraph
                .last()
                .map_or(0, |p| if joins { *p } else { p + 1 });
            paragraph.push(next);
        }
        let (words, spans, code) = markup(&source, &source_lines);
        Layout {
            lines,
            chars,
            units,
            of_line,
            unit_lines,
            paragraph,
            source,
            source_lines,
            words,
            spans,
            code,
        }
    }

    fn len(&self, line: usize) -> usize {
        self.chars.get(line).map_or(0, Vec::len)
    }

    fn at(&self, pos: Pos) -> char {
        self.chars[pos.line].get(pos.ch).copied().unwrap_or(' ')
    }

    fn class_at(&self, pos: Pos) -> Class {
        class(self.at(pos))
    }

    fn next(&self, pos: Pos) -> Option<Pos> {
        if pos.ch + 1 < self.len(pos.line) {
            Some(Pos {
                line: pos.line,
                ch: pos.ch + 1,
            })
        } else {
            (pos.line + 1..self.lines.len())
                .find(|&line| self.len(line) > 0)
                .map(|line| Pos { line, ch: 0 })
        }
    }

    fn previous(&self, pos: Pos) -> Option<Pos> {
        if pos.ch > 0 {
            Some(Pos {
                line: pos.line,
                ch: pos.ch - 1,
            })
        } else {
            (0..pos.line)
                .rev()
                .find(|&line| self.len(line) > 0)
                .map(|line| Pos {
                    line,
                    ch: self.len(line) - 1,
                })
        }
    }

    /// The last place on the page.
    fn end(&self) -> Option<Pos> {
        (0..self.lines.len())
            .rev()
            .find(|&line| self.len(line) > 0)
            .map(|line| Pos {
                line,
                ch: self.len(line) - 1,
            })
    }

    /// The piece the cursor's line, whose top is at `top_pt`, is in, where
    /// y y, y i p, y a p and y } copy from; on a line no piece holds, an
    /// eyebrow or a caption, the first piece below it that shows whole
    /// above `foot_pt`.
    pub fn piece_at(&self, top_pt: f64, foot_pt: f64) -> Option<usize> {
        let here = (0..self.lines.len())
            .find(|&line| (self.line_extent(line).0 - top_pt).abs() < NEAR_PT)
            .and_then(|line| self.of_line[line]);
        here.or_else(|| {
            (0..self.units.len()).find(|&unit| {
                self.unit_lines[unit].is_some_and(|(first, _)| {
                    let (top, foot) = self.line_extent(first);
                    top >= top_pt - NEAR_PT && foot <= foot_pt
                })
            })
        })
    }

    /// `count` pieces from `unit` on, taken whole, as y y takes them.
    pub fn pieces_from(&self, unit: usize, count: usize) -> Option<Selection> {
        let shown: Vec<usize> = (unit..self.units.len())
            .filter(|&u| self.unit_lines[u].is_some())
            .take(count.max(1))
            .collect();
        let first = self.unit_lines[*shown.first()?]?.0;
        let last = self.unit_lines[*shown.last()?]?.1;
        let mut selection = Selection::new(Kind::Pieces, Pos { line: first, ch: 0 });
        selection.cursor = Pos { line: last, ch: 0 };
        Some(selection)
    }

    /// The paragraph a piece is in, taken whole, as y i p takes it, or with
    /// the blank line after it, as y a p does.
    pub fn paragraph_of(&self, unit: usize, object: Object) -> Option<Selection> {
        let line = self.unit_lines[unit]?.0;
        let mut selection = Selection::new(Kind::Pieces, Pos { line, ch: 0 });
        self.object(&mut selection, object);
        Some(selection)
    }

    /// From a piece to its paragraph's end, and `count` less one paragraphs
    /// on, taken whole, as y } takes the lines up to the blank one.
    pub fn to_paragraph_end(&self, unit: usize, count: usize) -> Option<Selection> {
        let first = self.unit_lines[unit]?.0;
        let last_paragraph = self.paragraph[unit] + count.max(1) - 1;
        let last = (unit..self.units.len())
            .filter(|&u| self.paragraph[u] <= last_paragraph)
            .filter_map(|u| self.unit_lines[u])
            .map(|(_, last)| last)
            .max()?;
        let mut selection = Selection::new(Kind::Pieces, Pos { line: first, ch: 0 });
        selection.cursor = Pos { line: last, ch: 0 };
        Some(selection)
    }

    /// Each piece of the file from line `from` to `to` that `query`
    /// matches, as :g/pat/ lists them: where the page's line holding its
    /// first match starts, and that line's words, matched as / matches
    /// them.
    #[expect(
        clippy::float_cmp,
        reason = "a match's baseline is its line's, copied, never computed"
    )]
    pub fn matching(&self, query: &str, from: usize, to: usize) -> Vec<(f64, String)> {
        self.unit_lines
            .iter()
            .zip(&self.units)
            .filter(|(_, unit)| unit.first <= to && unit.last >= from)
            .filter_map(|(lines, _)| {
                let (first, last) = (*lines)?;
                let found = crate::text::find_as(&self.lines[first..=last], query, true);
                let baseline = found.first()?.pieces.first()?.baseline_pt;
                let line = (first..=last).find(|&l| self.lines[l].baseline_pt == baseline)?;
                Some((self.line_extent(line).0, self.lines[line].text.clone()))
            })
            .collect()
    }

    /// The words of the page's line starting at `top_pt`, or the first
    /// after it, as :marks and :jumps say a place by its line.
    pub fn words_at(&self, top_pt: f64) -> Option<&str> {
        (0..self.lines.len())
            .find(|&line| self.line_extent(line).0 >= top_pt - 0.5)
            .map(|line| self.lines[line].text.as_str())
    }

    /// The character on a line nearest across to `x_pt`.
    fn nearest(&self, line: usize, x_pt: f64) -> usize {
        let spans = &self.lines[line].spans;
        spans
            .iter()
            .position(|(_, to)| x_pt < *to)
            .unwrap_or(spans.len().saturating_sub(1))
    }

    /// Where a character sits across the page, its middle.
    fn across(&self, pos: Pos) -> f64 {
        self.lines[pos.line]
            .spans
            .get(pos.ch)
            .map_or(0.0, |(from, to)| (from + to) / 2.0)
    }

    /// The place a character starting at `x_pt` on the line set on
    /// `baseline_pt` is, as a search's match says where it starts.
    #[expect(
        clippy::float_cmp,
        reason = "a match's baseline is its line's, copied, never computed"
    )]
    pub fn place_of(&self, baseline_pt: f64, x_pt: f64) -> Option<Pos> {
        let line = self.lines.iter().position(|line| {
            line.baseline_pt == baseline_pt
                && line.spans.first().is_some_and(|span| span.0 <= x_pt + 0.01)
                && line.spans.last().is_some_and(|span| span.1 > x_pt)
        })?;
        Some(Pos {
            line,
            ch: self.nearest(line, x_pt + 0.01),
        })
    }
}

#[cfg(test)]
mod tests;
