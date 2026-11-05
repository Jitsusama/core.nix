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

use std::ops::Range;

use typst::foundations::Value;
use typst::introspection::{MetadataElem, Tag};
use typst::layout::{Frame, FrameItem, Point};

use crate::text::{Line, Piece};

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

/// Each piece of a selection as the ground drawn behind it: its top and
/// foot down the page and its ends across, in points. A line's ground is
/// its words' room, as search's marks are, reaching down to the next
/// line's where the two lines are a paragraph's, so a selection across
/// them is one ground, as a terminal's is, and apart across a gap
/// between paragraphs.
pub fn grounds(pieces: &[Piece]) -> Vec<(f64, f64, f64, f64)> {
    pieces
        .iter()
        .enumerate()
        .map(|(i, piece)| {
            let top = piece.baseline_pt - piece.size_pt * 0.92;
            let mut foot = top + piece.size_pt * 1.2;
            if let Some(next) = pieces.get(i + 1) {
                let next_top = next.baseline_pt - next.size_pt * 0.92;
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

    /// Lines `first` to `last` of the file, from 1, `usize::MAX` its last,
    /// taken whole as :y takes a range: the page's lines set from them,
    /// for the flash, and the lines themselves for copying, blank ones and
    /// all, with whether the page shows any of them. None for a range past
    /// the file's end, as Neovim's E16.
    pub fn file_range(&self, first: usize, last: usize) -> Option<(Selection, bool)> {
        // A file's last newline ends its last line rather than starting one.
        let count = self.source_lines.len() - usize::from(self.source.ends_with('\n'));
        let last = if last == usize::MAX { count } else { last };
        if first == 0 || first > last || last > count {
            return None;
        }
        let shown = |unit: &usize| {
            let Unit {
                first: from,
                last: to,
                ..
            } = self.units[*unit];
            from <= last && to >= first
        };
        let lines: Vec<usize> = (0..self.lines.len())
            .filter(|&line| self.of_line[line].as_ref().is_some_and(shown))
            .collect();
        let at = |line: Option<&usize>| Pos {
            line: line.copied().unwrap_or(0),
            ch: 0,
        };
        let mut selection = Selection::new(Kind::Pieces, at(lines.first()));
        selection.cursor = at(lines.last());
        selection.taken = Some(Taken::Lines(first, last));
        Some((selection, !lines.is_empty()))
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

    /// The lines of the file a selection took, from 1, as Neovim's '< and
    /// '> keep them once visual mode ends.
    pub fn file_lines_of(&self, selection: &Selection) -> Option<(usize, usize)> {
        if let Some(Taken::Lines(first, last)) = selection.taken {
            return Some((first, last));
        }
        let (first, last) = self.covered_lines(selection);
        self.file_lines(first, last)
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

    /// Moves the cursor `count` times.
    pub fn moved(&self, selection: &mut Selection, motion: Motion, count: usize) {
        let mut at = selection.cursor;
        let vertical = matches!(motion, Motion::Down | Motion::Up);
        if vertical {
            let want = selection.want_pt.unwrap_or_else(|| self.across(at));
            selection.want_pt = Some(want);
        } else {
            selection.want_pt = None;
        }
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
                Motion::Down => (at.line + 1..self.lines.len())
                    .find(|&line| self.len(line) > 0)
                    .map(|line| Pos { line, ch: 0 }),
                Motion::Up => (0..at.line)
                    .rev()
                    .find(|&line| self.len(line) > 0)
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
    fn paragraph_key(&self, line: usize) -> (bool, usize) {
        match self.of_line[line] {
            Some(unit) => (true, self.paragraph[unit]),
            None => (false, line),
        }
    }

    /// The first line of the paragraph a line is in.
    fn paragraph_start(&self, line: usize) -> usize {
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

    /// The file's lines the pieces on the page's lines `first` to `last`
    /// were written on.
    fn file_lines(&self, first: usize, last: usize) -> Option<(usize, usize)> {
        let units = (first..=last).filter_map(|line| self.of_line[line]);
        let mut found: Option<(usize, usize)> = None;
        for unit in units {
            let Unit { first, last, .. } = self.units[unit];
            let span = found.get_or_insert((first, last));
            span.0 = span.0.min(first);
            span.1 = span.1.max(last);
        }
        found
    }

    /// The page's lines a selection covers: a selection of pieces from its
    /// first piece's first line to its last's last.
    fn covered_lines(&self, selection: &Selection) -> (usize, usize) {
        let (from, to) = selection.ends();
        if selection.kind == Kind::Chars {
            return (from.line, to.line);
        }
        let first = self.of_line[from.line]
            .and_then(|unit| self.unit_lines[unit])
            .map_or(from.line, |(first, _)| first.min(from.line));
        let last = self.of_line[to.line]
            .and_then(|unit| self.unit_lines[unit])
            .map_or(to.line, |(_, last)| last.max(to.line));
        (first, last)
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
            from_pt: line.spans[from].0,
            to_pt: line.spans[to].1,
        }
    }

    /// The characters of a piece matched to the file's bytes, in order:
    /// each character the page sets to the next byte of the file's words
    /// that spells it, within the piece's lines. One the file doesn't
    /// spell, a list's bullet or an emoji's short name, matches nothing.
    fn matched(&self, unit: usize) -> Vec<(Pos, usize)> {
        let Some((first, last)) = self.unit_lines[unit] else {
            return Vec::new();
        };
        let Unit {
            first: from,
            last: to,
            ..
        } = self.units[unit];
        let (Some(start), Some(end)) = (
            self.source_lines.get(from.wrapping_sub(1)),
            self.source_lines.get(to.wrapping_sub(1)),
        ) else {
            return Vec::new();
        };
        let bytes = self.source.as_bytes();
        let (start, end) = (start.start, end.end);
        let mut at = start;
        let mut found = Vec::new();
        for line in first..=last {
            if self.of_line[line] != Some(unit) {
                continue;
            }
            for (ch, &c) in self.chars[line].iter().enumerate() {
                if c.is_whitespace() || c == BREAK {
                    continue;
                }
                let mut spelled = [0; 4];
                let spelled = c.encode_utf8(&mut spelled).as_bytes();
                let hit = (at..end)
                    .find(|&byte| self.words[byte] && bytes[byte..end].starts_with(spelled));
                if let Some(byte) = hit {
                    found.push((Pos { line, ch }, byte));
                    at = byte + spelled.len();
                }
            }
        }
        found
    }

    /// What a selection copies: the file's own markdown for what it covers.
    /// A selection of pieces copies the lines they were written on, whole,
    /// each ending with its newline, as Neovim's V does. A selection of
    /// characters copies the bytes from its first character matched to the
    /// file to its last, and the markup round a span whose words it holds
    /// whole. Words the file doesn't hold, the eyebrow over the title, are
    /// copied as the page sets them.
    pub fn copied(&self, selection: &Selection) -> String {
        let (from, to) = selection.ends();
        if selection.kind == Kind::Pieces {
            let (first, last) = self.covered_lines(selection);
            let taken = match selection.taken {
                Some(Taken::Lines(first, last)) => Some((first, last)),
                _ => self.file_lines(first, last),
            };
            let Some((start, end)) = taken else {
                return self.page_text(from, to);
            };
            let mut out = String::new();
            let lines = start.max(1) - 1..end.min(self.source_lines.len());
            for range in &self.source_lines[lines] {
                out.push_str(&self.source[range.clone()]);
                out.push('\n');
            }
            return out;
        }
        if let Some(Taken::Bytes(bytes)) = &selection.taken {
            return self.source[bytes.clone()].to_string();
        }
        let within = |pos: &Pos| *pos >= from && *pos <= to;
        let mut units: Vec<usize> = (from.line..=to.line)
            .filter_map(|line| self.of_line[line])
            .collect();
        units.dedup();
        let first = units
            .iter()
            .find_map(|&unit| self.matched(unit).into_iter().find(|(pos, _)| within(pos)));
        let last = units.iter().rev().find_map(|&unit| {
            self.matched(unit)
                .into_iter()
                .rev()
                .find(|(pos, _)| within(pos))
        });
        let (Some((start_at, start)), Some((end_at, end))) = (first, last) else {
            return self.page_text(from, to);
        };
        let end = end + self.at(end_at).len_utf8();
        let (mut start, mut end) = (start, end);
        // A span whose words are all taken comes whole, its markup too;
        // spans hold spans, so this goes on until none grows it.
        loop {
            let grown = self.spans.iter().find(|(whole, words)| {
                words.start >= start && words.end <= end && (whole.start < start || whole.end > end)
            });
            match grown {
                Some((whole, _)) => {
                    start = start.min(whole.start);
                    end = end.max(whole.end);
                }
                None => break,
            }
        }
        // A blank the selection ends on past its words, as aw and as take,
        // is the file's blanks there too, as Neovim yanks them.
        let blank_at = |pos: Pos| {
            self.chars[pos.line]
                .get(pos.ch)
                .is_some_and(|c| c.is_whitespace())
        };
        let bytes = self.source.as_bytes();
        let blank = |b: &&u8| **b == b' ' || **b == b'\t';
        if to > end_at && blank_at(to) {
            end += bytes[end..].iter().take_while(blank).count();
        }
        if from < start_at && blank_at(from) {
            start -= bytes[..start].iter().rev().take_while(blank).count();
        }
        self.source[start..end].to_string()
    }

    /// The words between two places as the page sets them: a line each
    /// where the file breaks the line, and joined where only the page
    /// wrapped it.
    fn page_text(&self, from: Pos, to: Pos) -> String {
        let mut out = String::new();
        for line in from.line..=to.line {
            let chars = &self.chars[line];
            let start = if line == from.line { from.ch } else { 0 };
            let end = if line == to.line {
                (to.ch + 1).min(chars.len())
            } else {
                chars.len()
            };
            if line > from.line {
                out.push_str(self.between(line - 1, line));
            }
            out.extend(chars[start.min(end)..end].iter().filter(|c| **c != BREAK));
        }
        out
    }

    /// What goes between two page lines in their words: a line break where
    /// the file has one, and where the page wrapped a line of the file, the
    /// blank the wrap took, if the page left none at the first line's end.
    fn between(&self, above: usize, below: usize) -> &'static str {
        let Some(unit) = self.of_line[above].filter(|&unit| self.of_line[below] == Some(unit))
        else {
            return "\n";
        };
        let matched = self.matched(unit);
        let last = matched.iter().rev().find(|(pos, _)| pos.line == above);
        let first = matched.iter().find(|(pos, _)| pos.line == below);
        let (Some(&(_, last)), Some(&(_, first))) = (last, first) else {
            return "\n";
        };
        let past = last + self.source[last..].chars().next().map_or(1, char::len_utf8);
        let gap = self.source.get(past..first).unwrap_or("\n");
        if gap.contains('\n') {
            "\n"
        } else if gap.contains(char::is_whitespace)
            && !self.chars[above].last().is_some_and(|c| c.is_whitespace())
        {
            " "
        } else {
            ""
        }
    }
}

/// The file's words, its spans of markup and its code spans, by bytes, as
/// comrak reads the file.
type Markup = (
    Vec<bool>,
    Vec<(Range<usize>, Range<usize>)>,
    Vec<(Range<usize>, Range<usize>)>,
);

fn markup(source: &str, lines: &[Range<usize>]) -> Markup {
    use comrak::nodes::NodeValue;
    let mut words = vec![false; source.len()];
    let mut spans = Vec::new();
    let mut code = Vec::new();
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, source, &crate::walk::options());
    // A place comrak gives, a line and a byte across it from 1, as a byte
    // of the file; an end is its character's last byte, so it's taken past
    // it to the next character's start.
    let byte = |line: usize, column: usize| -> Option<usize> {
        let range = lines.get(line.checked_sub(1)?)?;
        Some((range.start + column.checked_sub(1)?).min(source.len()))
    };
    let past = |at: usize| {
        let mut at = (at + 1).min(source.len());
        while !source.is_char_boundary(at) {
            at += 1;
        }
        at
    };
    let mut nodes: Vec<(Range<usize>, bool)> = Vec::new();
    for node in root.descendants() {
        let data = node.data();
        let place = data.sourcepos;
        let (Some(start), Some(end)) = (
            byte(place.start.line, place.start.column),
            byte(place.end.line, place.end.column),
        ) else {
            continue;
        };
        let whole = start..past(end).max(start);
        match &data.value {
            NodeValue::Text(_) | NodeValue::Math(_) | NodeValue::ShortCode(_) => {
                words[whole].fill(true);
            }
            NodeValue::Code(span) => {
                let ticks = span.num_backticks.min(whole.len() / 2);
                let inside = whole.start + ticks..whole.end - ticks;
                words[inside.clone()].fill(true);
                code.push((inside, whole.clone()));
                nodes.push((whole, true));
            }
            NodeValue::CodeBlock(_) => {
                for line in place.start.line..=place.end.line {
                    let Some(range) = lines.get(line.wrapping_sub(1)) else {
                        continue;
                    };
                    let text = source[range.clone()].trim_start();
                    if !(text.starts_with("```") || text.starts_with("~~~")) {
                        words[range.clone()].fill(true);
                    }
                }
            }
            NodeValue::Emph
            | NodeValue::Strong
            | NodeValue::Strikethrough
            | NodeValue::Highlight
            | NodeValue::Insert
            | NodeValue::Superscript
            | NodeValue::Subscript
            | NodeValue::Underline
            | NodeValue::SpoileredText
            | NodeValue::Link(_)
            | NodeValue::Image(_)
            | NodeValue::WikiLink(_) => nodes.push((whole, false)),
            _ => {}
        }
    }
    for (whole, _) in nodes {
        let first = (whole.start..whole.end).find(|&b| words[b]);
        let last = (whole.start..whole.end).rev().find(|&b| words[b]);
        if let (Some(first), Some(last)) = (first, last) {
            spans.push((whole, first..last + 1));
        }
    }
    (words, spans, code)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line of the page at `baseline`, its characters 6 pt wide each.
    #[expect(clippy::cast_precision_loss, reason = "a line's few characters")]
    fn line(baseline: f64, text: &str) -> Line {
        let spans = (0..text.chars().count())
            .map(|i| (i as f64 * 6.0, (i + 1) as f64 * 6.0))
            .collect();
        Line {
            x_pt: 0.0,
            tail_x_pt: 0.0,
            baseline_pt: baseline,
            size_pt: 13.0,
            text: text.to_string(),
            spans,
        }
    }

    /// The same, set in from the page's edge by `x_pt`, as a list's item
    /// is from its bullet.
    fn indented(baseline: f64, x_pt: f64, text: &str) -> Line {
        let mut set = line(baseline, text);
        set.x_pt = x_pt;
        set.tail_x_pt = x_pt;
        for span in &mut set.spans {
            *span = (span.0 + x_pt, span.1 + x_pt);
        }
        set
    }

    #[test]
    fn a_list_is_marked_by_its_bullets_and_numbers_alone() {
        assert!(marker("\u{2022}"));
        assert!(marker("4."));
        assert!(marker("b)"));
        assert!(!marker("4 items."));
        assert!(!marker(""));
        assert!(listed("- first item"));
        assert!(listed("  12. twelfth"));
        assert!(listed("> * quoted"));
        assert!(!listed("| - | one |"));
        assert!(!listed("-5 degrees"));
    }

    #[test]
    fn a_lines_words_are_the_files_in_any_case() {
        assert!(holds_its_words("| Name | Size |", "NAME"));
        assert!(holds_its_words("A **bold** start", "A bold start,"));
        assert!(!holds_its_words("echo one", "SH"));
    }

    fn unit(top_pt: f64, first: usize, last: usize) -> Unit {
        Unit {
            top_pt,
            first,
            last,
        }
    }

    /// A page of a title, a paragraph of two lines, a list of two items
    /// and a line of code, as the reader sets this file.
    const FILE: &str = "# The Title\n\nSome **bold words** and a\n[link](https://example.com) here. Then more.\n\n- first item\n- second `code` item\n\n```\nlet x = 1;\n```\n";

    fn page() -> Layout {
        let lines = vec![
            line(20.0, "The Title"),
            line(60.0, "Some bold words and a"),
            line(81.0, "link here. Then more."),
            line(120.0, "\u{2022}"),
            indented(120.0, 12.0, "first item"),
            line(141.0, "\u{2022}"),
            indented(141.0, 12.0, "second code item"),
            line(180.0, "let x = 1;"),
        ];
        let units = vec![
            unit(20.0, 1, 1),
            unit(60.0, 3, 4),
            unit(120.0, 6, 6),
            unit(141.0, 7, 7),
            unit(180.0, 10, 10),
        ];
        Layout::new(lines, units, &[5.0, 50.0, 110.0, 170.0], FILE.to_string())
    }

    fn at(line: usize, ch: usize) -> Pos {
        Pos { line, ch }
    }

    fn chars(page: &Layout, from: Pos, to: Pos) -> String {
        let mut selection = Selection::new(Kind::Chars, from);
        selection.cursor = to;
        page.copied(&selection)
    }

    #[test]
    fn each_line_is_the_piece_marked_last_above_it() {
        let page = page();
        assert_eq!(
            page.of_line,
            [Some(0), Some(1), Some(1), Some(2), Some(3), Some(4)],
            "a bullet is the margin's, no line of its item's"
        );
        assert_eq!(
            page.paragraph,
            [0, 1, 2, 2, 3],
            "the list's items are one paragraph"
        );
    }

    #[test]
    fn a_line_a_block_starts_with_that_no_piece_marks_is_no_pieces() {
        // An alert's kind, over its words, below the paragraph before it.
        let lines = vec![
            line(20.0, "Before."),
            line(60.0, "NOTE"),
            line(81.0, "Noted."),
        ];
        let units = vec![unit(20.0, 1, 1), unit(81.0, 4, 4)];
        let page = Layout::new(
            lines,
            units,
            &[5.0, 50.0],
            "Before.\n\n> [!NOTE]\n> Noted.\n".into(),
        );
        assert_eq!(page.of_line, [Some(0), None, Some(1)]);
    }

    #[test]
    fn v_starts_on_the_first_word_wholly_in_view() {
        let page = page();
        assert_eq!(page.first_in_view(0.0, 400.0), Some(at(0, 0)));
        assert_eq!(
            page.first_in_view(10.0, 400.0),
            Some(at(1, 0)),
            "the title's top is cut"
        );
        assert_eq!(
            page.first_in_view(100.0, 400.0),
            Some(at(3, 0)),
            "a bullet is no word"
        );
    }

    #[test]
    fn the_letters_and_arrows_move_by_characters_and_lines_keeping_across() {
        let page = page();
        let mut s = Selection::new(Kind::Chars, at(1, 12));
        page.moved(&mut s, Motion::Down, 1);
        assert_eq!(s.cursor, at(2, 12));
        page.moved(&mut s, Motion::Down, 1);
        assert_eq!(
            s.cursor,
            at(3, 9),
            "past the bullet, which is no line, to the item's end"
        );
        page.moved(&mut s, Motion::Down, 1);
        assert_eq!(s.cursor, at(4, 10), "back across to where it was");
        page.moved(&mut s, Motion::Left, 3);
        assert_eq!(s.cursor, at(4, 7));
        page.moved(&mut s, Motion::Right, 99);
        assert_eq!(s.cursor, at(4, 15), "l stops at the line's end");
        page.moved(&mut s, Motion::LineStart, 1);
        assert_eq!(
            s.cursor,
            at(4, 0),
            "the item's first letter, not its bullet"
        );
        page.moved(&mut s, Motion::LineEnd, 1);
        assert_eq!(s.cursor, at(4, 15));
        page.moved(&mut s, Motion::Up, 3);
        assert_eq!(s.cursor, at(1, 17));
    }

    #[test]
    fn w_b_and_e_go_by_words_as_vi_does() {
        let page = page();
        let mut s = Selection::new(Kind::Chars, at(1, 0));
        page.moved(&mut s, Motion::WordOn, 1);
        assert_eq!(s.cursor, at(1, 5));
        page.moved(&mut s, Motion::WordEnd, 1);
        assert_eq!(s.cursor, at(1, 8));
        page.moved(&mut s, Motion::WordOn, 4);
        assert_eq!(s.cursor, at(2, 0), "a line's end is a blank between words");
        page.moved(&mut s, Motion::WordOn, 2);
        assert_eq!(s.cursor, at(2, 9), "punctuation is a word of its own");
        page.moved(&mut s, Motion::WordBack, 3);
        assert_eq!(s.cursor, at(1, 20));
    }

    #[test]
    fn braces_go_by_paragraphs() {
        let page = page();
        let mut s = Selection::new(Kind::Chars, at(1, 4));
        page.moved(&mut s, Motion::ParagraphOn, 1);
        assert_eq!(s.cursor, at(3, 0));
        page.moved(&mut s, Motion::ParagraphOn, 1);
        assert_eq!(s.cursor, at(5, 0), "both items are one paragraph");
        page.moved(&mut s, Motion::ParagraphBack, 1);
        assert_eq!(s.cursor, at(3, 0));
        page.moved(&mut s, Motion::ParagraphBack, 2);
        assert_eq!(s.cursor, at(0, 0));
    }

    #[test]
    fn a_word_and_a_sentence_are_selected_round_the_cursor() {
        let page = page();
        let mut s = Selection::new(Kind::Chars, at(1, 6));
        page.object(&mut s, Object::InWord);
        assert_eq!(s.ends(), (at(1, 5), at(1, 8)));
        page.object(&mut s, Object::AWord);
        assert_eq!(s.ends(), (at(1, 5), at(1, 9)));
        let mut s = Selection::new(Kind::Chars, at(1, 6));
        page.object(&mut s, Object::InSentence);
        assert_eq!(
            s.ends(),
            (at(1, 0), at(2, 9)),
            "a sentence goes across lines"
        );
        let mut s = Selection::new(Kind::Chars, at(2, 13));
        page.object(&mut s, Object::ASentence);
        assert_eq!(s.ends(), (at(2, 11), at(2, 20)));
    }

    #[test]
    fn characters_copy_the_files_bytes_and_a_spans_markup_only_when_its_words_are_whole() {
        let page = page();
        assert_eq!(chars(&page, at(1, 5), at(1, 14)), "**bold words**");
        assert_eq!(
            chars(&page, at(1, 5), at(1, 8)),
            "bold",
            "half a span takes none of its markup"
        );
        assert_eq!(
            chars(&page, at(2, 0), at(2, 3)),
            "[link](https://example.com)"
        );
        assert_eq!(
            chars(&page, at(1, 16), at(2, 3)),
            "and a\n[link](https://example.com)",
            "across lines, the file's own break"
        );
        assert_eq!(chars(&page, at(4, 7), at(4, 10)), "`code`");
        assert_eq!(
            chars(&page, at(3, 0), at(4, 5)),
            "first item\n- second",
            "a bullet the file spells its own way"
        );
    }

    #[test]
    fn i_backtick_selects_a_code_spans_words() {
        let page = page();
        let mut s = Selection::new(Kind::Chars, at(4, 8));
        page.object(&mut s, Object::InCode);
        assert_eq!(s.ends(), (at(4, 7), at(4, 10)));
        assert_eq!(page.copied(&s), "code", "inside, as Vim's i` takes it");
        let mut elsewhere = Selection::new(Kind::Chars, at(3, 2));
        page.object(&mut elsewhere, Object::InCode);
        assert_eq!(
            elsewhere.ends(),
            (at(3, 2), at(3, 2)),
            "outside code, nothing"
        );
    }

    #[test]
    fn a_backtick_takes_the_ticks_and_the_blank_after_them() {
        let page = page();
        let mut s = Selection::new(Kind::Chars, at(4, 9));
        page.object(&mut s, Object::ACode);
        assert_eq!(s.ends(), (at(4, 7), at(4, 11)), "the page has no ticks");
        assert_eq!(page.copied(&s), "`code` ");
        s.put(at(4, 13));
        assert_eq!(
            page.copied(&s),
            "`code` it",
            "moved, it copies what it covers"
        );
        let at_end = Layout::new(
            vec![line(20.0, "Say code")],
            vec![unit(20.0, 1, 1)],
            &[5.0],
            "Say `code`\n".into(),
        );
        let mut s = Selection::new(Kind::Chars, at(0, 5));
        at_end.object(&mut s, Object::ACode);
        assert_eq!(s.ends(), (at(0, 3), at(0, 7)), "none after, so before");
        assert_eq!(at_end.copied(&s), " `code`");
    }

    #[test]
    fn a_line_the_page_wraps_is_copied_and_shown_as_one() {
        let page = Layout::new(
            vec![
                line(20.0, "Some words wrap "),
                line(41.0, "here, and hy\u{ad}"),
                line(62.0, "phens too."),
                line(100.0, "Its own line."),
                line(121.0, "And another."),
            ],
            vec![unit(20.0, 1, 1), unit(100.0, 3, 4)],
            &[5.0, 90.0],
            "Some words wrap here, and hyphens too.\n\nIts own line.\nAnd another.\n".into(),
        );
        let mut s = Selection::new(Kind::Chars, at(0, 5));
        s.cursor = at(2, 4);
        assert_eq!(page.copied(&s), "words wrap here, and hyphens");
        assert_eq!(page.shown(&s), "words wrap here, and hyphens");
        s.cursor = at(4, 2);
        assert_eq!(
            page.copied(&s),
            "words wrap here, and hyphens too.\n\nIts own line.\nAnd",
            "the file's own breaks stay"
        );
        assert_eq!(
            page.shown(&s),
            "words wrap here, and hyphens too.\nIts own line.\nAnd"
        );
    }

    #[test]
    fn pieces_copy_the_lines_they_were_written_on() {
        let page = page();
        let mut s = Selection::new(Kind::Pieces, at(2, 4));
        assert_eq!(
            page.copied(&s),
            "Some **bold words** and a\n[link](https://example.com) here. Then more.\n"
        );
        assert_eq!(
            page.pieces(&s).len(),
            2,
            "both of the paragraph's lines are drawn"
        );
        page.moved(&mut s, Motion::Down, 1);
        assert_eq!(
            page.copied(&s),
            "Some **bold words** and a\n[link](https://example.com) here. Then more.\n\n- first item\n"
        );
        let code = page.pieces_from(4, 1).expect("the code's line");
        assert_eq!(page.copied(&code), "let x = 1;\n");
    }

    #[test]
    fn a_range_of_the_files_lines_is_taken_whole_and_kept_after_a_selection() {
        let page = page();
        let (whole, shown) = page.file_range(1, usize::MAX).expect("every line");
        assert!(shown);
        assert_eq!(page.copied(&whole), FILE, "as :%y takes it");
        assert_eq!(
            page.pieces(&whole).len(),
            page.lines.len(),
            "all of it flashes"
        );
        let (list, _) = page.file_range(5, 7).expect("the list");
        assert_eq!(page.copied(&list), "\n- first item\n- second `code` item\n");
        assert_eq!(
            page.pieces(&list).len(),
            2,
            "the items flash, not the blank line"
        );
        let (blank, shown) = page.file_range(2, 2).expect("a blank line");
        assert!(!shown, "nothing to flash");
        assert_eq!(page.copied(&blank), "\n");
        assert!(page.file_range(0, 1).is_none());
        assert!(page.file_range(10, 12).is_none(), "past the end");
        assert!(page.file_range(11, 11).is_some(), "the last line");
        let mut s = Selection::new(Kind::Chars, at(1, 2));
        page.moved(&mut s, Motion::Down, 1);
        assert_eq!(page.file_lines_of(&s), Some((3, 4)));
        page.object(&mut s, Object::AParagraph);
        assert_eq!(page.file_lines_of(&s), Some((3, 5)), "its blank line too");
    }

    #[test]
    fn g_lists_each_piece_by_its_first_matching_line_and_a_mark_by_its_line() {
        let page = page();
        let every = |query| page.matching(query, 1, usize::MAX);
        let more = every("more");
        assert_eq!(
            more,
            [(page.line_extent(2).0, "link here. Then more.".to_string())]
        );
        let items: Vec<String> = every("item").into_iter().map(|(_, w)| w).collect();
        assert_eq!(items, ["first item", "second code item"]);
        assert!(every("Item").is_empty(), "a capital matches its case");
        assert_eq!(page.matching("item", 7, 7).len(), 1, "within its range");
        assert_eq!(
            page.words_at(page.line_extent(1).0),
            Some("Some bold words and a")
        );
        assert_eq!(
            page.words_at(page.line_extent(1).0 + 1.0),
            Some("link here. Then more.")
        );
        assert_eq!(page.words_at(10_000.0), None);
    }

    #[test]
    fn ip_takes_the_paragraph_and_ap_the_blank_lines_after_it() {
        let page = page();
        let mut s = Selection::new(Kind::Chars, at(4, 2));
        page.object(&mut s, Object::InParagraph);
        assert_eq!(page.copied(&s), "- first item\n- second `code` item\n");
        assert_eq!(
            page.pieces(&s).len(),
            2,
            "both items' words, their bullets the margin's and left bare"
        );
        page.object(&mut s, Object::AParagraph);
        assert_eq!(page.copied(&s), "- first item\n- second `code` item\n\n");
        let code = page
            .paragraph_of(4, Object::InParagraph)
            .expect("the code's paragraph");
        assert_eq!(
            page.copied(&code),
            "```\nlet x = 1;\n```\n",
            "its fences with it"
        );
        let list = page
            .paragraph_of(3, Object::AParagraph)
            .expect("the list's paragraph");
        assert_eq!(
            page.copied(&list),
            "- first item\n- second `code` item\n\n",
            "y a p takes the blank line after it"
        );
    }

    #[test]
    fn y_y_takes_the_piece_the_cursors_line_is_in() {
        let page = page();
        assert_eq!(page.piece_at(7.0, 400.0), Some(0), "the title's line");
        assert_eq!(
            page.piece_at(68.0, 400.0),
            Some(1),
            "the paragraph's second line is the paragraph's"
        );
        assert_eq!(page.piece_at(128.0, 400.0), Some(3), "the second item");
        assert_eq!(
            page.piece_at(0.0, 400.0),
            Some(0),
            "over the first piece, as an eyebrow is, the first below"
        );
        let two = page.pieces_from(2, 2).expect("two items");
        assert_eq!(page.copied(&two), "- first item\n- second `code` item\n");
    }

    #[test]
    fn y_brace_takes_the_pieces_to_the_paragraphs_end() {
        let page = page();
        let rest = page.to_paragraph_end(2, 1).expect("the list");
        assert_eq!(page.copied(&rest), "- first item\n- second `code` item\n");
        let from_second = page.to_paragraph_end(3, 1).expect("its second item");
        assert_eq!(page.copied(&from_second), "- second `code` item\n");
        let two = page.to_paragraph_end(1, 2).expect("two paragraphs");
        assert_eq!(
            page.copied(&two),
            "Some **bold words** and a\n[link](https://example.com) here. Then more.\n\n- first item\n- second `code` item\n"
        );
        let past = page.to_paragraph_end(4, 9).expect("the last paragraph");
        assert_eq!(
            page.copied(&past),
            "let x = 1;\n",
            "a count past the end stops there"
        );
    }

    #[test]
    fn o_goes_to_the_other_end() {
        let mut s = Selection::new(Kind::Chars, at(1, 2));
        s.cursor = at(2, 4);
        s.swap();
        assert_eq!((s.anchor, s.cursor), (at(2, 4), at(1, 2)));
    }

    #[test]
    fn words_the_file_doesnt_hold_copy_as_the_page_sets_them() {
        let lines = vec![line(10.0, "NOTES \u{b7} TODAY")];
        let page = Layout::new(lines, Vec::new(), &[], String::new());
        assert_eq!(chars(&page, at(0, 0), at(0, 4)), "NOTES");
    }

    /// The file set as the reader sets it, its words read for selecting.
    fn set(name: &str, markdown: &str) -> Layout {
        let dir = std::env::temp_dir().join(format!("reader-select-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let file = dir.join("page.md");
        std::fs::write(&file, markdown).expect("the file");
        let (page, _) = crate::page::Setter::new(file.to_str().expect("a path"))
            .set()
            .expect("set");
        std::fs::remove_dir_all(&dir).expect("cleaned");
        Layout::read(&page.frame, markdown.to_string())
    }

    /// The page's line holding `words`, past the eyebrow, which names the
    /// file's folder by the test's process and the time it was written,
    /// so holds a 12 when either does.
    fn holding(page: &Layout, words: &str) -> usize {
        page.lines
            .iter()
            .position(|line| !line.text.starts_with("READER-SELECT-") && line.text.contains(words))
            .unwrap_or_else(|| panic!("no line holds {words}"))
    }

    #[test]
    fn a_set_pages_lines_are_the_pieces_they_were_written_in() {
        let markdown = "# A Title\n\nThe lede.\n\n> [!NOTE]\n> Noted **words** here.\n\n- One item\n- Two items\n\n```sh\necho one\necho two\n```\n\nThe last words.\n";
        let page = set("pieces", markdown);
        let piece =
            |words: &str| page.of_line[holding(&page, words)].map(|unit| page.units[unit].first);
        assert_eq!(piece("A Title"), Some(1));
        assert_eq!(piece("The lede."), Some(3));
        assert_eq!(piece("NOTE"), None, "an alert's kind is no piece's");
        assert_eq!(piece("Noted"), Some(6));
        assert_eq!(piece("One item"), Some(8));
        assert_eq!(piece("Two items"), Some(9));
        assert!(
            !page.lines.iter().any(|line| marker(&line.text)),
            "a list's bullets are the margin's, no line to select"
        );
        let starts =
            |page: &Layout, words: &str| page.lines[holding(page, words)].text.starts_with(words);
        assert!(starts(&page, "One item"), "nor the start of one");
        let numbered = set("numbers", "1. One\n2. Two\n10. Ten\n\n- [ ] A task\n");
        assert!(
            !numbered.lines.iter().any(|line| marker(&line.text)),
            "nor are its numbers or a task's box"
        );
        for words in ["One", "Two", "Ten", "A task"] {
            assert!(
                starts(&numbered, words),
                "{words}'s line is {:?}, not its words alone",
                numbered.lines[holding(&numbered, words)].text
            );
        }
        assert_eq!(piece("echo one"), Some(12));
        assert!(
            !page.lines.iter().any(|line| line.text.contains("SH")),
            "the code's language, beside its first line, is no line to select"
        );
        let table = set("cells", "| Name | Size |\n|---|---|\n| one | 12 |\n");
        let cell =
            |words: &str| table.of_line[holding(&table, words)].map(|u| table.units[u].first);
        assert_eq!(
            cell("NAME"),
            Some(1),
            "a head's cells beside each other are its own"
        );
        assert_eq!(cell("SIZE"), Some(1));
        assert_eq!(cell("12"), Some(3));
        assert_eq!(piece("echo two"), Some(13));
        assert_eq!(piece("The last"), Some(16));
        assert_eq!(
            page.first_in_view(0.0, 1000.0),
            Some(Pos {
                line: holding(&page, "A Title"),
                ch: 0
            }),
            "v starts on the title, not the eyebrow over it"
        );
        // Copying a line of words by its characters takes its markup whole.
        let noted = holding(&page, "Noted");
        let mut s = Selection::new(Kind::Chars, Pos { line: noted, ch: 0 });
        page.moved(&mut s, Motion::LineEnd, 1);
        assert_eq!(page.copied(&s), "Noted **words** here.");
        // The code's paragraph is its fences and lines.
        let code = page.of_line[holding(&page, "echo two")].expect("a piece");
        let whole = page
            .paragraph_of(code, Object::InParagraph)
            .expect("its paragraph");
        assert_eq!(page.copied(&whole), "```sh\necho one\necho two\n```\n");
        assert_eq!(page.pieces(&whole).len(), 2, "both lines of code show it");
    }

    #[test]
    fn a_search_match_is_found_by_where_it_starts() {
        let page = page();
        assert_eq!(page.place_of(60.0, 30.0), Some(at(1, 5)));
        assert_eq!(page.place_of(61.0, 30.0), None);
    }
}
