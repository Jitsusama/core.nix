//! The hour's figures, held against their targets, for cadence's hourly
//! read: each repository's measures beside the target its machine set,
//! the figure the hour before, the owner and the first files it counted,
//! those holding more than the day before first, what each of those holds
//! by who owns it, and the one furthest past its target. It's
//! `hour.json`, rewritten whole each hour, so it always says this hour and
//! the last.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

/// How many of a measure's files the hour names; the day's named file
/// holds the rest.
pub const FILES: usize = 20;

/// How many of the newest day records a target is ratcheted by: a week,
/// so a gain holds a week after it's lost, and a change to how a measure
/// counts is forgotten within one.
pub const DAYS: usize = 7;

/// The measures of the day's merges, which swing with how many merged
/// and what, so their lowest says nothing of what's kept; they're held
/// only to the target set.
const SWINGING: [&str; 3] = ["edit-to-merge", "files-per-change", "shared-files"];

/// The folders a repository's owners are given that own nothing, which
/// is always a mistake, so it's held to none where no target is set.
pub const STRAY_OWNERS: &str = "stray-owners";

/// The files whose comments the stale reader couldn't know, which only
/// its own misreading leaves, so it's held to none where no target is set.
pub const UNREAD_COMMENTS: &str = "unread-comments";

/// The measures that are always a mistake, held to none unless a target
/// is set for them.
const HELD_TO_NONE: [&str; 2] = [STRAY_OWNERS, UNREAD_COMMENTS];

/// What a repository's machine judges it by: a target for some of its
/// measures, who owns each, by measure or for the whole repository, and
/// who owns each folder, by its path from the repository's top.
#[derive(Debug, Default, PartialEq)]
pub struct Judged {
    pub targets: BTreeMap<String, f64>,
    pub owner: Option<String>,
    pub owners: BTreeMap<String, String>,
    pub folders: BTreeMap<String, String>,
}

/// The hour at `at`: every measure of every repository, held against its
/// target, and the worst, from the figures and names read and the hour
/// before, `before`, as it was written, if it could be read.
///
/// A measure's files name first each entry of a file it counts more of
/// than in `earlier`, the newest day's counts by file before today's, so
/// what was just added leads rather than what sorts first, among all of
/// a measure's names rather than the first the day's record keeps. Lines
/// move as files change, so a file is held against itself whole. Every
/// entry of those files goes to who owns it, in `by_owner`: the owner of
/// its deepest folder the repository's `folders` name, else the measure's;
/// with no counts the day before, there's no `by_owner`.
///
/// A target ratchets down to the lowest figure `days`, the newest day
/// records, held two in a row, so a gain is kept rather than lost back to
/// the target set, which it then says as `set`; the merges' figures,
/// which swing day to day, are held only to the target set.
///
/// The worst is the measure most past its target for the target's size,
/// so 22 tests past 21 outranks 227 lines past 7,000; a measure with no
/// target, or at or under it, is never the worst, and with none past
/// theirs the worst is null.
pub fn hour(
    at: &str,
    figures: &BTreeMap<String, BTreeMap<&'static str, f64>>,
    named: &BTreeMap<String, BTreeMap<&'static str, Vec<String>>>,
    judged: &BTreeMap<String, Judged>,
    before: Option<&Value>,
    earlier: Option<&Value>,
    days: &[Value],
) -> Value {
    let unjudged = Judged::default();
    let mut measures = Vec::new();
    let mut worst: Option<(f64, usize)> = None;
    for (repo, read) in figures {
        let judged = judged.get(repo).unwrap_or(&unjudged);
        for (measure, now) in read {
            let set = judged
                .targets
                .get(*measure)
                .copied()
                .or_else(|| HELD_TO_NONE.contains(measure).then_some(0.0));
            let lowest = held(days, repo, measure).filter(|_| !SWINGING.contains(measure));
            let target = set.map(|set| lowest.map_or(set, |lowest| set.min(lowest)));
            let owner = judged.owners.get(*measure).or(judged.owner.as_ref());
            let counted = named.get(repo).and_then(|named| named.get(measure));
            let had = earlier.map(|earlier| &earlier[repo][*measure]);
            let gained = counted.and_then(|counted| gained_files(counted, had));
            let files =
                counted.map_or_else(Vec::new, |counted| gained_first(counted, gained.as_ref()));
            if let Some(target) = target.filter(|target| now > target) {
                let past = (now - target) / target.max(1.0);
                if worst.is_none_or(|(most, _)| past > most) {
                    worst = Some((past, measures.len()));
                }
            }
            let mut said = json!({
                "repo": repo,
                "measure": measure,
                "now": now,
                "target": target,
                "was": was(before, repo, measure),
                "owner": owner,
                "files": files,
            });
            if target != set {
                said["set"] = json!(set);
            }
            if let (Some(counted), Some(gained)) = (counted, &gained) {
                said["by_owner"] = json!(by_owner(counted, gained, &judged.folders, owner));
            }
            measures.push(said);
        }
    }
    let worst = worst.map_or(Value::Null, |(_, index)| measures[index].clone());
    json!({"at": at, "measures": measures, "worst": worst})
}

/// The lowest figure `days` held for a measure two records in a row, the
/// higher of each two, so one odd read doesn't lower it.
fn held(days: &[Value], repo: &str, measure: &str) -> Option<f64> {
    let figures: Vec<Option<f64>> = days.iter().map(|day| day[repo][measure].as_f64()).collect();
    figures
        .windows(2)
        .filter_map(|two| Some(two[0]?.max(two[1]?)))
        .reduce(f64::min)
}

/// The files `counted` names more entries of than `had`, the day before's
/// count of each file; with no counts the day before, none.
fn gained_files<'a>(counted: &'a [String], had: Option<&Value>) -> Option<BTreeSet<&'a str>> {
    let had = had.and_then(Value::as_object)?;
    let now = per_file(counted.iter().map(String::as_str));
    let more = |(file, now): &(&str, usize)| {
        let was = had.get(*file).and_then(Value::as_u64).unwrap_or(0);
        u64::try_from(*now).unwrap_or(u64::MAX) > was
    };
    Some(now.into_iter().filter(more).map(|(file, _)| file).collect())
}

/// The first of a measure's entries, each of a file that `gained`, first,
/// in their order, then the rest.
fn gained_first<'a>(counted: &'a [String], gained: Option<&BTreeSet<&str>>) -> Vec<&'a String> {
    let Some(gained) = gained else {
        return counted.iter().take(FILES).collect();
    };
    let (first, rest): (Vec<&String>, Vec<&String>) = counted
        .iter()
        .partition(|entry| gained.contains(file_of(entry)));
    first.into_iter().chain(rest).take(FILES).collect()
}

/// Each entry of a file that `gained`, by who owns it: the owner of its
/// deepest folder of `folders`, else `owner`; with neither, it's left out.
fn by_owner<'a>(
    counted: &'a [String],
    gained: &BTreeSet<&str>,
    folders: &BTreeMap<String, String>,
    owner: Option<&String>,
) -> BTreeMap<String, Vec<&'a String>> {
    let mut each: BTreeMap<String, Vec<&String>> = BTreeMap::new();
    for entry in counted {
        let file = file_of(entry);
        if !gained.contains(file) {
            continue;
        }
        let within = |folder: &&String| holds(folder, file);
        let deepest = folders
            .iter()
            .filter(|(folder, _)| within(folder))
            .max_by_key(|(folder, _)| folder.len());
        if let Some(whose) = deepest.map(|(_, whose)| whose).or(owner) {
            each.entry(whose.clone()).or_default().push(entry);
        }
    }
    each
}

/// The folders of `folders` holding none of `paths`, read as `by_owner`
/// reads an entry's file: a folder misspelt, gone or empty, which owns
/// nothing, so what it should hold goes to the owner of a folder above
/// it, else the measure's or the repository's.
pub fn astray<'a>(folders: &'a BTreeMap<String, String>, paths: &[String]) -> Vec<&'a str> {
    folders
        .keys()
        .filter(|folder| !paths.iter().any(|path| holds(folder, file_of(path))))
        .map(String::as_str)
        .collect()
}

/// Whether `folder` holds `file`, by whole parts of its path.
fn holds(folder: &str, file: &str) -> bool {
    file.strip_prefix(folder)
        .is_some_and(|rest| rest.starts_with('/'))
}

/// How many entries name each file.
pub fn per_file<'a>(entries: impl Iterator<Item = &'a str>) -> BTreeMap<&'a str, usize> {
    let mut counts = BTreeMap::new();
    for entry in entries {
        *counts.entry(file_of(entry)).or_default() += 1;
    }
    counts
}

/// The file an entry names: what comes before its first colon, if any.
fn file_of(entry: &str) -> &str {
    entry.split(':').next().unwrap_or(entry)
}

/// A measure's figure in the hour before, if that hour was read whole
/// enough to hold it.
fn was(before: Option<&Value>, repo: &str, measure: &str) -> Value {
    before
        .and_then(|before| before["measures"].as_array())
        .and_then(|measures| {
            measures
                .iter()
                .find(|each| each["repo"] == repo && each["measure"] == measure)
        })
        .map(|each| each["now"].clone())
        .filter(Value::is_number)
        .unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::NAMED;

    #[test]
    fn a_folder_owned_but_holding_no_file_is_astray() {
        let folders: BTreeMap<String, String> = [
            ("modules/net", "someone"),
            ("modules/ne", "a typo"),
            ("modules/gone", "another"),
            ("modules/net/switch", "a third"),
            (
                "modules/odd:name",
                "a fourth, whose entries by_owner cuts at the colon",
            ),
            ("", "a fifth, given the empty folder"),
        ]
        .into_iter()
        .map(|(folder, whose)| (folder.to_owned(), whose.to_owned()))
        .collect();
        let paths = [
            "modules/net/home.nix".to_owned(),
            "modules/net/switch/main.rs".to_owned(),
            "modules/gone.nix".to_owned(),
            "modules/odd:name/home.nix".to_owned(),
        ];
        assert_eq!(
            astray(&folders, &paths),
            ["", "modules/gone", "modules/ne", "modules/odd:name"],
            "a folder by_owner gives entries to was called astray, or one it gives none wasn't"
        );
    }

    fn figures(
        each: &[(&str, &'static str, f64)],
    ) -> BTreeMap<String, BTreeMap<&'static str, f64>> {
        let mut figures: BTreeMap<String, BTreeMap<&'static str, f64>> = BTreeMap::new();
        for (repo, measure, figure) in each {
            figures
                .entry((*repo).to_owned())
                .or_default()
                .insert(measure, *figure);
        }
        figures
    }

    fn judged(targets: &[(&str, f64)], owner: Option<&str>) -> Judged {
        Judged {
            targets: targets
                .iter()
                .map(|(measure, figure)| ((*measure).to_owned(), *figure))
                .collect(),
            owner: owner.map(str::to_owned),
            ..Judged::default()
        }
    }

    /// The hour of `read` and `named`, with no targets or hour before, since
    /// the day `earlier` counted by file.
    fn since_earlier(
        read: &BTreeMap<String, BTreeMap<&'static str, f64>>,
        named: &BTreeMap<String, BTreeMap<&'static str, Vec<String>>>,
        earlier: &Value,
    ) -> Value {
        hour(
            "now",
            read,
            named,
            &BTreeMap::new(),
            None,
            Some(earlier),
            &[],
        )
    }

    fn measure<'a>(hour: &'a Value, repo: &str, measure: &str) -> &'a Value {
        hour["measures"]
            .as_array()
            .expect("no measures")
            .iter()
            .find(|each| each["repo"] == repo && each["measure"] == measure)
            .unwrap_or_else(|| panic!("{repo}'s {measure} wasn't in the hour"))
    }

    #[test]
    fn each_measure_says_its_target_the_hour_before_its_owner_and_files() {
        let read = figures(&[
            ("core", "long-files", 98.0),
            ("core", "suppressions", 487.0),
        ]);
        let named = BTreeMap::from([(
            "core".to_owned(),
            BTreeMap::from([(
                "long-files",
                (0..30).map(|n| format!("f{n}.rs")).collect::<Vec<_>>(),
            )]),
        )]);
        let mut core = judged(&[("long-files", 96.0)], Some("someone"));
        core.owners
            .insert("suppressions".to_owned(), "another".to_owned());
        let judged = BTreeMap::from([("core".to_owned(), core)]);
        let before = json!({"at": "then", "measures": [
            {"repo": "core", "measure": "long-files", "now": 97.0},
        ]});
        let hour = hour("now", &read, &named, &judged, Some(&before), None, &[]);
        assert_eq!(hour["at"], "now", "the hour didn't say when it was read");
        let long = measure(&hour, "core", "long-files");
        assert_eq!(
            (&long["now"], &long["target"], &long["was"], &long["owner"]),
            (&json!(98.0), &json!(96.0), &json!(97.0), &json!("someone")),
            "a measure's figure, target, hour before or owner was lost: {long}"
        );
        assert_eq!(
            long["files"].as_array().map(Vec::len),
            Some(FILES),
            "the hour didn't name the first of a measure's files: {long}"
        );
        let suppressions = measure(&hour, "core", "suppressions");
        assert_eq!(
            (&suppressions["target"], &suppressions["was"]),
            (&Value::Null, &Value::Null),
            "a measure with no target or no hour before wasn't said as none: {suppressions}"
        );
        assert_eq!(
            suppressions["owner"], "another",
            "a measure's own owner didn't come before its repository's"
        );
        assert_eq!(
            suppressions["files"],
            json!([]),
            "a measure naming nothing didn't say so"
        );
    }

    #[test]
    fn the_worst_is_furthest_past_its_target_for_its_size() {
        // 7,227 lines past 7,000 is 3% over; 43 tests past 21 is double.
        let read = figures(&[
            ("core", "duplicated-lines", 7227.0),
            ("core", "quarantined-tests", 43.0),
            ("core", "suppressions", 900.0),
            ("pi", "long-files", 1.0),
        ]);
        let judged = BTreeMap::from([
            (
                "core".to_owned(),
                judged(
                    &[("duplicated-lines", 7000.0), ("quarantined-tests", 21.0)],
                    None,
                ),
            ),
            ("pi".to_owned(), judged(&[("long-files", 0.0)], None)),
        ]);
        let hour = hour("now", &read, &BTreeMap::new(), &judged, None, None, &[]);
        assert_eq!(
            (&hour["worst"]["repo"], &hour["worst"]["measure"]),
            (&json!("core"), &json!("quarantined-tests")),
            "the worst wasn't the measure furthest past its target: {}",
            hour["worst"]
        );
        assert_eq!(
            hour["worst"]["owner"],
            Value::Null,
            "a measure no one owns was given an owner"
        );
    }

    #[test]
    fn a_stray_owner_or_unread_file_is_held_to_none_unless_a_target_is_set() {
        let read = figures(&[
            ("core", STRAY_OWNERS, 1.0),
            ("pi", STRAY_OWNERS, 0.0),
            ("pi", UNREAD_COMMENTS, 0.0),
            ("set", STRAY_OWNERS, 1.0),
        ]);
        let judged = BTreeMap::from([("set".to_owned(), judged(&[(STRAY_OWNERS, 2.0)], None))]);
        let hour = hour("now", &read, &BTreeMap::new(), &judged, None, None, &[]);
        let targets =
            ["core", "pi", "set"].map(|repo| &measure(&hour, repo, STRAY_OWNERS)["target"]);
        assert_eq!(
            targets,
            [&json!(0.0), &json!(0.0), &json!(2.0)],
            "a stray owner wasn't held to none, or a target set for it was lost"
        );
        assert_eq!(
            measure(&hour, "pi", UNREAD_COMMENTS)["target"],
            json!(0.0),
            "a file whose comments couldn't be read wasn't held to none"
        );
        assert_eq!(
            (&hour["worst"]["repo"], &hour["worst"]["measure"]),
            (&json!("core"), &json!(STRAY_OWNERS)),
            "a stray owner with no target set wasn't the worst: {}",
            hour["worst"]
        );
    }

    #[test]
    fn no_measure_is_worst_when_none_is_past_its_target() {
        let read = figures(&[
            ("core", "long-files", 96.0),
            ("core", "suppressions", 900.0),
        ]);
        let judged = BTreeMap::from([("core".to_owned(), judged(&[("long-files", 96.0)], None))]);
        let hour = hour("now", &read, &BTreeMap::new(), &judged, None, None, &[]);
        assert_eq!(
            hour["worst"],
            Value::Null,
            "a measure at its target, or with none, was called the worst"
        );
    }

    #[test]
    fn an_hour_before_that_cant_be_read_leaves_was_unsaid() {
        let read = figures(&[("core", "long-files", 96.0)]);
        let before = json!({"measures": "not a list"});
        let hour = hour(
            "now",
            &read,
            &BTreeMap::new(),
            &BTreeMap::new(),
            Some(&before),
            None,
            &[],
        );
        assert_eq!(
            measure(&hour, "core", "long-files")["was"],
            Value::Null,
            "an hour before out of shape gave a figure"
        );
    }

    #[test]
    fn what_a_file_gained_since_the_day_before_is_named_first() {
        let read = figures(&[("core", "suppressions", 4.0), ("core", "tf-invalid", 1.0)]);
        let named = BTreeMap::from([(
            "core".to_owned(),
            BTreeMap::from([
                (
                    "suppressions",
                    ["a.sh:3", "b.rs:4", "b.rs:9", "c.py:2"]
                        .map(str::to_owned)
                        .to_vec(),
                ),
                ("tf-invalid", vec!["ws: Missing provider".to_owned()]),
            ]),
        )]);
        let earlier = json!({"core": {"suppressions": {"a.sh": 1, "b.rs": 1}, "tf-invalid": 3}});
        let hour = since_earlier(&read, &named, &earlier);
        assert_eq!(
            measure(&hour, "core", "suppressions")["files"],
            json!(["b.rs:4", "b.rs:9", "c.py:2", "a.sh:3"]),
            "what files gained since the day before wasn't named first, in order"
        );
        assert_eq!(
            measure(&hour, "core", "tf-invalid")["files"],
            json!(["ws: Missing provider"]),
            "an earlier record out of shape lost a measure's files"
        );
    }

    #[test]
    fn a_file_past_the_days_named_is_named_first_when_it_gained() {
        let read = figures(&[("core", "suppressions", 251.0)]);
        let mut every: Vec<String> = (0..250).map(|line| format!("a.rs:{line}")).collect();
        every.push("z.rs:1".to_owned());
        let named =
            BTreeMap::from([("core".to_owned(), BTreeMap::from([("suppressions", every)]))]);
        let earlier = json!({"core": {"suppressions": {"a.rs": 250}}});
        let hour = since_earlier(&read, &named, &earlier);
        assert_eq!(
            measure(&hour, "core", "suppressions")["files"][0],
            "z.rs:1",
            "a file past the first {NAMED} names wasn't named first when it gained"
        );
    }

    #[test]
    fn each_gained_entry_goes_to_the_owner_of_its_deepest_folder() {
        let read = figures(&[("core", "suppressions", 5.0)]);
        let entries = [
            "modules/net/a.rs:1",
            "modules/net/switch/b.py:2",
            "modules/netting.rs:3",
            "c.sh:4",
            "old.rs:5",
        ];
        let named = BTreeMap::from([(
            "core".to_owned(),
            BTreeMap::from([("suppressions", entries.map(str::to_owned).to_vec())]),
        )]);
        let mut core = judged(&[], Some("me"));
        core.folders = BTreeMap::from([
            ("modules/net".to_owned(), "net".to_owned()),
            ("modules/net/switch".to_owned(), "switch".to_owned()),
        ]);
        let judged = BTreeMap::from([("core".to_owned(), core)]);
        let earlier = json!({"core": {"suppressions": {"old.rs": 1}}});
        let hour = hour("now", &read, &named, &judged, None, Some(&earlier), &[]);
        assert_eq!(
            measure(&hour, "core", "suppressions")["by_owner"],
            json!({
                "net": ["modules/net/a.rs:1"],
                "switch": ["modules/net/switch/b.py:2"],
                "me": ["modules/netting.rs:3", "c.sh:4"],
            }),
            "a gained entry wasn't given to its deepest folder's owner, or the measure's"
        );
    }

    #[test]
    fn with_no_counts_the_day_before_no_owner_is_given_entries() {
        let read = figures(&[("core", "suppressions", 1.0)]);
        let named = BTreeMap::from([(
            "core".to_owned(),
            BTreeMap::from([("suppressions", vec!["a.rs:1".to_owned()])]),
        )]);
        let judged = BTreeMap::from([("core".to_owned(), judged(&[], Some("me")))]);
        let hour = hour("now", &read, &named, &judged, None, None, &[]);
        assert_eq!(
            measure(&hour, "core", "suppressions").get("by_owner"),
            None,
            "entries were given owners with nothing to say what they gained"
        );
    }

    #[test]
    fn a_figure_held_two_days_running_is_the_lowest_held() {
        let days = |each: &[Option<f64>]| -> Vec<Value> {
            each.iter()
                .map(|figure| figure.map_or(json!({}), |f| json!({"core": {"long-files": f}})))
                .collect()
        };
        let cases = [
            (
                days(&[Some(10.0), Some(10.0), Some(0.0), Some(0.0)]),
                Some(0.0),
            ),
            (
                days(&[Some(10.0), Some(0.0), Some(10.0), Some(0.0)]),
                Some(10.0),
            ),
            (days(&[Some(9.0), None, Some(3.0)]), None),
            (days(&[Some(4.0)]), None),
        ];
        for (given, lowest) in cases {
            assert_eq!(
                held(&given, "core", "long-files"),
                lowest,
                "the lowest held two records running wasn't found in {given:?}"
            );
        }
    }

    #[test]
    fn a_target_ratchets_down_to_what_was_held_but_not_a_merges_figure() {
        let read = figures(&[
            ("dots", "duplicated-lines", 3.0),
            ("dots", "edit-to-merge", 3.0),
        ]);
        let set = judged(&[("duplicated-lines", 10.0), ("edit-to-merge", 10.0)], None);
        let judged = BTreeMap::from([("dots".to_owned(), set)]);
        let day = json!({"dots": {"duplicated-lines": 0.0, "edit-to-merge": 1.0}});
        let hour = hour(
            "now",
            &read,
            &BTreeMap::new(),
            &judged,
            None,
            None,
            &[day.clone(), day],
        );
        let lines = measure(&hour, "dots", "duplicated-lines");
        assert_eq!(
            (&lines["target"], &lines["set"]),
            (&json!(0.0), &json!(10.0)),
            "the target wasn't ratcheted to what was held, beside the one set"
        );
        assert_eq!(
            hour["worst"]["measure"], "duplicated-lines",
            "a figure past its ratcheted target wasn't the worst"
        );
        let merges = measure(&hour, "dots", "edit-to-merge");
        assert_eq!(
            (&merges["target"], merges.get("set")),
            (&json!(10.0), None),
            "a merges figure was ratcheted"
        );
    }
}
