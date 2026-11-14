//! What the quarantine list excuses: a whole test, or only the subtests
//! its entry names. On 12 November eleven launcher tests sat on the list
//! whole, some for two days after their owners' fixes merged, and merges
//! broke six of them in other subtests meanwhile: the net ran each twice,
//! saw them fail, and said every head passed. An entry naming `subtests`
//! excuses a failed run only when every subtest that failed in it is one
//! of them, so a new failure in the same test is searched for and
//! reverted like any other. An entry naming none excuses the whole test,
//! as every entry did before.
//!
//! A test may have several entries, one for each time the net listed it,
//! each owned by the PR that broke what it lists. On 13 November #4054
//! filed three subtests #4004 broke into vm-desktop-captures' entry, owned
//! by #3737, so the net named one owner and wrote another, and #3737 taking
//! its own subtests off would have taken #4004's with them. Each listing
//! now adds an entry of its own for the subtests not yet held, and a test's
//! entries are read together. A reader not yet built with this reads only
//! a test's first entry, so it excuses less, never more.

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

/// The subtests `list`'s entries for `test` name between them, none when
/// it lists the test only whole or not at all.
#[must_use]
pub fn subtests_of(list: &Value, test: &str) -> BTreeSet<String> {
    entries(list)
        .filter(|entry| entry["test"] == test)
        .flat_map(named)
        .collect()
}

/// Whether a failed run whose log names `failing` is excused by an entry
/// naming `listed`: every subtest that failed is listed. A run naming no
/// failed subtest, as when it timed out or didn't build, is excused by
/// nothing, since what broke in it can't be told.
#[must_use]
pub fn excuses(listed: &BTreeSet<String>, failing: &BTreeSet<String>) -> bool {
    !failing.is_empty() && failing.is_subset(listed)
}

/// Whether `list` still holds what failed in `test`: the whole test, or,
/// where `failing` names subtests, every one of them between its entries.
/// A failure naming none is held by any entry for the test, as every
/// entry held a whole test before entries named subtests.
#[must_use]
pub fn covers(list: &Value, test: &str, failing: &BTreeSet<String>) -> bool {
    if whole(list).iter().any(|held| held == test) {
        return true;
    }
    if failing.is_empty() {
        return entries(list).any(|entry| entry["test"] == test);
    }
    failing.is_subset(&subtests_of(list, test))
}

/// List `test` in `entries` with `fields`, its owner and reason among
/// them, excusing `subtests` of it, or the whole test when none are named.
/// A test already listed gets an entry of its own for the subtests not
/// held yet, leaving every other owner's entry as it was; one listed whole
/// stays whole, and one listed by its subtests isn't listed whole. Gives
/// whether the list changed.
pub fn list(
    entries: &mut Vec<Value>,
    test: &str,
    subtests: &BTreeSet<String>,
    fields: Value,
) -> bool {
    let earlier: Vec<&Value> = entries
        .iter()
        .filter(|entry| entry["test"] == test)
        .collect();
    let fresh: BTreeSet<String> = if earlier.is_empty() {
        subtests.clone()
    } else if earlier.iter().any(|entry| named(entry).is_empty()) || subtests.is_empty() {
        return false;
    } else {
        let held: BTreeSet<String> = earlier.into_iter().flat_map(named).collect();
        let fresh: BTreeSet<String> = subtests.difference(&held).cloned().collect();
        if fresh.is_empty() {
            return false;
        }
        fresh
    };
    let mut entry = fields;
    entry["test"] = json!(test);
    if !fresh.is_empty() {
        entry["subtests"] = json!(fresh);
    }
    entries.push(entry);
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
    fn a_list_covers_a_failure_only_while_it_holds_every_failed_subtest() {
        let list = json!([
            {"test": "vm-a"},
            {"test": "vm-b", "owner_pr": 1, "subtests": ["test_x"]},
            {"test": "vm-b", "owner_pr": 2, "subtests": ["test_y"]},
        ]);
        assert!(
            covers(&list, "vm-a", &set("test_q")),
            "vm-a is listed whole"
        );
        assert!(
            covers(&list, "vm-b", &set("test_x test_y")),
            "vm-b's two entries hold both its failures between them"
        );
        assert!(
            !covers(&list, "vm-b", &set("test_x test_z")),
            "test_z isn't held"
        );
        assert!(
            covers(&list, "vm-b", &set("")),
            "a failure naming no subtest is held by any entry for its test"
        );
        assert!(!covers(&list, "vm-c", &set("")), "vm-c isn't listed");
    }

    #[test]
    fn a_tests_entries_give_every_subtest_they_name_between_them() {
        let list = json!([
            {"test": "vm-b", "owner_pr": 1, "subtests": ["test_x"]},
            {"test": "vm-c", "subtests": ["test_q"]},
            {"test": "vm-b", "owner_pr": 2, "subtests": ["test_y"]},
        ]);
        assert_eq!(
            subtests_of(&list, "vm-b"),
            set("test_x test_y"),
            "a second owner's entry for vm-b wasn't read"
        );
        assert!(
            whole(&list).is_empty(),
            "an entry naming subtests read as whole"
        );
    }

    #[test]
    fn listing_adds_an_entry_for_what_isnt_held_yet() {
        let mut entries = sample().as_array().unwrap().clone();
        assert!(
            list(&mut entries, "vm-c", &set("test_q"), json!({"owner_pr": 3})),
            "a test not listed wasn't added"
        );
        assert_eq!(
            entries[2],
            json!({"test": "vm-c", "owner_pr": 3, "subtests": ["test_q"]})
        );
        assert!(
            list(
                &mut entries,
                "vm-b",
                &set("test_y test_z"),
                json!({"owner_pr": 4})
            ),
            "a new subtest of a listed test wasn't listed"
        );
        assert_eq!(
            entries.get(3),
            Some(&json!({"test": "vm-b", "owner_pr": 4, "subtests": ["test_z"]})),
            "a new subtest wasn't listed under its own owner, alone"
        );
        assert_eq!(
            entries[1],
            json!({"test": "vm-b", "subtests": ["test_x", "test_y"]}),
            "the first owner's entry changed"
        );
        assert_eq!(
            subtests_of(&json!(entries), "vm-b"),
            set("test_x test_y test_z")
        );
        assert!(
            !list(&mut entries, "vm-b", &set("test_x test_z"), json!({})),
            "test_x and test_z are listed"
        );
        assert!(
            !list(&mut entries, "vm-a", &set("test_q"), json!({})),
            "vm-a stays whole"
        );
        assert!(
            list(&mut entries, "vm-d", &set(""), json!({})),
            "a test with no failed subtest named wasn't added"
        );
        assert!(
            whole(&json!(entries)).contains(&"vm-d".to_owned()),
            "vm-d is listed whole"
        );
        assert_eq!(
            entries.len(),
            5,
            "a listing that changed nothing added an entry"
        );
    }
}
