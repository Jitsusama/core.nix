//! How long the changes a repository merged took from their first edit to
//! their merge, the speed half of its health: a gate or a review that
//! grows slower shows here the day it does, as a file grown long shows in
//! measure.rs. A change's first edit is its first commit's author time,
//! which a rebase keeps, and its merge the merge commit's own time.

use crate::measure::NAMED;

/// A change merged into the line read: its merge commit, its title, when
/// its first commit was authored and when it merged, in Unix seconds.
#[derive(Debug, PartialEq)]
pub struct Merged {
    pub merge: String,
    pub title: String,
    pub first: i64,
    pub landed: i64,
}

/// The day's median minutes from first edit to merge, and each change
/// with its minutes, slowest first; none when nothing merged.
pub fn edit_to_merge(merged: &[Merged]) -> Option<(f64, Vec<String>)> {
    let mut took: Vec<(f64, &Merged)> = merged
        .iter()
        .map(|change| (minutes(change.landed - change.first), change))
        .collect();
    took.sort_by(|one, other| other.0.total_cmp(&one.0));
    let middle = took.len() / 2;
    let median = match took.len() {
        0 => return None,
        count if count % 2 == 1 => took[middle].0,
        _ => f64::midpoint(took[middle - 1].0, took[middle].0),
    };
    let named = took
        .iter()
        .take(NAMED)
        .map(|(minutes, change)| {
            let short = change.merge.get(..12).unwrap_or(&change.merge);
            format!("{short} {}: {minutes} min", change.title)
        })
        .collect();
    Some((median, named))
}

/// Seconds as minutes to a tenth; a merge its edit's clock put before the
/// edit took none.
fn minutes(seconds: i64) -> f64 {
    let tenths = i32::try_from(seconds.max(0) / 6).unwrap_or(i32::MAX);
    f64::from(tenths) / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(merge: &str, first: i64, landed: i64) -> Merged {
        Merged {
            merge: merge.to_owned(),
            title: format!("Merge {merge}"),
            first,
            landed,
        }
    }

    #[test]
    fn the_figure_is_the_middle_change_and_each_is_named_slowest_first() {
        let day = [
            change("aaaaaaaaaaaaaaaa", 0, 600),
            change("bbbbbbbbbbbbbbbb", 0, 3600),
            change("cccccccccccccccc", 0, 1200),
        ];
        let (median, named) = edit_to_merge(&day).expect("three merges said nothing");
        assert!(
            (median - 20.0).abs() < f64::EPSILON,
            "the median was {median}"
        );
        assert_eq!(
            named,
            [
                "bbbbbbbbbbbb Merge bbbbbbbbbbbbbbbb: 60 min",
                "cccccccccccc Merge cccccccccccccccc: 20 min",
                "aaaaaaaaaaaa Merge aaaaaaaaaaaaaaaa: 10 min",
            ],
            "the changes weren't named slowest first"
        );
    }

    #[test]
    fn an_even_day_takes_the_two_middle_changes_mean() {
        let day = [change("a", 0, 600), change("b", 0, 1800)];
        let (median, _) = edit_to_merge(&day).expect("two merges said nothing");
        assert!(
            (median - 20.0).abs() < f64::EPSILON,
            "the median was {median}"
        );
    }

    #[test]
    fn a_day_with_no_merge_has_no_figure() {
        assert_eq!(edit_to_merge(&[]), None, "no merges gave a figure");
    }

    #[test]
    fn a_merge_before_its_edit_took_no_time() {
        let (median, _) = edit_to_merge(&[change("a", 900, 600)]).expect("one merge said nothing");
        assert!(median.abs() < f64::EPSILON, "the median was {median}");
    }
}
