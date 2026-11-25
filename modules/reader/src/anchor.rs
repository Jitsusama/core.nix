//! Where the view goes when the page is set again under it, as after a
//! save or at a new width: on the same words as before, or at the end if
//! it was there.

use crate::text::Line;
use crate::{moves, place};

/// Where the view's top line is on a new page: the same words, the same
/// how-many-th time they come, at the same height in the view; the end of
/// the page if the view was at the end, as while writing there. Points.
pub fn anchored(old: &[Line], new: &[Line], top_pt: f64, at_end: bool, new_most_pt: f64) -> f64 {
    if at_end {
        return new_most_pt;
    }
    matched(old, new, top_pt, new_most_pt, false).unwrap_or_else(|| top_pt.min(new_most_pt))
}

/// Where the words of the line at `top_pt` are on a new page, as
/// `anchored` finds them, or none where no new line says them: at another
/// width a paragraph's lines wrap anew, and only its unbroken lines match.
/// The view takes the last line saying them where fewer do now, near
/// enough to read on from; a point kept `alone` takes only the same
/// how-many-th of as many, since a short line such as "it." or "}" says
/// the same as others far off, and one rewrapped away would land on them.
fn matched(old: &[Line], new: &[Line], top_pt: f64, new_most_pt: f64, alone: bool) -> Option<f64> {
    let (index, line) = old
        .iter()
        .enumerate()
        .find(|(_, line)| line.baseline_pt >= top_pt)?;
    let nth = old[..index]
        .iter()
        .filter(|other| other.text == line.text)
        .count();
    let same: Vec<&Line> = new.iter().filter(|other| other.text == line.text).collect();
    let found = if alone {
        let was = old.iter().filter(|other| other.text == line.text).count();
        same.get(nth).filter(|_| same.len() == was)?
    } else {
        same.get(nth).or(same.last())?
    };
    Some((found.baseline_pt - (line.baseline_pt - top_pt)).clamp(0.0, new_most_pt))
}

/// What each block of a page says, by the block its lines' baselines are
/// in, as `place::at` finds it: its words run together without the spaces
/// and hyphens wrapping puts between them, so a paragraph says the same at
/// any width. `starts` run down the page.
fn said(starts: &[(usize, f64)], lines: &[Line]) -> Vec<(usize, String)> {
    let mut blocks: Vec<(usize, String)> = starts
        .iter()
        .map(|(block, _)| (*block, String::new()))
        .collect();
    for line in lines {
        let next = starts.partition_point(|(_, top)| *top <= line.baseline_pt);
        if let Some((_, words)) = blocks.get_mut(next.saturating_sub(1)) {
            words.extend(
                line.text
                    .chars()
                    .filter(|c| !c.is_whitespace() && !matches!(c, '-' | '\u{ad}')),
            );
        }
    }
    blocks
}

/// How much of `one` another block's words `other` say alike: as many
/// characters as they open with and end with in common, up to the shorter,
/// since an edit into a paragraph keeps its start or its end.
fn alike(one: &str, other: &str) -> usize {
    let opening = one
        .chars()
        .zip(other.chars())
        .take_while(|(a, b)| a == b)
        .count();
    let ending = one
        .chars()
        .rev()
        .zip(other.chars().rev())
        .take_while(|(a, b)| a == b)
        .count();
    (opening + ending).min(one.chars().count().min(other.chars().count()))
}

/// What each block of the page set before and of the page set now says,
/// as `said` reads them.
type BlocksSaid = (Vec<(usize, String)>, Vec<(usize, String)>);

/// A page set again, as a mark, the cursor's line and a jump's place are
/// moved across it: whether the file was `edited`, whether it was set at
/// another width (`reflowed`), where its blocks started `before` and start
/// `after`, the lines it was set from (`was`) and is set from now
/// (`now`, and their tops and baselines, `ends`), and the most a point goes:
/// the new page's height, past which no line stands. A jump's place, a
/// view's top, may land past the most the view goes, which `leap` holds.
/// What its blocks say is read once, at the first point that asks (`said`,
/// empty to start), however many marks and jumps move.
pub struct Moving<'a> {
    pub edited: bool,
    pub reflowed: bool,
    pub before: &'a [(usize, f64)],
    pub after: &'a [(usize, f64)],
    pub was: &'a [Line],
    pub now: &'a [Line],
    pub ends: &'a [(f64, f64)],
    pub most_pt: f64,
    pub said: std::cell::OnceCell<BlocksSaid>,
}

impl Moving<'_> {
    /// Where a point goes. After an edit, with its line's words, since an
    /// edit may renumber the blocks. Where no new line says them, as at
    /// another width, where a paragraph wraps anew, or with no edit, as far
    /// through its block as it was, on that block's nearest line: after an
    /// edit, in the block saying what its block said, or where the edit
    /// changed that, numbered as the nearest block saying the same is, and
    /// by its number only where no block does. At the same width with no
    /// edit, nowhere; after an edit there with its words gone, no further
    /// than the page goes.
    pub fn moved(&self, pt: f64) -> f64 {
        let words = if self.edited {
            matched(self.was, self.now, pt, self.most_pt, true)
        } else {
            None
        };
        words
            .or_else(|| {
                // At the same width unedited, every block stands where it
                // stood, so nothing moves by its block.
                if !(self.reflowed || self.edited) {
                    return None;
                }
                let line = moves::line_topped(self.was, pt);
                place::moved_into(self.before, self.after, line, self.ends, |block| {
                    if self.edited {
                        self.block_now(self.before, block).unwrap_or(block)
                    } else {
                        block
                    }
                })
            })
            .unwrap_or(if self.edited {
                pt.min(self.most_pt)
            } else {
                pt
            })
    }

    /// The block on the edited page that `block` on the page set `before`
    /// is: the one saying what it said, the same how-many-th of as many.
    /// Where the edit changed its words, its number moved as the nearest
    /// blocks' saying the same, above and below it, moved. Where those two
    /// disagree, the edit added or took a block between them, so it's the
    /// block between them saying most of what it said, opening or ending
    /// alike. Where none does, it's the only block between them if there's
    /// one, else the one above's count, unless that falls outside them and
    /// the one below's doesn't: a count outside them names a block that
    /// says what another did. The one there is where only one is. None
    /// where no block says what it did.
    fn block_now(&self, before: &[(usize, f64)], block: usize) -> Option<usize> {
        let (was, now) = self
            .said
            .get_or_init(|| (said(before, self.was), said(self.after, self.now)));
        let at = was.iter().position(|(b, _)| *b == block)?;
        let found = |at: usize| -> Option<usize> {
            let words = &was[at].1;
            if words.is_empty() {
                return None;
            }
            let nth = was[..at].iter().filter(|(_, w)| w == words).count();
            let many = was.iter().filter(|(_, w)| w == words).count();
            let same: Vec<usize> = now
                .iter()
                .filter(|(_, w)| w == words)
                .map(|(b, _)| *b)
                .collect();
            same.get(nth).filter(|_| same.len() == many).copied()
        };
        if let Some(into) = found(at) {
            return Some(into);
        }
        // A block saying the same, and the number that puts the mark's at.
        let renumbered = |near: usize| {
            let into = found(near)?;
            Some((into, (block + into).checked_sub(was[near].0)?))
        };
        let above = (0..at).rev().find_map(renumbered);
        let below = (at + 1..was.len()).find_map(renumbered);
        match (above, below) {
            (Some((up, by_up)), Some((down, by_down))) if by_up != by_down => {
                let words = &was[at].1;
                let within = |b: usize| b > up && b < down;
                let between: Vec<&(usize, String)> =
                    now.iter().filter(|(b, _)| within(*b)).collect();
                let otherwise = match between.as_slice() {
                    [only] => only.0,
                    _ if !within(by_up) && within(by_down) => by_down,
                    _ => by_up,
                };
                Some(
                    between
                        .iter()
                        .map(|(b, w)| (*b, alike(words, w)))
                        .filter(|(_, much)| *much > 0)
                        .rev()
                        .max_by_key(|(_, much)| *much)
                        .map_or(otherwise, |(b, _)| b),
                )
            }
            (Some((_, by)), _) | (None, Some((_, by))) => Some(by),
            (None, None) => None,
        }
    }
}

/// Where a view that a key moved while a page was being set goes once the
/// page shows, in points: nowhere new if it's still where it was when the
/// page was set, and otherwise where it's going, carried onto the new page
/// as a top is.
pub fn carried(
    old: &[Line],
    new: &[Line],
    set_at_pt: f64,
    going_pt: f64,
    at_end: bool,
    new_most_pt: f64,
) -> Option<f64> {
    // Within half a point, the page was set for where the view is.
    ((going_pt - set_at_pt).abs() >= 0.5).then(|| anchored(old, new, going_pt, at_end, new_most_pt))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::test_line as line;

    #[test]
    fn a_view_no_key_moved_stays_where_the_page_was_set_for() {
        let old = [line(20.0, "one"), line(40.0, "two")];
        let new = [line(30.0, "one"), line(60.0, "two")];
        assert_eq!(carried(&old, &new, 20.0, 20.0, false, 500.0), None);
    }

    #[test]
    fn a_view_sent_to_the_end_while_the_page_was_set_goes_to_its_end() {
        let old = [line(20.0, "one"), line(40.0, "two")];
        let new = [line(30.0, "one"), line(60.0, "two")];
        assert_eq!(carried(&old, &new, 0.0, 400.0, true, 900.0), Some(900.0));
    }

    #[test]
    fn a_view_moved_while_the_page_was_set_keeps_the_words_it_moved_to() {
        let old = [line(20.0, "one"), line(40.0, "two"), line(60.0, "three")];
        let new = [line(30.0, "one"), line(60.0, "two"), line(90.0, "three")];
        // Moved to 5 pt over "two", it stays 5 pt over "two" where it's set now.
        assert_eq!(carried(&old, &new, 0.0, 35.0, false, 500.0), Some(55.0));
    }

    #[test]
    fn the_view_on_a_page_an_edit_renumbered_goes_with_its_words() {
        // A paragraph saved in above it: a view on "two" goes with "two",
        // where its block's number would now name the paragraph before.
        let old = [line(20.0, "one"), line(40.0, "two")];
        let new = [line(20.0, "new"), line(40.0, "one"), line(60.0, "two")];
        assert_eq!(anchored(&old, &new, 40.0, false, 500.0), 60.0);
    }

    /// Where `pt` goes across a page set again, each line its own block
    /// unless told: whether it was `edited`, whether it was set at another
    /// width (`reflowed`), and where its blocks started `before`.
    fn moving(
        edited: bool,
        reflowed: bool,
        before: &[(usize, f64)],
        after: &[(usize, f64)],
        was: &[Line],
        now: &[Line],
        pt: f64,
    ) -> f64 {
        let ends: Vec<(f64, f64)> = now.iter().map(moves::ends_of).collect();
        Moving {
            edited,
            reflowed,
            before,
            after,
            was,
            now,
            ends: &ends,
            most_pt: 500.0,
            said: std::cell::OnceCell::new(),
        }
        .moved(pt)
    }

    #[test]
    fn a_point_kept_alone_takes_a_line_only_where_as_many_say_it() {
        // The first of two "it." lines cut: the one left was the second.
        let old = [line(20.0, "it."), line(40.0, "x"), line(60.0, "it.")];
        let new = [line(20.0, "x"), line(40.0, "it.")];
        assert_eq!(matched(&old, &new, 10.0, 500.0, true), None);
        assert_eq!(
            matched(&old, &new, 10.0, 500.0, false),
            Some(30.0),
            "the view reads on"
        );
        // One "}" become two: the mark's may be either.
        let old = [line(20.0, "}"), line(40.0, "x")];
        let new = [line(20.0, "}"), line(40.0, "}"), line(60.0, "x")];
        assert_eq!(matched(&old, &new, 10.0, 500.0, true), None);
    }

    #[test]
    fn a_blocks_words_run_together_as_any_width_wraps_them() {
        // A line over the first block is the first block's; a hyphen or a
        // soft hyphen a wrap puts in is no word of the paragraph's.
        let starts = [(0, 30.0), (1, 60.0)];
        let lines = [
            line(20.0, "over"),
            line(40.0, "hy-"),
            line(50.0, "phen soft\u{ad}"),
            line(70.0, "ly"),
        ];
        assert_eq!(
            said(&starts, &lines),
            vec![(0, "overhyphensoft".to_string()), (1, "ly".to_string())]
        );
    }

    #[test]
    fn an_edit_moves_a_mark_with_its_words_though_the_width_changed_too() {
        // A paragraph saved in above "two" renumbers its block: by its
        // block, a mark on it would land on "one".
        let old = [line(20.0, "one"), line(40.0, "two")];
        let new = [line(20.0, "new"), line(40.0, "one"), line(60.0, "two")];
        let before = [(0, 0.0), (1, 25.0)];
        let after = [(0, 0.0), (1, 25.0), (2, 45.0)];
        assert_eq!(
            moving(true, true, &before, &after, &old, &new, 30.0),
            50.0,
            "edited, it goes with \"two\""
        );
    }

    #[test]
    fn a_mark_on_a_page_set_wider_unedited_goes_as_far_through_its_block() {
        // A paragraph of two lines, 40 pt tall, set in one 20 pt tall: a
        // mark on its second line, 25 pt into it, goes 12.5 pt in, onto
        // the one line it has now, not to a line saying "c d" in another
        // block, since nothing was edited.
        let old = [line(40.0, "a b"), line(60.0, "c d")];
        let new = [line(40.0, "a b c d"), line(90.0, "c d")];
        let before = [(0, 0.0), (1, 25.0), (2, 65.0)];
        let after = [(0, 0.0), (1, 25.0), (2, 45.0)];
        assert_eq!(moving(false, true, &before, &after, &old, &new, 50.0), 30.0);
    }

    #[test]
    fn an_edit_at_a_new_width_that_adds_a_block_above_moves_a_mark_into_its_own() {
        // A paragraph saved in above "alpha beta gamma delta", set at a
        // narrower window: no new line says "delta" alone, and its block's
        // number now names the new paragraph. The mark goes as far through
        // the block saying what its block said, onto "gamma delta".
        let old = [line(50.0, "alpha beta gamma"), line(70.0, "delta")];
        let new = [
            line(40.0, "inserted"),
            line(70.0, "alpha beta"),
            line(90.0, "gamma delta"),
        ];
        let before = [(0, 0.0), (1, 30.0), (2, 80.0)];
        let after = [(0, 0.0), (1, 30.0), (2, 50.0), (3, 120.0)];
        assert_eq!(moving(true, true, &before, &after, &old, &new, 60.0), 80.0);
    }

    #[test]
    fn an_edit_to_a_marks_own_paragraph_numbers_it_as_the_one_above() {
        // A paragraph saved in above "intro", and words into the marked
        // paragraph: "intro" says it moved down a block, and so did the
        // mark's, onto "gamma delta" rather than the new paragraph.
        let old = [
            line(20.0, "intro"),
            line(50.0, "alpha beta gamma"),
            line(70.0, "delta"),
        ];
        let new = [
            line(20.0, "inserted"),
            line(40.0, "intro"),
            line(70.0, "alpha beta more"),
            line(90.0, "gamma delta"),
        ];
        let before = [(0, 0.0), (1, 30.0), (2, 80.0)];
        let after = [(0, 0.0), (1, 30.0), (2, 50.0), (3, 120.0)];
        assert_eq!(moving(true, false, &before, &after, &old, &new, 60.0), 80.0);
    }

    #[test]
    fn a_mark_on_a_short_line_fewer_say_now_takes_none_of_them() {
        // Two lines said "it." and one does now: the mark on the second
        // is on neither for sure, and its block is gone, so it stays.
        let old = [line(20.0, "it."), line(40.0, "x"), line(60.0, "it.")];
        let new = [line(20.0, "it."), line(40.0, "x y")];
        let before = [(0, 0.0), (1, 25.0), (2, 45.0)];
        let after = [(0, 0.0), (1, 25.0)];
        assert_eq!(moving(true, false, &before, &after, &old, &new, 50.0), 50.0);
        // The view, reading on, goes to the one there is.
        assert_eq!(anchored(&old, &new, 50.0, false, 500.0), 10.0);
    }

    #[test]
    fn an_edit_at_a_new_width_whose_line_wrapped_anew_moves_a_mark_by_its_block() {
        // Words saved into a paragraph set at a wider window: its lines
        // wrap anew, so no new line says "delta" alone, and the mark on it
        // goes as far through the paragraph, onto the line holding it.
        let old = [line(50.0, "alpha beta gamma"), line(70.0, "delta")];
        let new = [
            line(50.0, "inserted"),
            line(70.0, "alpha beta"),
            line(90.0, "gamma delta"),
        ];
        let before = [(0, 0.0), (1, 30.0), (2, 80.0)];
        let after = [(0, 0.0), (1, 30.0), (2, 100.0)];
        assert_eq!(
            moving(true, true, &before, &after, &old, &new, 60.0),
            80.0,
            "it lands on \"gamma delta\", not where \"delta\" stood"
        );
    }

    #[test]
    fn an_edit_that_adds_a_block_just_above_a_marks_edited_one_leaves_it_its_own() {
        // "new para" saved in between "aaa" and the marked "bee one", which
        // gains a word: "aaa" says the mark's block stays first after it,
        // "cee" that it moved down one. Of the two between them, "bee one
        // two" opens as it did, so the mark goes there.
        let old = [line(20.0, "aaa"), line(40.0, "bee one"), line(60.0, "cee")];
        let new = [
            line(20.0, "aaa"),
            line(40.0, "new para"),
            line(60.0, "bee one two"),
            line(80.0, "cee"),
        ];
        let before = [(0, 0.0), (1, 25.0), (2, 45.0)];
        let after = [(0, 0.0), (1, 25.0), (2, 45.0), (3, 65.0)];
        assert_eq!(moving(true, false, &before, &after, &old, &new, 30.0), 50.0);
    }

    #[test]
    fn an_edit_to_a_marks_first_block_numbers_it_as_the_one_below() {
        // A paragraph saved in at the top, and a word into the marked first
        // one: nothing above it, "cee" below says it moved down one.
        let old = [line(20.0, "aaa bbb"), line(40.0, "cee")];
        let new = [
            line(20.0, "new"),
            line(40.0, "aaa bbb more"),
            line(60.0, "cee"),
        ];
        let before = [(0, 0.0), (1, 25.0)];
        let after = [(0, 0.0), (1, 25.0), (2, 45.0)];
        assert_eq!(moving(true, false, &before, &after, &old, &new, 10.0), 30.0);
    }

    #[test]
    fn a_block_saying_what_more_blocks_say_now_numbers_nothing() {
        // A line of stars saved in at the top, and the marked block edited:
        // "***" above it now says what two blocks do, so it can't say where
        // the mark's went, and "p" further up does.
        let old = [line(20.0, "p"), line(40.0, "***"), line(60.0, "aa")];
        let new = [
            line(20.0, "***"),
            line(40.0, "p"),
            line(60.0, "***"),
            line(80.0, "aa c"),
        ];
        let before = [(0, 0.0), (1, 25.0), (2, 45.0)];
        let after = [(0, 0.0), (1, 25.0), (2, 45.0), (3, 65.0)];
        assert_eq!(moving(true, false, &before, &after, &old, &new, 50.0), 70.0);
    }

    #[test]
    fn a_mark_on_a_renamed_heading_under_a_cut_block_goes_into_the_one_between() {
        // "ddd" cut and the marked heading "Install" renamed "Setup", at
        // the same width: no line says "Install" now, so the mark goes by
        // its block. "aaa" above says it stays third, "bbb" below that it's
        // second. Nothing between them opens or ends as "Install" did, but
        // one block is there, so the mark goes into it, not onto "bbb",
        // which says what another block did.
        let old = [
            line(20.0, "aaa"),
            line(40.0, "ddd"),
            line(60.0, "Install"),
            line(80.0, "bbb"),
        ];
        let new = [line(20.0, "aaa"), line(40.0, "Setup"), line(60.0, "bbb")];
        let before = [(0, 0.0), (1, 25.0), (2, 45.0), (3, 65.0)];
        let after = [(0, 0.0), (1, 25.0), (2, 45.0)];
        assert_eq!(
            moving(true, false, &before, &after, &old, &new, 50.0),
            30.0,
            "the mark landed off \"Setup\""
        );
    }

    #[test]
    fn a_page_set_again_at_its_width_unedited_moves_nothing() {
        let lines = [line(20.0, "one"), line(40.0, "two")];
        let starts = [(0, 0.0), (1, 25.0)];
        assert_eq!(
            moving(false, false, &starts, &starts, &lines, &lines, 33.0),
            33.0
        );
    }

    #[test]
    fn a_view_moved_past_a_shorter_pages_end_stops_at_it() {
        let old = [line(20.0, "one"), line(40.0, "two"), line(60.0, "three")];
        let new = [line(10.0, "one"), line(20.0, "two"), line(30.0, "three")];
        // 5 pt over "three" would be 25 pt on the new page, past its most.
        assert_eq!(carried(&old, &new, 0.0, 55.0, false, 20.0), Some(20.0));
    }

    #[test]
    fn a_view_moved_before_the_page_had_its_words_keeps_its_place() {
        let new = [line(30.0, "one")];
        assert_eq!(carried(&[], &new, 0.0, 120.0, false, 500.0), Some(120.0));
        assert_eq!(carried(&[], &new, 0.0, 120.0, false, 80.0), Some(80.0));
    }
}
