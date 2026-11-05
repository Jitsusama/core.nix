//! Which of the page's lines show above an overlay: only whole ones, so an
//! overlay's top never cuts a line through. The page is bared from the top
//! of the first line cut, as the drawer leaves it, so the overlay's frame
//! never moves as the page scrolls under it (VKCF3Z, 2026-11-05). Baring
//! from that line's top rather than the foot of the whole line over it
//! keeps the room between them shown, and with it a ground that hangs
//! under its block's last line, as code's does. A heading counts with its
//! first line: one whose next line won't show whole under it hides too, so
//! a heading never stands alone over an overlay.

use crate::place::Block;
use crate::select::line_cell;
use crate::text::Line;

/// A line of the page in view: its cell's top and foot down the screen,
/// and whether it is a heading's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shown {
    pub top: f64,
    pub foot: f64,
    pub heading: bool,
}

/// Each of a page's lines, top to bottom, as it shows with the page's top
/// at `offset` and `px` to the point: its cell, and whether it is a
/// heading's, as `headings` says.
pub fn shown(lines: &[Line], headings: &[bool], px: f64, offset: f64) -> Vec<Shown> {
    lines
        .iter()
        .zip(headings)
        .map(|(line, &heading)| {
            let (top, foot) = line_cell(line.baseline_pt, line.cap_pt, line.size_pt);
            Shown {
                top: top * px - offset,
                foot: foot * px - offset,
                heading,
            }
        })
        .collect()
}

/// Which of a page's lines, top to bottom, are a heading's: those whose
/// baseline lies in a heading's block, from its top to the next block's.
pub fn headings(blocks: &[Block], lines: &[Line]) -> Vec<bool> {
    lines
        .iter()
        .map(|line| {
            let at = blocks.partition_point(|block| block.top_pt <= line.baseline_pt);
            at.checked_sub(1)
                .is_some_and(|at| blocks[at].heading.is_some())
        })
        .collect()
}

/// Where the page stops showing above an overlay whose top stands at
/// `over`: the top of the first line that won't show whole above it, or of
/// a heading it would leave standing alone, or `over` itself when no line
/// is cut and none would stand alone. `lines` go top to bottom.
pub fn bare_from(lines: &[Shown], over: f64) -> f64 {
    let Some(mut hidden) = lines.iter().position(|line| line.foot > over) else {
        return over;
    };
    while hidden > 0 && lines[hidden - 1].heading {
        hidden -= 1;
    }
    lines[hidden].top.min(over)
}

#[cfg(test)]
#[expect(clippy::float_cmp, reason = "the places given, read back whole")]
mod tests {
    use super::*;

    fn words(top: f64) -> Shown {
        Shown {
            top,
            foot: top + 28.0,
            heading: false,
        }
    }

    fn heading(top: f64) -> Shown {
        Shown {
            heading: true,
            ..words(top)
        }
    }

    #[test]
    fn an_overlay_over_a_gap_bares_nothing() {
        let lines = [words(0.0), words(28.0), words(98.0)];
        assert_eq!(bare_from(&lines, 70.0), 70.0);
    }

    #[test]
    fn an_overlay_through_a_line_bares_it_from_its_top() {
        let lines = [words(0.0), words(28.0), words(56.0)];
        assert_eq!(bare_from(&lines, 70.0), 56.0);
    }

    #[test]
    fn the_room_over_a_line_cut_stays_shown() {
        let lines = [words(0.0), words(70.0)];
        assert_eq!(bare_from(&lines, 80.0), 70.0);
    }

    #[test]
    fn a_heading_whose_next_line_is_cut_hides_with_it() {
        let lines = [words(0.0), heading(56.0), words(84.0)];
        assert_eq!(bare_from(&lines, 100.0), 56.0);
    }

    #[test]
    fn a_heading_whose_next_line_falls_under_the_overlay_hides_too() {
        let lines = [words(0.0), heading(56.0), words(112.0)];
        assert_eq!(bare_from(&lines, 100.0), 56.0);
    }

    #[test]
    fn a_heading_of_two_lines_hides_whole() {
        let lines = [words(0.0), heading(56.0), heading(84.0), words(126.0)];
        assert_eq!(bare_from(&lines, 140.0), 56.0);
    }

    #[test]
    fn a_heading_with_its_next_line_whole_stays() {
        let lines = [heading(0.0), words(42.0), words(70.0)];
        assert_eq!(bare_from(&lines, 90.0), 70.0);
    }

    #[test]
    fn a_first_line_cut_bares_from_its_top() {
        let lines = [words(-10.0), words(18.0)];
        assert_eq!(bare_from(&lines, 10.0), -10.0);
    }

    fn line(baseline_pt: f64) -> Line {
        Line {
            x_pt: 0.0,
            tail_x_pt: 0.0,
            baseline_pt,
            size_pt: 10.0,
            cap_pt: 7.0,
            text: "Words".into(),
            spans: vec![(0.0, 1.0); 5],
        }
    }

    fn block(number: usize, top_pt: f64, heading: bool) -> Block {
        Block {
            number,
            top_pt,
            lines: None,
            heading: heading.then(|| (2, format!("Heading {number}"))),
        }
    }

    #[test]
    fn a_line_is_a_headings_when_it_lies_in_a_headings_block() {
        let blocks = [
            block(0, 0.0, false),
            block(1, 40.0, true),
            block(2, 80.0, false),
        ];
        let lines = [line(10.0), line(30.0), line(50.0), line(70.0), line(90.0)];
        assert_eq!(headings(&blocks, &lines), [false, false, true, true, false]);
    }
}
