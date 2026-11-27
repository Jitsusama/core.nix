//! Which riders a red candidate's failed checks are theirs, by what each
//! rider's own pass named as moved, so the train drops those riders at
//! once rather than halving through all of them. A check one crate or
//! one area owns, such as `jobs-sorter-passes-clippy`, can only be broken
//! by a rider that moved it: a check whose derivation no rider moved
//! builds as main's, and changed-checks builds only what moved. A failure
//! none of them owns, or a log naming none, is left to the halving, as is
//! one every rider owns, as a lint over every file is.
//!
//! A candidate whose every failed check is on the quarantine list runs
//! once more before anyone is blamed, since such a check is known to fail
//! by itself.

use std::collections::BTreeSet;

/// What nix says of each derivation it couldn't build, as
/// `error: Cannot build '/nix/store/<hash>-<name>.drv'.`, and of those a
/// build asked for, as `error: build of '<path>.drv^*', ... failed`.
const UNBUILT: [&str; 2] = ["error: Cannot build ", "error: build of "];

/// What changed-checks says of a quarantined check that failed outside
/// what its entry excuses, after its name.
const UNEXCUSED: &str = "which main's quarantine doesn't excuse";

/// What changed-checks heads its table of the checks a run moved with.
const TABLE: &str = "KIND";

/// Derivations whose names aren't their check's.
const NAMED_APART: [(&str, &str); 1] = [("treefmt-check", "formatting")];

/// The checks the words of a failed gate say failed, or none when they
/// name no failed build, as when the gate failed before building or a
/// builder was lost. A failed derivation counts only when changed-checks'
/// last table names it as a check the change moved, so the parts and
/// files a check is built from, which fail beneath it, aren't counted:
/// the check they failed beneath is.
#[must_use]
pub fn failed(words: &str) -> Option<BTreeSet<String>> {
    let table = words
        .rfind(&format!("\n{TABLE} "))
        .map_or("", |at| &words[at + 1..]);
    let checks = moved(table);
    let unbuilt = words
        .lines()
        .filter_map(|line| UNBUILT.iter().find_map(|said| line.strip_prefix(said)))
        .flat_map(|said| said.split('\''))
        .filter_map(|word| {
            word.strip_suffix(".drv^*")
                .or_else(|| word.strip_suffix(".drv"))
        })
        .filter_map(|path| path.rsplit('/').next())
        .filter_map(|name| name.split_once('-').map(|(_, name)| check_of(name)));
    let unexcused = words
        .lines()
        .filter(|line| line.contains(UNEXCUSED))
        .filter_map(|line| line.strip_prefix("changed-checks: "))
        .filter_map(|said| said.split(' ').next())
        .map(str::to_owned);
    let found: BTreeSet<String> = unbuilt
        .chain(unexcused)
        .filter(|check| checks.contains(check))
        .collect();
    (!found.is_empty()).then_some(found)
}

/// The check a derivation called `name` is: itself, its version after
/// the last dash taken off, or the check it's named apart from.
fn check_of(name: &str) -> String {
    if let Some((_, check)) = NAMED_APART.iter().find(|(drv, _)| *drv == name) {
        return (*check).to_owned();
    }
    match name.rsplit_once('-') {
        Some((check, version)) if version.starts_with(|c: char| c.is_ascii_digit()) => {
            check.to_owned()
        }
        _ => name.to_owned(),
    }
}

/// The checks a pass's `printed` words, changed-checks' first table,
/// name as moved.
#[must_use]
pub fn moved(printed: &str) -> BTreeSet<String> {
    printed
        .lines()
        .skip_while(|line| !line.starts_with(TABLE))
        .skip(1)
        .filter_map(|line| line.split_whitespace().nth(1))
        .map(str::to_owned)
        .collect()
}

/// The checks every changed-checks pass in nix-offload's `record` of a
/// commit named as moved, or none when it kept no table.
#[must_use]
pub fn moved_in(record: &str) -> Option<BTreeSet<String>> {
    let found: BTreeSet<String> = record
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|line| line["printed"].as_str().map(moved))
        .flatten()
        .collect();
    (!found.is_empty()).then_some(found)
}

/// Of `riders`, each a head and what its own pass moved, none where it
/// kept none, the heads whose riders `failed` is theirs: each that moved
/// one of the failed checks, and each whose pass can't say. None when a
/// failed check is no rider's, or every rider's, so the search halves as
/// before.
#[must_use]
pub fn owners(
    failed: &BTreeSet<String>,
    riders: &[(String, Option<BTreeSet<String>>)],
) -> Option<Vec<String>> {
    let owns = |moved: &Option<BTreeSet<String>>, check: &String| {
        moved.as_ref().is_none_or(|moved| moved.contains(check))
    };
    let owned = failed
        .iter()
        .all(|check| riders.iter().any(|(_, moved)| owns(moved, check)));
    let heads: Vec<String> = riders
        .iter()
        .filter(|(_, moved)| failed.iter().any(|check| owns(moved, check)))
        .map(|(head, _)| head.clone())
        .collect();
    (owned && heads.len() < riders.len()).then_some(heads)
}

/// The checks the quarantine list `list` has an entry for, whole or by
/// their subtests.
#[must_use]
pub fn listed(list: &serde_json::Value) -> Vec<String> {
    list.as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry["test"].as_str().map(str::to_owned))
        .collect()
}

/// Whether every one of `failed` is on the quarantine list `listed`, the
/// tests its entries name, so the candidate runs again before anyone is
/// blamed. changed-checks already excuses a listed check failing only in
/// its listed subtests, so what's left failed outside them, where a
/// check known to fail by itself may still have.
#[must_use]
pub fn flaky(failed: &BTreeSet<String>, listed: &[String]) -> bool {
    !failed.is_empty() && failed.iter().all(|check| listed.contains(check))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    /// A gate's words: `said`, then changed-checks' verdict over a table
    /// of `checks`.
    fn gate(said: &str, checks: &[&str]) -> String {
        let table: String = checks
            .iter()
            .map(|check| format!("changed  {check}\n"))
            .collect();
        format!("{said}\u{2715} a check the change touched failed\nKIND     CHECK\n{table}")
    }

    // From the train's candidate 9f60a7682dac on 26 November.
    #[test]
    fn a_gates_failed_checks_are_read_from_nixs_last_word() {
        let words = gate(
            "error: Cannot build '/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-home-manager-files\
             .drv'.\n       Reason: 1 dependency failed.\n\
             error: build of '/nix/store/p1wxdcj29kr8cq9pz4j45dbcaksrqss5-jobs-sorter-passes-clippy\
             -0.1.0.drv^*', '/nix/store/waljvg8r2rjp4rjkcac1cs103fh0dd2q-treefmt-check.drv^*', \
             '/nix/store/yjs53dq06gzggf4v4d1l8z9m0bhk1zi9-the-margin-cat-sits-above-its-column\
             .drv^*' failed\n",
            &[
                "formatting",
                "jobs-sorter-passes-clippy",
                "the-margin-cat-sits-above-its-column",
                "bat-reads-its-theme",
            ],
        );
        assert_eq!(
            failed(&words),
            Some(set(&[
                "formatting",
                "jobs-sorter-passes-clippy",
                "the-margin-cat-sits-above-its-column"
            ])),
            "the failed checks weren't read by their checks' names alone"
        );
        assert_eq!(
            failed(&gate("", &["formatting"])),
            None,
            "a gate naming no failed build was read as naming some"
        );
    }

    // From the train's candidates c101c82f66a9 and 083fa77b9003.
    #[test]
    fn a_lone_failure_and_one_beneath_a_check_are_read_as_the_check() {
        let lone = gate(
            "error: Cannot build '/nix/store/g91db4n1if6imcbp0g02yf9l4cp96n7i-the-margin-cat-\
             follows-the-keys-on-niri.drv'.\n       Reason: builder failed with exit code 1.\n\
             changed-checks: the-margin-cat-follows-the-keys-on-niri failed in test_a, which \
             main's quarantine doesn't excuse, so it blocks\n",
            &["the-margin-cat-follows-the-keys-on-niri", "formatting"],
        );
        assert_eq!(
            failed(&lone),
            Some(set(&["the-margin-cat-follows-the-keys-on-niri"])),
            "a lone failure wasn't read"
        );
        let beneath = gate(
            "error: Cannot build '/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-the-launcher-\
             answers-whats-typed-on-niri-part-1.drv'.\n\
             error: Cannot build '/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-the-launcher-\
             answers-whats-typed-on-niri.drv'.\n       Reason: 1 dependency failed.\n",
            &["the-launcher-answers-whats-typed-on-niri"],
        );
        assert_eq!(
            failed(&beneath),
            Some(set(&["the-launcher-answers-whats-typed-on-niri"])),
            "a part's failure wasn't read as its check's"
        );
    }

    #[test]
    fn a_pass_names_what_it_moved_from_its_table() {
        let printed = "comparing\nKIND     CHECK\nchanged  formatting\nnew      \
                       jobs-sorter-passes-clippy\n";
        assert_eq!(
            moved(printed),
            set(&["formatting", "jobs-sorter-passes-clippy"]),
            "a pass's table wasn't read whole"
        );
        let record = format!(
            "{}\n{}\n",
            serde_json::json!({"printed": printed}),
            serde_json::json!({"check": "x", "dir": null})
        );
        assert_eq!(
            moved_in(&record),
            Some(set(&["formatting", "jobs-sorter-passes-clippy"])),
            "a record's passes weren't read"
        );
        assert_eq!(
            moved_in("{\"dir\": null}\n"),
            None,
            "a record with no table named some"
        );
    }

    #[test]
    fn a_failure_falls_only_on_the_riders_that_moved_it() {
        let riders = vec![
            (
                "a".to_owned(),
                Some(set(&["formatting", "jobs-sorter-passes-clippy"])),
            ),
            (
                "b".to_owned(),
                Some(set(&["formatting", "cadence-passes-clippy"])),
            ),
            ("c".to_owned(), Some(set(&["formatting"]))),
        ];
        assert_eq!(
            owners(&set(&["jobs-sorter-passes-clippy"]), &riders),
            Some(vec!["a".to_owned()]),
            "a crate's red wasn't put on the one rider moving it"
        );
        assert_eq!(
            owners(
                &set(&["jobs-sorter-passes-clippy", "cadence-passes-clippy"]),
                &riders
            ),
            Some(vec!["a".to_owned(), "b".to_owned()]),
            "two crates' reds weren't put on both their riders"
        );
        assert_eq!(
            owners(&set(&["formatting"]), &riders),
            None,
            "a red every rider moved was put on all of them rather than searched"
        );
        assert_eq!(
            owners(
                &set(&["every-script-meets-shellcheck-at-its-strictest"]),
                &riders
            ),
            None,
            "a red no rider moved, which is main's, was put on a rider"
        );
    }

    #[test]
    fn a_rider_whose_pass_cant_say_is_suspected() {
        let riders = vec![
            ("a".to_owned(), Some(set(&["formatting"]))),
            ("b".to_owned(), None),
            ("c".to_owned(), Some(set(&["cadence-passes-clippy"]))),
        ];
        assert_eq!(
            owners(&set(&["jobs-sorter-passes-clippy"]), &riders),
            Some(vec!["b".to_owned()]),
            "a rider with no pass to read was cleared"
        );
    }

    #[test]
    fn only_a_red_wholly_on_the_list_runs_again() {
        let listed = listed(&serde_json::json!([
            {"test": "the-margin-cat-follows-the-keys-on-niri", "subtests": ["test_a"]},
        ]));
        assert!(
            flaky(&set(&["the-margin-cat-follows-the-keys-on-niri"]), &listed),
            "a listed check's red was blamed at once"
        );
        assert!(
            !flaky(
                &set(&["the-margin-cat-follows-the-keys-on-niri", "formatting"]),
                &listed
            ),
            "a red beside a listed one ran again"
        );
        assert!(
            !flaky(&BTreeSet::new(), &listed),
            "a red naming nothing ran again"
        );
    }
}
