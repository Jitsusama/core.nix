//! The tests the net holds in quarantine, as origin's main lists them.
//!
//! A test that failed and then passed on the same input goes on the net's
//! quarantine list (`tools/merge-net/quarantine.json`), with its reason and
//! the PR that owns it. Such a test still runs before a merge, but its
//! failure says nothing about the change, so it blocks nothing. The list is
//! read from origin's main rather than the working tree, so a branch can't
//! quarantine its own failure.
//!
//! An entry naming subtests excuses a failure only in them: on 12 November
//! a change broke a new subtest of a test quarantined for another, and
//! every gate let it through. `run-subtests --there` and
//! `nix-offload there --changed` judge an entry the same way.
//!
//! A test may have several entries, one for each time the net listed it,
//! each owned by the PR that broke what it lists: on 13 November #4054
//! filed three subtests #4004 broke under #3737, the first owner, so the
//! owner named was never the one to mend them. A test's entries are read
//! together, a whole one excusing everything, and a failure excused is
//! said with the owner of each subtest that failed.

use std::collections::BTreeSet;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Command, ExitStatus, Stdio};

use serde_json::Value;

/// Where the net keeps its quarantine list in the repository.
pub const LIST: &str = "tools/merge-net/quarantine.json";

/// One entry for a test: why it's there, the PR that owns it, and the
/// subtests it holds, none when it holds the whole test.
#[derive(Debug, PartialEq, Eq)]
pub struct Listing {
    pub reason: String,
    pub owner: Option<u64>,
    pub subtests: BTreeSet<String>,
}

impl Listing {
    fn said(&self) -> String {
        let owner = self
            .owner
            .map_or_else(String::new, |pr| format!("; #{pr} owns it"));
        format!("({}{owner})", self.reason)
    }
}

/// A test on the list, with every entry that holds it.
#[derive(Debug, PartialEq, Eq)]
pub struct Held {
    pub test: String,
    pub listings: Vec<Listing>,
}

impl Held {
    /// Say why a failure in `failed` blocks nothing, naming the entry of
    /// each subtest that failed, or the one holding the whole test.
    pub fn said(&self, failed: &BTreeSet<String>) -> String {
        if let Some(whole) = self
            .listings
            .iter()
            .find(|listing| listing.subtests.is_empty())
        {
            return format!("main quarantines it {}", whole.said());
        }
        let each: Vec<String> = self
            .listings
            .iter()
            .filter_map(|listing| {
                let met: Vec<&str> = listing
                    .subtests
                    .intersection(failed)
                    .map(String::as_str)
                    .collect();
                (!met.is_empty()).then(|| format!("{} {}", met.join(", "), listing.said()))
            })
            .collect();
        format!("main quarantines {}", each.join(" and "))
    }

    /// Whether a failure in `failed`, the subtests its log says failed,
    /// blocks nothing: any failure when an entry holds the whole test, and
    /// otherwise one naming only subtests its entries hold, so a log naming
    /// none, as a build that never ran its tests gives, still blocks.
    pub fn excuses(&self, failed: &BTreeSet<String>) -> bool {
        self.listings
            .iter()
            .any(|listing| listing.subtests.is_empty())
            || (!failed.is_empty() && self.unlisted(failed).is_empty())
    }

    /// The subtests in `failed` no entry holds.
    pub fn unlisted<'failed>(&self, failed: &'failed BTreeSet<String>) -> Vec<&'failed str> {
        failed
            .iter()
            .filter(|subtest| {
                !self
                    .listings
                    .iter()
                    .any(|listing| listing.subtests.contains(*subtest))
            })
            .map(String::as_str)
            .collect()
    }
}

/// The subtest a line of a VM test's log says failed, as pytest says it:
/// `FAILED vm-pytest-x/x-vm-test.py::test_y - why`, a parametrized case
/// by its function's name. It is merge-net's `failed_on` in subtests.rs,
/// and nix-offload's in changed.rs, copied, so the three change together.
pub fn failed_on(line: &str) -> Option<&str> {
    let (_, rest) = line.split_once("FAILED ")?;
    let (_, name) = rest.split_whitespace().next()?.rsplit_once("::")?;
    let name = name.split('[').next().unwrap_or(name);
    (name.starts_with("test_") && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .then_some(name)
}

/// Run `command` to its end, passing on every line it says on stderr,
/// where a build's log comes, and gathering the subtests those lines say
/// failed, for `Held::excuses` to judge.
pub fn heard(command: &mut Command) -> io::Result<(ExitStatus, BTreeSet<String>)> {
    let mut child = command.stderr(Stdio::piped()).spawn()?;
    let mut failed = BTreeSet::new();
    if let Some(said) = child.stderr.take() {
        let mut here = io::stderr();
        for line in BufReader::new(said).split(b'\n').map_while(Result::ok) {
            let line = String::from_utf8_lossy(&line);
            let _ = writeln!(here, "{line}");
            failed.extend(failed_on(&line).map(str::to_owned));
        }
    }
    Ok((child.wait()?, failed))
}

/// The list as origin's main holds it in the repository at `root`. A
/// repository without one quarantines nothing; a list that doesn't read is
/// said so, and quarantines nothing either, so every failure still blocks.
pub fn on_main(root: &str, main: &str) -> Vec<Held> {
    let shown = Command::new("git")
        .args(["-C", root, "show", &format!("{main}:{LIST}")])
        .stderr(Stdio::null())
        .output();
    let Some(shown) = shown.ok().filter(|shown| shown.status.success()) else {
        return Vec::new();
    };
    parse(&String::from_utf8_lossy(&shown.stdout)).unwrap_or_else(|why| {
        eprintln!("changed-checks: {LIST} on main doesn't read, so nothing is quarantined: {why}");
        Vec::new()
    })
}

/// Read a quarantine list: entries with a test, a reason and an owner PR,
/// each test's entries gathered into one, in the order they're listed.
pub fn parse(listed: &str) -> Result<Vec<Held>, String> {
    let list: Value = serde_json::from_str(listed).map_err(|error| error.to_string())?;
    let entries = list.as_array().ok_or("it isn't a list")?;
    let mut held: Vec<Held> = Vec::new();
    for entry in entries {
        let test = entry["test"]
            .as_str()
            .ok_or_else(|| format!("an entry names no test: {entry}"))?;
        let listing = Listing {
            reason: entry["reason"]
                .as_str()
                .unwrap_or("no reason given")
                .to_owned(),
            owner: entry["owner_pr"].as_u64(),
            subtests: entry["subtests"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
        };
        match held.iter_mut().find(|held| held.test == test) {
            Some(earlier) => earlier.listings.push(listing),
            None => held.push(Held {
                test: test.to_owned(),
                listings: vec![listing],
            }),
        }
    }
    Ok(held)
}

/// The entry holding `check`, a test or one of its parts, such as its
/// gallery, `vm-a.gallery`, which fails as its test does.
pub fn find<'list>(list: &'list [Held], check: &str) -> Option<&'list Held> {
    let test = check.split('.').next().unwrap_or(check);
    list.iter().find(|held| held.test == test)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTED: &str = r#"[
      {"test": "vm-a", "reason": "failed then passed at abc", "owner_pr": 12, "commit": "abc"},
      {"test": "vm-b"}
    ]"#;

    fn failed(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn a_list_gives_each_test_its_reason_and_owner() {
        let list = parse(LISTED).expect("it reads");
        assert_eq!(
            list[0],
            Held {
                test: "vm-a".to_owned(),
                listings: vec![Listing {
                    reason: "failed then passed at abc".to_owned(),
                    owner: Some(12),
                    subtests: BTreeSet::new(),
                }],
            }
        );
        assert_eq!(
            list[0].said(&failed(&[])),
            "main quarantines it (failed then passed at abc; #12 owns it)"
        );
        assert_eq!(
            list[1].said(&failed(&[])),
            "main quarantines it (no reason given)"
        );
        assert_eq!(
            parse("[]").expect("it reads"),
            [] as [crate::quarantine::Held; 0]
        );
    }

    #[test]
    fn an_entry_naming_subtests_excuses_only_a_failure_in_them() {
        let list =
            parse(r#"[{"test": "vm-a", "subtests": ["test_p", "test_q"]}, {"test": "vm-b"}]"#)
                .expect("it reads");
        assert!(
            list[0].excuses(&failed(&["test_q"])),
            "a listed subtest blocked"
        );
        assert!(
            !list[0].excuses(&failed(&["test_q", "test_r"])),
            "an unlisted subtest was excused"
        );
        assert!(
            !list[0].excuses(&failed(&[])),
            "a log naming no subtest was excused"
        );
        assert!(
            list[1].excuses(&failed(&[])),
            "a whole entry didn't excuse a failure"
        );
        assert!(
            list[1].excuses(&failed(&["test_r"])),
            "a whole entry didn't excuse a subtest"
        );
    }

    #[test]
    fn a_tests_entries_are_read_together_each_failure_said_with_its_owner() {
        let list = parse(
            r#"[{"test": "vm-a", "reason": "r1", "owner_pr": 1, "subtests": ["test_p"]},
                {"test": "vm-b", "subtests": ["test_p"]},
                {"test": "vm-a", "reason": "r2", "owner_pr": 2, "subtests": ["test_q"]}]"#,
        )
        .expect("it reads");
        assert_eq!(list.len(), 2, "vm-a's two entries weren't read as one test");
        let held = find(&list, "vm-a").expect("vm-a is held");
        assert!(
            held.excuses(&failed(&["test_p", "test_q"])),
            "a failure in subtests two owners hold blocked"
        );
        assert_eq!(
            held.said(&failed(&["test_q"])),
            "main quarantines test_q (r2; #2 owns it)",
            "a failure wasn't said with its own owner alone"
        );
        assert_eq!(
            held.said(&failed(&["test_p", "test_q"])),
            "main quarantines test_p (r1; #1 owns it) and test_q (r2; #2 owns it)"
        );
        assert_eq!(held.unlisted(&failed(&["test_q", "test_r"])), ["test_r"]);
        let whole =
            parse(r#"[{"test": "vm-a", "subtests": ["test_p"]}, {"test": "vm-a", "reason": "r"}]"#)
                .expect("it reads");
        assert!(
            whole[0].excuses(&failed(&[])),
            "an entry holding the whole test didn't excuse every failure"
        );
    }

    #[test]
    fn a_failed_subtest_is_read_from_its_line_of_the_log() {
        let line = "vm-test-run-x> FAILED vm-pytest-x/x-vm-test.py::test_y[2] - AssertionError";
        assert_eq!(
            failed_on(line),
            Some("test_y"),
            "a parametrized case wasn't named"
        );
        assert_eq!(
            failed_on("vm-test-run-x> 1 passed"),
            None,
            "a passing line named one"
        );
        assert_eq!(
            failed_on("FAILED x.py::helper - e"),
            None,
            "a non-test was named"
        );
    }

    #[test]
    fn a_build_is_heard_line_by_line_for_the_subtests_it_failed() {
        let (ended, failed) = heard(Command::new("sh").args([
            "-c",
            "echo 'FAILED a.py::test_p - e' >&2; echo 'fine' >&2; exit 3",
        ]))
        .expect("it ran");
        assert_eq!(ended.code(), Some(3), "its status wasn't kept");
        assert_eq!(
            failed,
            BTreeSet::from(["test_p".to_owned()]),
            "its failed subtest wasn't gathered"
        );
    }

    #[test]
    fn a_list_that_isnt_one_is_refused() {
        assert!(parse("{}").is_err());
        assert!(parse(r#"[{"reason": "r"}]"#).is_err());
        assert!(parse("not json").is_err());
    }

    #[test]
    fn a_test_is_held_by_its_name_and_its_parts_with_it() {
        let list = parse(LISTED).expect("it reads");
        assert_eq!(
            find(&list, "vm-a").map(|held| held.listings[0].owner),
            Some(Some(12))
        );
        assert!(find(&list, "vm-a.gallery").is_some());
        assert!(find(&list, "vm-ab").is_none());
        assert!(find(&list, "vm-c").is_none());
    }
}
