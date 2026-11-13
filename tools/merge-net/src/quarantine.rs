//! What the quarantine list excuses: a whole test, or only the subtests
//! its entry names. On 12 November eleven launcher tests sat on the list
//! whole, some for two days after their owners' fixes merged, and merges
//! broke six of them in other subtests meanwhile: the net ran each twice,
//! saw them fail, and said every head passed. An entry naming `subtests`
//! excuses a failed run only when every subtest that failed in it is one
//! of them, so a new failure in the same test is searched for and
//! reverted like any other. An entry naming none excuses the whole test,
//! as every entry did before.

use std::collections::BTreeSet;

use serde_json::{Value, json};

/// The tests `list` excuses whole: those whose entries name no subtests.
#[must_use]
pub fn whole(list: &Value) -> Vec<String> {
    entries(list)
        .filter(|entry| named(entry).is_empty())
        .filter_map(|entry| entry["test"].as_str().map(str::to_owned))
        .collect()
}

/// The subtests `list`'s entry for `test` names, none when it lists the
/// test whole or not at all.
#[must_use]
pub fn subtests_of(list: &Value, test: &str) -> BTreeSet<String> {
    entries(list)
        .find(|entry| entry["test"] == test)
        .map(named)
        .unwrap_or_default()
}

/// Whether a failed run whose log names `failing` is excused by an entry
/// naming `listed`: every subtest that failed is listed. A run naming no
/// failed subtest, as when it timed out or didn't build, is excused by
/// nothing, since what broke in it can't be told.
#[must_use]
pub fn excuses(listed: &BTreeSet<String>, failing: &BTreeSet<String>) -> bool {
    !failing.is_empty() && failing.is_subset(listed)
}

/// List `test` in `entries` with `fields`, excusing `subtests` of it, or
/// the whole test when none are named. A test already listed by its
/// subtests has these added to them, and one listed whole stays whole,
/// so each test keeps one entry. Gives whether the list changed.
pub fn list(
    entries: &mut Vec<Value>,
    test: &str,
    subtests: &BTreeSet<String>,
    fields: Value,
) -> bool {
    let Some(entry) = entries.iter_mut().find(|entry| entry["test"] == test) else {
        let mut entry = fields;
        entry["test"] = json!(test);
        if !subtests.is_empty() {
            entry["subtests"] = json!(subtests);
        }
        entries.push(entry);
        return true;
    };
    let kept = named(entry);
    if kept.is_empty() || subtests.is_empty() || subtests.is_subset(&kept) {
        return false;
    }
    entry["subtests"] = json!(&kept | subtests);
    true
}

fn entries(list: &Value) -> impl Iterator<Item = &Value> {
    list.as_array().into_iter().flatten()
}

fn named(entry: &Value) -> BTreeSet<String> {
    entry["subtests"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|subtest| subtest.as_str().map(str::to_owned))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(names: &str) -> BTreeSet<String> {
        names.split_whitespace().map(str::to_owned).collect()
    }

    fn sample() -> Value {
        json!([
            {"test": "vm-a"},
            {"test": "vm-b", "subtests": ["test_x", "test_y"]},
        ])
    }

    #[test]
    fn only_entries_naming_no_subtests_excuse_a_whole_test() {
        assert_eq!(
            whole(&sample()),
            ["vm-a"],
            "vm-b names subtests, so isn't whole"
        );
        assert!(
            whole(&json!({})).is_empty(),
            "a list that isn't one excuses nothing"
        );
    }

    #[test]
    fn an_entry_gives_the_subtests_it_names() {
        assert_eq!(subtests_of(&sample(), "vm-b"), set("test_x test_y"));
        assert!(
            subtests_of(&sample(), "vm-a").is_empty(),
            "vm-a is listed whole"
        );
        assert!(
            subtests_of(&sample(), "vm-c").is_empty(),
            "vm-c isn't listed"
        );
    }

    #[test]
    fn a_run_is_excused_only_when_every_failed_subtest_is_listed() {
        let listed = set("test_x test_y");
        assert!(excuses(&listed, &set("test_x")), "test_x alone is listed");
        assert!(
            !excuses(&listed, &set("test_x test_z")),
            "test_z isn't listed"
        );
        assert!(
            !excuses(&listed, &set("")),
            "a run naming no failure can't be excused"
        );
    }

    #[test]
    fn listing_adds_an_entry_or_extends_one_naming_subtests() {
        let mut entries = sample().as_array().unwrap().clone();
        assert!(list(
            &mut entries,
            "vm-c",
            &set("test_q"),
            json!({"owner_pr": 3})
        ));
        assert_eq!(
            entries[2],
            json!({"test": "vm-c", "owner_pr": 3, "subtests": ["test_q"]})
        );
        assert!(list(&mut entries, "vm-b", &set("test_z"), json!({})));
        assert_eq!(
            subtests_of(&json!(entries), "vm-b"),
            set("test_x test_y test_z")
        );
        assert!(
            !list(&mut entries, "vm-b", &set("test_x"), json!({})),
            "test_x is listed"
        );
        assert!(
            !list(&mut entries, "vm-a", &set("test_q"), json!({})),
            "vm-a stays whole"
        );
        assert!(list(&mut entries, "vm-d", &set(""), json!({})));
        assert!(
            whole(&json!(entries)).contains(&"vm-d".to_owned()),
            "vm-d is listed whole"
        );
        assert_eq!(entries.len(), 4, "each test keeps one entry");
    }
}
