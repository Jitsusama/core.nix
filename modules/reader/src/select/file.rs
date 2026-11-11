//! The file side of a selection: the lines of the file it was written on,
//! its characters matched to the file's bytes, and the markdown it copies.

use std::ops::Range;

use super::{BREAK, Kind, Layout, Pos, Selection, Taken, Unit};

impl Layout {
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

    /// The lines of the file a selection took, from 1, as Neovim's '< and
    /// '> keep them once visual mode ends.
    pub fn file_lines_of(&self, selection: &Selection) -> Option<(usize, usize)> {
        if let Some(Taken::Lines(first, last)) = selection.taken {
            return Some((first, last));
        }
        let (first, last) = self.covered_lines(selection);
        self.file_lines(first, last)
    }

    /// The file's lines the pieces on the page's lines `first` to `last`
    /// were written on.
    pub(super) fn file_lines(&self, first: usize, last: usize) -> Option<(usize, usize)> {
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
    pub(super) fn covered_lines(&self, selection: &Selection) -> (usize, usize) {
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

    /// The characters of a piece matched to the file's bytes, in order:
    /// each character the page sets to the next byte of the file's words
    /// that spells it, within the piece's lines. One the file doesn't
    /// spell, a list's bullet or an emoji's short name, matches nothing.
    pub(super) fn matched(&self, unit: usize) -> Vec<(Pos, usize)> {
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
    pub(super) fn page_text(&self, from: Pos, to: Pos) -> String {
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
pub(super) type Markup = (
    Vec<bool>,
    Vec<(Range<usize>, Range<usize>)>,
    Vec<(Range<usize>, Range<usize>)>,
);

pub(super) fn markup(source: &str, lines: &[Range<usize>]) -> Markup {
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
