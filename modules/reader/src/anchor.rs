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
    matched(old, new, top_pt, new_most_pt).unwrap_or_else(|| top_pt.min(new_most_pt))
}

/// Where the words of the line at `top_pt` are on a new page, as
/// `anchored` finds them, or none where no new line says them: at another
/// width a paragraph's lines wrap anew, and only its unbroken lines match.
fn matched(old: &[Line], new: &[Line], top_pt: f64, new_most_pt: f64) -> Option<f64> {
    let (index, line) = old
        .iter()
        .enumerate()
        .find(|(_, line)| line.baseline_pt >= top_pt)?;
    let nth = old[..index]
        .iter()
        .filter(|other| other.text == line.text)
        .count();
    let same: Vec<&Line> = new.iter().filter(|other| other.text == line.text).collect();
    let found = same.get(nth).or(same.last())?;
    Some((found.baseline_pt - (line.baseline_pt - top_pt)).clamp(0.0, new_most_pt))
}

/// A page set again, as a mark, the cursor's line and a jump's place are
/// moved across it: whether the file was `edited`, where its blocks started
/// `before` when it was set at another width (none at the same one) and
/// start `after`, the lines it was set from (`was`) and is set from now
/// (`now`, and their tops and baselines, `ends`), and the most the view goes.
pub struct Moving<'a> {
    pub edited: bool,
    pub before: Option<&'a [(usize, f64)]>,
    pub after: &'a [(usize, f64)],
    pub was: &'a [Line],
    pub now: &'a [Line],
    pub ends: &'a [(f64, f64)],
    pub most_pt: f64,
}

impl Moving<'_> {
    /// Where a point goes. After an edit, with its line's words, since an
    /// edit may renumber the blocks. Where no new line says them, as at
    /// another width, where a paragraph wraps anew, or with no edit, as far
    /// through its block as it was, on that block's nearest line. At the
    /// same width with no edit, nowhere; after an edit there with its words
    /// gone, no further than the page goes.
    pub fn moved(&self, pt: f64) -> f64 {
        let words = if self.edited {
            matched(self.was, self.now, pt, self.most_pt)
        } else {
            None
        };
        words
            .or_else(|| {
                let line = moves::line_topped(self.was, pt);
                place::moved(self.before?, self.after, line, self.ends)
            })
            .unwrap_or(if self.edited {
                pt.min(self.most_pt)
            } else {
                pt
            })
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
    fn a_mark_on_a_page_an_edit_renumbered_goes_with_its_words() {
        // A paragraph saved in above it: a mark on "two" goes with "two",
        // where its block's number would now name the paragraph before.
        let old = [line(20.0, "one"), line(40.0, "two")];
        let new = [line(20.0, "new"), line(40.0, "one"), line(60.0, "two")];
        assert_eq!(anchored(&old, &new, 40.0, false, 500.0), 60.0);
    }

    /// Where `pt` goes across a page set again, each line its own block
    /// unless told: `before`'s blocks at another width, or none.
    fn moving(
        edited: bool,
        before: Option<&[(usize, f64)]>,
        after: &[(usize, f64)],
        was: &[Line],
        now: &[Line],
        pt: f64,
    ) -> f64 {
        let ends: Vec<(f64, f64)> = now.iter().map(moves::ends_of).collect();
        Moving {
            edited,
            before,
            after,
            was,
            now,
            ends: &ends,
            most_pt: 500.0,
        }
        .moved(pt)
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
            moving(true, Some(&before), &after, &old, &new, 30.0),
            50.0,
            "edited, it goes with \"two\""
        );
        assert_eq!(
            moving(false, Some(&before), &after, &old, &new, 30.0),
            30.0,
            "unedited, it goes as far through its block"
        );
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
            moving(true, Some(&before), &after, &old, &new, 60.0),
            80.0,
            "it lands on \"gamma delta\", not where \"delta\" stood"
        );
        assert_eq!(
            moving(true, None, &after, &old, &new, 60.0),
            60.0,
            "at the same width with its words gone, it stays"
        );
    }

    #[test]
    fn a_page_set_again_at_its_width_unedited_moves_nothing() {
        let lines = [line(20.0, "one"), line(40.0, "two")];
        let starts = [(0, 0.0), (1, 25.0)];
        assert_eq!(moving(false, None, &starts, &lines, &lines, 33.0), 33.0);
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
