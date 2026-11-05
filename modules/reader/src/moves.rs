//! Where vi's moves land on a page, kept apart from the window so they can
//! be tested: the lines j and k go by, paragraphs and headings, the lines
//! in view H, M and L pick, the view zt, zz and zb place, a bracket's
//! other end, the word * looks for, a source line's place, and the
//! markdown files beside this one.
//!
//! The cursor is a line of the page, as a help buffer's is: the view's top
//! line until a key puts it elsewhere, and kept within the view as it
//! scrolls, as Neovim's is. The window marks it with a rule in the margin.
use std::path::{Path, PathBuf};

use crate::place::Block;
use crate::text::Line;

/// Two places closer than this, in points, are one: the view's top after
/// a move to a line is that line's top, give or take rounding.
const SAME_PT: f64 = 0.5;

/// Where a line's top is, in points down the page: where the view's top
/// goes to show it first, as a link to a heading lands.
pub fn top_of(line: &Line) -> f64 {
    line.baseline_pt - line.size_pt
}

/// Where a line's foot is, its descenders' room in, as a match's is.
pub fn foot_of(line: &Line) -> f64 {
    line.baseline_pt + line.size_pt * 0.3
}

/// Where a block's words start: its first line's top, or the block's own
/// top where it has none, as a picture or a rule.
fn landing(blocks: &[Block], lines: &[Line], at: usize) -> f64 {
    let top = blocks[at].top_pt;
    let next = blocks
        .get(at + 1)
        .map_or(f64::INFINITY, |block| block.top_pt);
    lines
        .iter()
        .find(|line| line.baseline_pt >= top && line.baseline_pt < next)
        .map_or(top, top_of)
}

/// Whether a block starts a paragraph: the first does, and one with a blank
/// line in the file between it and the block before, as a heading with its
/// words on the next line is one paragraph to vi, and to a selection's {.
fn starts_paragraph(blocks: &[Block], at: usize) -> bool {
    let before = at.checked_sub(1).and_then(|before| blocks[before].lines);
    match (before, blocks[at].lines) {
        (Some((_, to)), Some((from, _))) => from > to + 1,
        _ => true,
    }
}

/// } and { for `times`, and ]] and [[ with `headings`: where the cursor's
/// line goes, `times` paragraphs or headings on from `at_pt` or back. Past the
/// last one it's the page's end, and before the first its top, as vi's go
/// to the file's ends.
pub fn block_from(
    blocks: &[Block],
    lines: &[Line],
    at_pt: f64,
    times: u32,
    on: bool,
    headings: bool,
) -> f64 {
    let mut stops: Vec<f64> = (0..blocks.len())
        .filter(|&at| {
            if headings {
                blocks[at].heading.is_some()
            } else {
                starts_paragraph(blocks, at)
            }
        })
        .map(|at| landing(blocks, lines, at))
        .collect();
    stops.dedup_by(|a, b| (*a - *b).abs() < SAME_PT);
    let times = times.max(1) as usize;
    // What's over the first block, as the eyebrow over a title, is no
    // block of its own, so a move on from it goes on from the first.
    let first = (!blocks.is_empty()).then(|| landing(blocks, lines, 0));
    let at_pt = match first {
        Some(first) if on => at_pt.max(first),
        _ => at_pt,
    };
    if on {
        stops
            .iter()
            .filter(|&&pt| pt > at_pt + SAME_PT)
            .nth(times - 1)
            .copied()
            .unwrap_or(f64::INFINITY)
    } else {
        stops
            .iter()
            .rev()
            .filter(|&&pt| pt < at_pt - SAME_PT)
            .nth(times - 1)
            .copied()
            .unwrap_or(0.0)
    }
}

/// The lines wholly in a view from `top_pt` to `foot_pt`, by their places
/// among the page's.
fn in_view(lines: &[Line], top_pt: f64, foot_pt: f64) -> Vec<usize> {
    // The lines come down the page by baseline, and a line whose top and
    // foot are in view has its baseline there too, so only the view's own
    // are looked at, however long the page: the window asks every frame.
    let from = lines.partition_point(|line| line.baseline_pt < top_pt - SAME_PT);
    let to = from + lines[from..].partition_point(|line| line.baseline_pt <= foot_pt);
    (from..to)
        .filter(|&i| top_of(&lines[i]) >= top_pt - SAME_PT && foot_of(&lines[i]) <= foot_pt)
        .collect()
}

/// The cursor's line: the line `aim_pt` is the top of while it's in view,
/// else the view's top line, or its bottom one when the cursor was left
/// below it, as Neovim keeps its cursor on the screen as it scrolls.
pub fn cursor(lines: &[Line], top_pt: f64, foot_pt: f64, aim_pt: Option<f64>) -> Option<usize> {
    let shown = in_view(lines, top_pt, foot_pt);
    let (&first, &last) = (shown.first()?, shown.last()?);
    let Some(aim) = aim_pt else {
        return Some(first);
    };
    if aim < top_of(&lines[first]) - SAME_PT {
        return Some(first);
    }
    if aim > top_of(&lines[last]) + SAME_PT {
        return Some(last);
    }
    shown.iter().copied().min_by(|&a, &b| {
        (top_of(&lines[a]) - aim)
            .abs()
            .total_cmp(&(top_of(&lines[b]) - aim).abs())
    })
}

/// j and k: the line `by` rows down from line `from`, or up for a negative
/// `by`, stopping at the page's first and last rows, as Neovim's gj and gk
/// go by the lines shown. Lines side by side on one baseline, a table's
/// cells or a bullet and its item, are one row, whose line is the leftmost.
pub fn line_by(lines: &[Line], from: usize, by: isize) -> usize {
    let starts =
        |i: usize| i == 0 || (lines[i].baseline_pt - lines[i - 1].baseline_pt).abs() >= SAME_PT;
    if lines.is_empty() {
        return from;
    }
    // Walked out from `from` a row at a time, rather than every row of the
    // page found first, so a step costs the lines it passes.
    let mut row = (0..=from.min(lines.len() - 1))
        .rev()
        .find(|&i| starts(i))
        .unwrap_or(0);
    for _ in 0..by.unsigned_abs() {
        let next = if by > 0 {
            (row + 1..lines.len()).find(|&i| starts(i))
        } else {
            (0..row).rev().find(|&i| starts(i))
        };
        match next {
            Some(next) => row = next,
            None => break,
        }
    }
    row
}

/// The cursor's line once the view has moved without it, as Ctrl+E and
/// Ctrl+F move it: the line `aim_pt` is the top of, while it shows between
/// `top_pt` and `foot_pt`, and else the nearest line there, so the cursor
/// keeps to the view as Neovim's keeps to it within its scrolloff.
pub fn kept_in(lines: &[Line], top_pt: f64, foot_pt: f64, aim_pt: f64) -> Option<usize> {
    let shown = in_view(lines, top_pt, foot_pt);
    shown.iter().copied().min_by(|&a, &b| {
        (top_of(&lines[a]) - aim_pt)
            .abs()
            .total_cmp(&(top_of(&lines[b]) - aim_pt).abs())
    })
}

/// Which line in view H, M or L picks: the `count`th from the top or the
/// bottom, or the one nearest the view's middle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InView {
    Top,
    Middle,
    Bottom,
}

pub fn line_in_view(
    lines: &[Line],
    top_pt: f64,
    foot_pt: f64,
    which: InView,
    count: u32,
) -> Option<usize> {
    let shown = in_view(lines, top_pt, foot_pt);
    let nth = (count.max(1) as usize - 1).min(shown.len().checked_sub(1)?);
    Some(match which {
        InView::Top => shown[nth],
        InView::Bottom => shown[shown.len() - 1 - nth],
        InView::Middle => {
            let middle = f64::midpoint(
                top_of(&lines[shown[0]]),
                foot_of(&lines[shown[shown.len() - 1]]),
            );
            shown.iter().copied().min_by(|&a, &b| {
                let mid = |i: usize| f64::midpoint(top_of(&lines[i]), foot_of(&lines[i]));
                (mid(a) - middle).abs().total_cmp(&(mid(b) - middle).abs())
            })?
        }
    })
}

/// Where the view's top goes to put a line at its top, middle or bottom,
/// as zt, zz and zb do, for a view `screen_pt` tall.
pub fn placed(line: &Line, screen_pt: f64, which: InView) -> f64 {
    match which {
        InView::Top => top_of(line),
        InView::Middle => f64::midpoint(top_of(line), foot_of(line)) - screen_pt / 2.0,
        InView::Bottom => foot_of(line) - screen_pt,
    }
}

/// N%: the line `percent` of the way down the page's lines, as vi rounds.
pub fn percent(lines: &[Line], percent: u32) -> Option<usize> {
    let count = lines.len();
    let nth = (percent.min(100) as usize * count).div_ceil(100);
    count
        .checked_sub(1)
        .map(|last| nth.saturating_sub(1).min(last))
}

const PAIRS: [(char, char); 3] = [('(', ')'), ('[', ']'), ('{', '}')];

/// %: the line holding the other end of the first bracket on line `from`,
/// counting the brackets nested between them, as vi's % does.
pub fn bracket(lines: &[Line], from: usize) -> Option<usize> {
    let line = lines.get(from)?;
    let (at, first) = line
        .text
        .chars()
        .enumerate()
        .find(|(_, c)| PAIRS.iter().any(|(open, close)| c == open || c == close))?;
    let (open, close, on) = PAIRS.iter().find_map(|&(open, close)| {
        (first == open)
            .then_some((open, close, true))
            .or((first == close).then_some((open, close, false)))
    })?;
    let mut depth = 0_i32;
    if on {
        for (i, line) in lines.iter().enumerate().skip(from) {
            let skip = if i == from { at } else { 0 };
            for c in line.text.chars().skip(skip) {
                depth += i32::from(c == open) - i32::from(c == close);
                if depth == 0 {
                    return Some(i);
                }
            }
        }
    } else {
        for (i, line) in lines.iter().enumerate().take(from + 1).rev() {
            let chars: Vec<char> = line.text.chars().collect();
            let end = if i == from { at + 1 } else { chars.len() };
            for &c in chars[..end].iter().rev() {
                depth += i32::from(c == close) - i32::from(c == open);
                if depth == 0 {
                    return Some(i);
                }
            }
        }
    }
    None
}

/// Whether a character is part of a word, as vi's 'iskeyword' has it.
fn wordy(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The first word on a line, which * and # look for.
pub fn first_word(line: &Line) -> Option<String> {
    let word: String = line
        .text
        .chars()
        .skip_while(|&c| !wordy(c))
        .take_while(|&c| wordy(c))
        .collect();
    (!word.is_empty()).then_some(word)
}

/// :{n}: where source line `n` is on the page, in points: as far into the
/// block written on it as the line is into the block's lines, or the top
/// of the first block after it, between blocks.
pub fn source_line(blocks: &[Block], n: usize) -> Option<f64> {
    let written: Vec<(usize, &Block)> = blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| block.lines.is_some())
        .collect();
    let (first, _) = written.first()?.1.lines?;
    if n < first {
        return Some(0.0);
    }
    let (at, block) = written
        .iter()
        .rev()
        .find(|(_, block)| block.lines.is_some_and(|(from, _)| from <= n))?;
    let (from, to) = block.lines?;
    if n > to {
        let next = written.iter().find(|(i, _)| i > at);
        return Some(next.map_or(f64::INFINITY, |(_, block)| block.top_pt));
    }
    let height = blocks
        .get(at + 1)
        .map_or(0.0, |next| next.top_pt - block.top_pt);
    #[expect(
        clippy::cast_precision_loss,
        reason = "a file's line numbers, far fewer than f64 holds exactly"
    )]
    let into = (n - from) as f64 / (to - from + 1) as f64;
    Some(block.top_pt + height * into)
}

/// The markdown files in a folder, by name, as ]b and [b go through them.
pub fn markdown_files(folder: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(folder)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| crate::links::markdown(path) && path.is_file())
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

/// ]b and [b: the file `times` on from `current` among `files`, or back,
/// round the folder's ends, as vi's :bnext goes round its buffers.
pub fn neighbour(files: &[PathBuf], current: &Path, times: u32, on: bool) -> Option<PathBuf> {
    let count = files.len();
    let at = files
        .iter()
        .position(|file| crate::links::same(file, current))?;
    if count < 2 {
        return None;
    }
    let times = times.max(1) as usize % count;
    let to = if on {
        (at + times) % count
    } else {
        (at + count - times) % count
    };
    (to != at).then(|| files[to].clone())
}

/// The paths a line writes, as gf reads a file's name under the cursor:
/// each run of the characters a path holds, its closing marks left off.
pub fn written_paths(line: &Line) -> Vec<String> {
    let path_char = |c: char| wordy(c) || "/.-~+".contains(c);
    line.text
        .split(|c: char| !path_char(c))
        .map(|word| word.trim_end_matches(['.', ',']))
        .filter(|word| word.contains('.') || word.contains('/'))
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "the places given, read back or added to by whole points"
)]
mod tests {
    use super::*;

    fn line(baseline_pt: f64, text: &str) -> Line {
        Line {
            x_pt: 0.0,
            tail_x_pt: 0.0,
            baseline_pt,
            size_pt: 10.0,
            cap_pt: 7.0,
            text: text.into(),
            spans: text.chars().map(|_| (0.0, 1.0)).collect(),
        }
    }

    fn block(number: usize, top_pt: f64, lines: (usize, usize), heading: bool) -> Block {
        Block {
            number,
            top_pt,
            lines: Some(lines),
            heading: heading.then(|| (2, format!("Heading {number}"))),
        }
    }

    /// A heading, two paragraphs, a heading and a paragraph, each a line
    /// at 20 pt pitch, its block starting 5 pt above its words' top.
    fn page() -> (Vec<Block>, Vec<Line>) {
        let blocks = vec![
            block(0, 5.0, (1, 1), true),
            block(1, 25.0, (3, 3), false),
            block(2, 45.0, (5, 6), false),
            block(3, 65.0, (8, 8), true),
            block(4, 85.0, (10, 10), false),
        ];
        let lines = (0..5)
            .map(|i| line(20.0 + 20.0 * f64::from(i), &format!("line {i}")))
            .collect();
        (blocks, lines)
    }

    #[test]
    fn a_paragraph_on_is_the_next_blocks_words_and_back_the_one_before() {
        let (blocks, lines) = page();
        assert_eq!(block_from(&blocks, &lines, 10.0, 1, true, false), 30.0);
        assert_eq!(block_from(&blocks, &lines, 30.0, 2, true, false), 70.0);
        assert_eq!(block_from(&blocks, &lines, 70.0, 1, false, false), 50.0);
    }

    #[test]
    fn past_the_last_paragraph_is_the_end_and_before_the_first_the_top() {
        let (blocks, lines) = page();
        assert_eq!(
            block_from(&blocks, &lines, 90.0, 1, true, false),
            f64::INFINITY
        );
        assert_eq!(block_from(&blocks, &lines, 10.0, 1, false, false), 0.0);
    }

    #[test]
    fn a_move_on_from_over_the_first_block_goes_on_from_the_first() {
        let (blocks, mut lines) = page();
        lines.insert(0, line(4.0, "eyebrow"));
        assert_eq!(block_from(&blocks, &lines, -6.0, 1, true, false), 30.0);
        assert_eq!(block_from(&blocks, &lines, -6.0, 1, true, true), 70.0);
        assert_eq!(block_from(&blocks, &lines, 10.0, 1, false, false), 0.0);
    }

    #[test]
    fn blocks_with_no_blank_line_between_are_one_paragraph() {
        let (mut blocks, lines) = page();
        // The first paragraph's words are on the line after the heading's.
        blocks[1].lines = Some((2, 2));
        assert_eq!(block_from(&blocks, &lines, 10.0, 1, true, false), 50.0);
        assert_eq!(block_from(&blocks, &lines, 50.0, 1, false, false), 10.0);
        assert_eq!(block_from(&blocks, &lines, 10.0, 1, true, true), 70.0);
    }

    #[test]
    fn headings_go_only_to_headings_with_a_count() {
        let (blocks, lines) = page();
        assert_eq!(block_from(&blocks, &lines, 10.0, 1, true, true), 70.0);
        assert_eq!(block_from(&blocks, &lines, 90.0, 1, false, true), 70.0);
        assert_eq!(block_from(&blocks, &lines, 90.0, 2, false, true), 10.0);
    }

    #[test]
    fn the_cursor_is_the_top_line_until_aimed_and_stays_in_view() {
        let (_, lines) = page();
        assert_eq!(cursor(&lines, 10.0, 60.0, None), Some(0));
        assert_eq!(cursor(&lines, 10.0, 60.0, Some(30.0)), Some(1));
        assert_eq!(
            cursor(&lines, 30.0, 80.0, Some(10.0)),
            Some(1),
            "left above"
        );
        assert_eq!(
            cursor(&lines, 10.0, 60.0, Some(90.0)),
            Some(1),
            "left below"
        );
    }

    #[test]
    fn j_and_k_go_by_the_rows_shown_and_stop_at_the_pages_ends() {
        let (_, lines) = page();
        assert_eq!(line_by(&lines, 0, 1), 1);
        assert_eq!(line_by(&lines, 1, 3), 4);
        assert_eq!(line_by(&lines, 3, -2), 1);
        assert_eq!(line_by(&lines, 3, 9), 4, "past the last row is the last");
        assert_eq!(line_by(&lines, 1, -9), 0, "before the first is the first");
    }

    #[test]
    fn lines_on_one_baseline_are_one_row_for_j_and_k() {
        let lines = vec![
            line(20.0, "title"),
            line(40.0, "\u{2022}"),
            line(40.0, "an item"),
            line(60.0, "after"),
        ];
        assert_eq!(line_by(&lines, 0, 1), 1, "the row's leftmost line");
        assert_eq!(line_by(&lines, 1, 1), 3);
        assert_eq!(line_by(&lines, 2, 1), 3, "from either of the row's lines");
        assert_eq!(line_by(&lines, 3, -1), 1);
    }

    #[test]
    fn the_cursor_kept_in_a_moved_view_is_the_nearest_line_there() {
        let (_, lines) = page();
        assert_eq!(kept_in(&lines, 30.0, 80.0, 10.0), Some(1), "left above");
        assert_eq!(kept_in(&lines, 10.0, 60.0, 90.0), Some(1), "left below");
        assert_eq!(kept_in(&lines, 10.0, 100.0, 50.0), Some(2), "still shown");
        assert_eq!(kept_in(&lines, 200.0, 300.0, 50.0), None);
    }

    #[test]
    fn h_m_and_l_pick_the_top_middle_and_bottom_lines_with_counts() {
        let (_, lines) = page();
        let pick = |which, count| line_in_view(&lines, 10.0, 100.0, which, count);
        assert_eq!(pick(InView::Top, 1), Some(0));
        assert_eq!(pick(InView::Top, 2), Some(1));
        assert_eq!(pick(InView::Bottom, 1), Some(3));
        assert_eq!(
            pick(InView::Bottom, 9),
            Some(0),
            "a count past the view stops at it"
        );
        assert_eq!(
            pick(InView::Middle, 1),
            Some(1),
            "of two as near, the upper"
        );
    }

    #[test]
    fn zt_zz_and_zb_place_the_line_at_the_views_top_middle_and_foot() {
        let one = line(50.0, "here");
        assert_eq!(placed(&one, 100.0, InView::Top), 40.0);
        assert_eq!(placed(&one, 100.0, InView::Middle), 46.5 - 50.0);
        assert_eq!(placed(&one, 100.0, InView::Bottom), 53.0 - 100.0);
    }

    #[test]
    fn percent_goes_that_far_through_the_lines_as_vi_rounds() {
        let (_, lines) = page();
        assert_eq!(percent(&lines, 50), Some(2));
        assert_eq!(percent(&lines, 100), Some(4));
        assert_eq!(percent(&lines, 1), Some(0));
        assert_eq!(percent(&[], 50), None);
    }

    #[test]
    fn a_bracket_finds_its_other_end_across_lines_past_those_nested() {
        let lines = vec![
            line(10.0, "fn main() {"),
            line(20.0, "    if a { b(); }"),
            line(30.0, "}"),
        ];
        assert_eq!(
            bracket(&lines, 0),
            Some(0),
            "the first bracket is the call's"
        );
        let lines = vec![
            line(10.0, "main {"),
            line(20.0, "    if a { b(); }"),
            line(30.0, "}"),
        ];
        assert_eq!(bracket(&lines, 0), Some(2));
        assert_eq!(bracket(&lines, 2), Some(0), "and back from the closing one");
        assert_eq!(bracket(&[line(10.0, "no brackets")], 0), None);
    }

    #[test]
    fn the_word_here_is_the_lines_first_whole_word() {
        assert_eq!(
            first_word(&line(0.0, "  (Paragraph 12) says")).as_deref(),
            Some("Paragraph")
        );
        assert_eq!(first_word(&line(0.0, "— …")), None);
    }

    #[test]
    fn a_source_line_is_as_far_into_its_block_as_into_its_lines() {
        let (blocks, _) = page();
        assert_eq!(source_line(&blocks, 3), Some(25.0));
        assert_eq!(
            source_line(&blocks, 6),
            Some(55.0),
            "halfway into lines 5 and 6"
        );
        assert_eq!(
            source_line(&blocks, 7),
            Some(65.0),
            "a blank line is the next block's"
        );
        assert_eq!(source_line(&blocks, 99), Some(f64::INFINITY));
    }

    #[test]
    fn the_next_file_goes_round_the_folder_by_name() {
        let files: Vec<PathBuf> = ["a.md", "b.md", "c.md"].iter().map(PathBuf::from).collect();
        let to = |from: &str, times, on| neighbour(&files, Path::new(from), times, on);
        assert_eq!(to("b.md", 1, true), Some(PathBuf::from("c.md")));
        assert_eq!(to("c.md", 1, true), Some(PathBuf::from("a.md")));
        assert_eq!(to("a.md", 1, false), Some(PathBuf::from("c.md")));
        assert_eq!(to("a.md", 3, true), None, "all the way round is here");
        assert_eq!(neighbour(&files[..1], Path::new("a.md"), 1, true), None);
    }

    #[test]
    fn a_written_path_is_read_as_gf_reads_one() {
        let said = written_paths(&line(0.0, "See docs/design.md, or `notes.md`."));
        assert_eq!(said, ["docs/design.md", "notes.md"]);
    }
}
