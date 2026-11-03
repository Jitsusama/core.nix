//! Where the page's words sit, so / and ? can find them. Typst's frames are
//! walked once into lines: the runs sharing a baseline and standing close,
//! left to right, each character with its span across the page. A word
//! hyphenation broke is found across the break, as it reads.
use typst::layout::{Frame, FrameItem, Point};

/// A run of set text and where it sits, in points from the page's top left.
pub struct Run {
    pub x_pt: f64,
    pub baseline_pt: f64,
    pub size_pt: f64,
    /// Each character and where it starts and ends across the page.
    pub chars: Vec<(char, f64, f64)>,
}

/// A line of the page: its runs joined, left to right.
pub struct Line {
    pub x_pt: f64,
    /// Where its last run starts: a table cell's, when its first line
    /// shares a baseline with the cells beside it.
    pub tail_x_pt: f64,
    pub baseline_pt: f64,
    pub size_pt: f64,
    pub text: String,
    /// For each character of `text`, by its place among them, where it
    /// starts and ends across the page.
    pub spans: Vec<(f64, f64)>,
}

/// A match's part on one line.
#[derive(Clone, Copy, Debug, PartialEq)]
#[expect(
    clippy::struct_field_names,
    reason = "each is in points, as everything on the page is"
)]
pub struct Piece {
    pub baseline_pt: f64,
    pub size_pt: f64,
    pub from_pt: f64,
    pub to_pt: f64,
}

/// A match: one piece, or one on each line of a word broken across them.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub pieces: Vec<Piece>,
}

/// Runs this close in baseline are one line: a superscript's or a mark's
/// nudge is less than this.
const SAME_LINE_PT: f64 = 2.0;

/// A gap wider than a word space, in ems: a space is set between runs
/// this far apart that neither spells one.
const SPACED_EM: f64 = 0.15;

/// A gap this wide, in ems, is a column's, a table cell's from the next:
/// what stands either side is a line of its own.
const APART_EM: f64 = 0.8;

/// Where a broken word's line goes on: starting at the same edge.
const SAME_EDGE_PT: f64 = 1.0;

/// The soft hyphen Typst sets at a break hyphenation made.
const BREAK: char = '\u{ad}';

/// Takes every run of text out of a page's frame, with where it sits.
pub fn runs(frame: &Frame) -> Vec<Run> {
    let mut found = Vec::new();
    walk(frame, Point::zero(), &mut found);
    found
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a cluster's character count, a handful"
)]
fn walk(frame: &Frame, at: Point, found: &mut Vec<Run>) {
    for (place, item) in frame.items() {
        let here = at + *place;
        match item {
            // Groups move what's in them; the page's text is never scaled
            // or turned, so only the move is taken.
            FrameItem::Group(group) => {
                let moved = Point::new(group.transform.tx, group.transform.ty);
                walk(&group.frame, here + moved, found);
            }
            FrameItem::Text(text) => {
                let size = text.size.to_pt();
                let mut x = here.x.to_pt();
                let mut chars = Vec::new();
                for glyph in &text.glyphs {
                    let advance = glyph.x_advance.at(text.size).to_pt();
                    let range = usize::from(glyph.range.start)..usize::from(glyph.range.end);
                    let cluster = text.text.get(range).unwrap_or_default();
                    // A ligature's characters share its advance evenly.
                    let count = cluster.chars().count().max(1) as f64;
                    for (i, c) in cluster.chars().enumerate() {
                        let from = x + advance * i as f64 / count;
                        chars.push((c, from, from + advance / count));
                    }
                    x += advance;
                }
                found.push(Run {
                    x_pt: here.x.to_pt(),
                    baseline_pt: here.y.to_pt(),
                    size_pt: size,
                    chars,
                });
            }
            _ => {}
        }
    }
}

/// Joins runs into lines, top to bottom and left to right, with a space
/// between two runs a gap apart that neither one spells.
pub fn lines(mut runs: Vec<Run>) -> Vec<Line> {
    runs.retain(|run| !run.chars.is_empty());
    runs.sort_by(|a, b| {
        a.baseline_pt
            .total_cmp(&b.baseline_pt)
            .then(a.x_pt.total_cmp(&b.x_pt))
    });
    let mut rows: Vec<Vec<Run>> = Vec::new();
    for run in runs {
        match rows.last_mut() {
            Some(row) if (run.baseline_pt - row[0].baseline_pt).abs() < SAME_LINE_PT => {
                row.push(run);
            }
            _ => rows.push(vec![run]),
        }
    }
    let mut lines = Vec::new();
    for mut row in rows {
        row.sort_by(|a, b| a.x_pt.total_cmp(&b.x_pt));
        let mut line: Option<Line> = None;
        for run in row {
            let starts_at = run.chars[0].1;
            if let Some(open) = line.as_mut() {
                let end = open.spans.last().map_or(open.x_pt, |span| span.1);
                let gap = starts_at - end;
                if gap > run.size_pt * APART_EM {
                    lines.extend(line.take());
                } else if gap > run.size_pt * SPACED_EM
                    && !open.text.ends_with(char::is_whitespace)
                    && !run.chars[0].0.is_whitespace()
                {
                    open.text.push(' ');
                    open.spans.push((end, starts_at));
                }
            }
            let open = line.get_or_insert_with(|| Line {
                x_pt: starts_at,
                tail_x_pt: starts_at,
                baseline_pt: run.baseline_pt,
                size_pt: 0.0,
                text: String::new(),
                spans: Vec::new(),
            });
            open.size_pt = open.size_pt.max(run.size_pt);
            open.tail_x_pt = starts_at;
            for (c, from, to) in run.chars {
                open.text.push(c);
                open.spans.push((from, to));
            }
        }
        lines.extend(line);
    }
    lines.sort_by(|a, b| {
        a.baseline_pt
            .total_cmp(&b.baseline_pt)
            .then(a.x_pt.total_cmp(&b.x_pt))
    });
    lines
}

/// The line a line ending in a break goes on to: the next one down
/// starting at its edge, or its last run's, within two of its lines.
fn goes_on(lines: &[Line], from: usize) -> Option<usize> {
    let line = &lines[from];
    if !line.text.ends_with(BREAK) {
        return None;
    }
    lines
        .iter()
        .enumerate()
        .skip(from + 1)
        .find_map(|(i, next)| {
            let below = next.baseline_pt - line.baseline_pt;
            let edge = |x: f64| (next.x_pt - x).abs() <= SAME_EDGE_PT;
            (below > SAME_LINE_PT
                && below <= line.size_pt * 3.0
                && (edge(line.x_pt) || edge(line.tail_x_pt)))
            .then_some(i)
        })
}

/// Every match of `query` on the page, top to bottom. Case is ignored until
/// the query holds a capital, as smartcase has it.
#[expect(
    clippy::float_cmp,
    reason = "a piece's baseline is its line's, copied, never computed"
)]
pub fn find(lines: &[Line], query: &str) -> Vec<Found> {
    if query.is_empty() {
        return Vec::new();
    }
    let ignore_case = !query.chars().any(char::is_uppercase);
    let fold = |text: &str| -> Vec<char> {
        if ignore_case {
            text.chars().flat_map(char::to_lowercase).collect()
        } else {
            text.chars().collect()
        }
    };
    let wanted = fold(query);
    // Lines a break carries on from are searched from where it starts.
    let mut continued = vec![false; lines.len()];
    let mut found = Vec::new();
    for start in 0..lines.len() {
        if continued[start] {
            continue;
        }
        let mut chain = vec![start];
        while let Some(next) = goes_on(lines, *chain.last().unwrap_or(&start)) {
            continued[next] = true;
            chain.push(next);
        }
        // The chain's characters, each with its line and its span; a
        // break's hyphen is left out, as it isn't in the word.
        let mut chars = Vec::new();
        let mut at = Vec::new();
        for (place, &l) in chain.iter().enumerate() {
            let line = &lines[l];
            let folded = fold(&line.text);
            // Folding never changes how many characters the page sets, so
            // a place in `folded` is one in `spans`; a line where it would
            // isn't searched.
            if folded.len() != line.spans.len() {
                continue;
            }
            let last = place + 1 == chain.len();
            for (k, c) in folded.into_iter().enumerate() {
                if c == BREAK && !last && k + 1 == line.spans.len() {
                    continue;
                }
                chars.push(c);
                at.push((l, line.spans[k]));
            }
        }
        if chars.len() < wanted.len() {
            continue;
        }
        for i in 0..=chars.len() - wanted.len() {
            if chars[i..i + wanted.len()] != wanted[..] {
                continue;
            }
            let mut pieces: Vec<Piece> = Vec::new();
            for &(l, (from, to)) in &at[i..i + wanted.len()] {
                let line = &lines[l];
                match pieces.last_mut() {
                    Some(piece) if piece.baseline_pt == line.baseline_pt => piece.to_pt = to,
                    _ => pieces.push(Piece {
                        baseline_pt: line.baseline_pt,
                        size_pt: line.size_pt,
                        from_pt: from,
                        to_pt: to,
                    }),
                }
            }
            found.push(Found { pieces });
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[expect(
        clippy::cast_precision_loss,
        reason = "a cluster's character count, a handful"
    )]
    fn run(x: f64, baseline: f64, text: &str) -> Run {
        let chars = text
            .chars()
            .enumerate()
            .map(|(i, c)| (c, x + i as f64 * 6.0, x + (i + 1) as f64 * 6.0))
            .collect();
        Run {
            x_pt: x,
            baseline_pt: baseline,
            size_pt: 13.1,
            chars,
        }
    }

    fn first(found: &Found) -> Piece {
        found.pieces[0]
    }

    #[test]
    fn runs_on_one_baseline_are_one_line_left_to_right() {
        let lines = lines(vec![
            run(36.0, 100.0, "world"),
            run(0.0, 100.5, "hello"),
            run(0.0, 121.0, "next"),
        ]);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "hello world");
        assert_eq!(lines[1].text, "next");
    }

    #[test]
    fn runs_that_touch_get_no_space_between() {
        let lines = lines(vec![run(0.0, 10.0, "bold"), run(24.0, 10.0, "face")]);
        assert_eq!(lines[0].text, "boldface");
    }

    #[test]
    fn cells_a_column_apart_are_lines_of_their_own() {
        let lines = lines(vec![run(0.0, 10.0, "left"), run(100.0, 10.0, "right")]);
        assert_eq!(
            lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>(),
            ["left", "right"]
        );
        assert!(find(&lines, "t r").is_empty());
    }

    #[test]
    #[expect(clippy::float_cmp, reason = "the baselines given, read back unchanged")]
    fn each_match_says_where_it_sits() {
        let lines = lines(vec![run(0.0, 10.0, "a cat sat"), run(0.0, 30.0, "cats")]);
        let found = find(&lines, "cat");
        assert_eq!(found.len(), 2);
        let piece = first(&found[0]);
        assert_eq!(
            (piece.baseline_pt, piece.from_pt, piece.to_pt),
            (10.0, 12.0, 30.0)
        );
        assert_eq!(first(&found[1]).baseline_pt, 30.0);
    }

    #[test]
    fn a_word_broken_across_lines_is_found_in_two_pieces() {
        // A table's cell, its neighbour beside it on each line.
        let lines = lines(vec![
            run(0.0, 10.0, "Quick\u{ad}"),
            run(100.0, 10.0, "beside"),
            run(0.0, 26.0, "shell runs"),
            run(100.0, 26.0, "also"),
        ]);
        let found = find(&lines, "quickshell");
        assert_eq!(found.len(), 1);
        let pieces = &found[0].pieces;
        assert_eq!(pieces.len(), 2);
        assert_eq!(
            (pieces[0].baseline_pt, pieces[0].from_pt, pieces[0].to_pt),
            (10.0, 0.0, 30.0)
        );
        assert_eq!(
            (pieces[1].baseline_pt, pieces[1].from_pt, pieces[1].to_pt),
            (26.0, 0.0, 30.0)
        );
        // The line it goes on to is still searched for what is its own.
        assert_eq!(find(&lines, "runs").len(), 1);
    }

    #[test]
    fn a_cell_beside_another_on_its_first_line_goes_on_at_its_own_edge() {
        // The cell to its left runs into it, so both are on one line.
        let lines = lines(vec![
            run(0.0, 10.0, "The greeter"),
            run(66.0, 10.0, "Quick\u{ad}"),
            run(66.0, 26.0, "shell"),
        ]);
        assert_eq!(find(&lines, "Quickshell").len(), 1);
    }

    #[test]
    fn case_is_ignored_until_a_capital_is_typed() {
        let lines = lines(vec![run(0.0, 10.0, "Reader reader")]);
        assert_eq!(find(&lines, "reader").len(), 2);
        assert_eq!(find(&lines, "Reader").len(), 1);
    }

    #[test]
    fn a_match_can_cross_two_runs() {
        let lines = lines(vec![run(0.0, 10.0, "one"), run(24.0, 10.0, "two")]);
        assert_eq!(lines[0].text, "one two");
        assert_eq!(find(&lines, "e t").len(), 1);
    }

    #[test]
    fn nothing_is_found_for_nothing() {
        let lines = lines(vec![run(0.0, 10.0, "text")]);
        assert!(find(&lines, "").is_empty());
        assert!(find(&lines, "absent").is_empty());
    }
}
