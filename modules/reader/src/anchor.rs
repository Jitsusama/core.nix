//! Where the view goes when the page is set again under it, as after a
//! save or at a new width: on the same words as before, or at the end if
//! it was there.

use crate::text::Line;

/// Where the view's top line is on a new page: the same words, the same
/// how-many-th time they come, at the same height in the view; the end of
/// the page if the view was at the end, as while writing there. Points.
pub fn anchored(old: &[Line], new: &[Line], top_pt: f64, at_end: bool, new_most_pt: f64) -> f64 {
    if at_end {
        return new_most_pt;
    }
    let Some((index, line)) = old
        .iter()
        .enumerate()
        .find(|(_, line)| line.baseline_pt >= top_pt)
    else {
        return top_pt.min(new_most_pt);
    };
    let nth = old[..index]
        .iter()
        .filter(|other| other.text == line.text)
        .count();
    let same: Vec<&Line> = new.iter().filter(|other| other.text == line.text).collect();
    match same.get(nth).or(same.last()) {
        Some(found) => (found.baseline_pt - (line.baseline_pt - top_pt)).clamp(0.0, new_most_pt),
        None => top_pt.min(new_most_pt),
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

    fn line(baseline_pt: f64, text: &str) -> Line {
        Line {
            x_pt: 0.0,
            tail_x_pt: 0.0,
            baseline_pt,
            size_pt: 10.0,
            cap_pt: 7.0,
            text: text.to_string(),
            spans: Vec::new(),
        }
    }

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
    fn a_view_moved_before_the_page_had_its_words_keeps_its_place() {
        let new = [line(30.0, "one")];
        assert_eq!(carried(&[], &new, 0.0, 120.0, false, 500.0), Some(120.0));
        assert_eq!(carried(&[], &new, 0.0, 120.0, false, 80.0), Some(80.0));
    }
}
