//! How long the changes a repository merged took from their first edit to
//! their merge, the speed half of its health: a gate or a review that
//! grows slower shows here the day it does, as a file grown long shows in
//! measure.rs. A change's first edit is its first commit's author time,
//! which a rebase keeps, and its merge the merge commit's own time.
//!
//! And which files many of the day's changes had to touch: a file a tenth
//! of them changed is one every change queues on, rebasing over the
//! others' edits and waiting on their gates, so one becoming shared is
//! named the day it does, before it's felt in every change's wait.
//!
//! And how many files a change had to touch: a design whose pieces each
//! stand alone lets a change touch few, so the median rising is drift the
//! day it starts. Core's 451 merges to 12 November touched a median of 5,
//! a tenth of them 14 or more.

use std::collections::BTreeMap;

use crate::measure::NAMED;

/// A change merged into the line read: its merge commit, its title, when
/// its first commit was authored and when it merged, in Unix seconds, and
/// the files it changed.
#[derive(Debug, PartialEq)]
pub struct Merged {
    pub merge: String,
    pub title: String,
    pub first: i64,
    pub landed: i64,
    pub files: Vec<String>,
}

/// A file is shared when at least this many of the day's changes touched
/// it, one in ten,
const SHARE: usize = 10;
/// and at least this many, so a day of three merges shares no file.
const FEWEST: usize = 5;

/// How many files the day's changes shared, and each with how many of
/// them touched it, most first; none when nothing merged.
pub fn shared_files(merged: &[Merged]) -> Option<(f64, Vec<String>)> {
    if merged.is_empty() {
        return None;
    }
    let mut touched: BTreeMap<&str, usize> = BTreeMap::new();
    for change in merged {
        for file in &change.files {
            *touched.entry(file).or_insert(0) += 1;
        }
    }
    let mut shared: Vec<(&str, usize)> = touched
        .into_iter()
        .filter(|(_, times)| *times >= FEWEST && times * SHARE >= merged.len())
        .collect();
    shared.sort_by_key(|(_, times)| std::cmp::Reverse(*times));
    let named: Vec<String> = shared
        .iter()
        .take(NAMED)
        .map(|(file, times)| format!("{file}: {times} of {} changes", merged.len()))
        .collect();
    let count = u32::try_from(shared.len()).unwrap_or(u32::MAX);
    Some((f64::from(count), named))
}

/// The day's median count of files a change touched, and the widest
/// changes with theirs, widest first; none when nothing merged.
pub fn files_per_change(merged: &[Merged]) -> Option<(f64, Vec<String>)> {
    ranked(merged, " files", |change| {
        f64::from(u32::try_from(change.files.len()).unwrap_or(u32::MAX))
    })
}

/// The day's median minutes from first edit to merge, and each change
/// with its minutes, slowest first; none when nothing merged.
pub fn edit_to_merge(merged: &[Merged]) -> Option<(f64, Vec<String>)> {
    ranked(merged, " min", |change| {
        minutes(change.landed - change.first)
    })
}

/// The median of what `measure` gives each change, and the changes with
/// theirs in `unit`, most first; none when nothing merged.
fn ranked(
    merged: &[Merged],
    unit: &str,
    measure: impl Fn(&Merged) -> f64,
) -> Option<(f64, Vec<String>)> {
    let mut each: Vec<(f64, &Merged)> = merged
        .iter()
        .map(|change| (measure(change), change))
        .collect();
    each.sort_by(|one, other| other.0.total_cmp(&one.0));
    let middle = each.len() / 2;
    let median = match each.len() {
        0 => return None,
        count if count % 2 == 1 => each[middle].0,
        _ => f64::midpoint(each[middle - 1].0, each[middle].0),
    };
    let named = each
        .iter()
        .take(NAMED)
        .map(|(figure, change)| {
            let short = change.merge.get(..12).unwrap_or(&change.merge);
            format!("{short} {}: {figure}{unit}", change.title)
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
            files: Vec::new(),
        }
    }

    fn touching(files: &[&str]) -> Merged {
        Merged {
            files: files.iter().map(|file| (*file).to_owned()).collect(),
            ..change("a", 0, 0)
        }
    }

    #[test]
    fn a_file_a_tenth_of_the_days_changes_touched_is_shared() {
        let mut day: Vec<Merged> = (0..40)
            .map(|each| touching(&[&format!("own-{each}.rs")]))
            .collect();
        day.extend((0..5).map(|_| touching(&["table.nix", "list.json"])));
        day.extend((0..5).map(|_| touching(&["table.nix"])));
        day.extend((0..4).map(|_| touching(&["rare.md"])));
        let (shared, named) = shared_files(&day).expect("54 merges said nothing");
        assert!(
            (shared - 1.0).abs() < f64::EPSILON,
            "not only table.nix, 10 of 54, was shared: {shared}, {named:?}"
        );
        assert_eq!(
            named,
            ["table.nix: 10 of 54 changes"],
            "the shared file wasn't named with its count"
        );
    }

    #[test]
    fn a_few_changes_share_no_file() {
        let day: Vec<Merged> = (0..4).map(|_| touching(&["main.rs"])).collect();
        let (shared, _) = shared_files(&day).expect("four merges said nothing");
        assert!(shared.abs() < f64::EPSILON, "four changes shared {shared}");
        assert_eq!(shared_files(&[]), None, "no merges gave a figure");
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
    fn the_figure_is_the_middle_changes_files_and_the_widest_are_named() {
        let mut day = vec![
            touching(&["a.rs"]),
            touching(&["a.rs", "b.rs", "c.rs", "d.rs"]),
            touching(&["a.rs", "b.rs"]),
        ];
        day[1].merge = "bbbbbbbbbbbbbbbb".to_owned();
        day[1].title = "Wide".to_owned();
        let (median, named) = files_per_change(&day).expect("three merges said nothing");
        assert!(
            (median - 2.0).abs() < f64::EPSILON,
            "the median was {median}"
        );
        assert_eq!(
            named.first().map(String::as_str),
            Some("bbbbbbbbbbbb Wide: 4 files"),
            "the widest change wasn't named first: {named:?}"
        );
        assert_eq!(files_per_change(&[]), None, "no merges gave a figure");
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
