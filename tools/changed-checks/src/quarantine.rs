//! The tests the net holds in quarantine, as origin's main lists them.
//!
//! A test that failed and then passed on the same input goes on the net's
//! quarantine list (`tools/merge-net/quarantine.json`), with its reason and
//! the PR that owns it. Such a test still runs before a merge, but its
//! failure says nothing about the change, so it blocks nothing. The list is
//! read from origin's main rather than the working tree, so a branch can't
//! quarantine its own failure.

use std::process::{Command, Stdio};

use serde_json::Value;

/// Where the net keeps its quarantine list in the repository.
pub const LIST: &str = "tools/merge-net/quarantine.json";

/// A test on the list: its name, why it's there, and the PR that owns it.
#[derive(Debug, PartialEq, Eq)]
pub struct Held {
    pub test: String,
    pub reason: String,
    pub owner: Option<u64>,
}

impl Held {
    /// Say why its failure blocks nothing.
    pub fn said(&self) -> String {
        let owner = self
            .owner
            .map_or_else(String::new, |pr| format!("; #{pr} owns it"));
        format!("main quarantines it ({}{owner})", self.reason)
    }
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

/// Read a quarantine list: entries with a test, a reason and an owner PR.
pub fn parse(listed: &str) -> Result<Vec<Held>, String> {
    let list: Value = serde_json::from_str(listed).map_err(|error| error.to_string())?;
    let entries = list.as_array().ok_or("it isn't a list")?;
    entries
        .iter()
        .map(|entry| {
            let test = entry["test"]
                .as_str()
                .ok_or_else(|| format!("an entry names no test: {entry}"))?;
            Ok(Held {
                test: test.to_owned(),
                reason: entry["reason"]
                    .as_str()
                    .unwrap_or("no reason given")
                    .to_owned(),
                owner: entry["owner_pr"].as_u64(),
            })
        })
        .collect()
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

    #[test]
    fn a_list_gives_each_test_its_reason_and_owner() {
        let list = parse(LISTED).expect("it reads");
        assert_eq!(
            list[0],
            Held {
                test: "vm-a".to_owned(),
                reason: "failed then passed at abc".to_owned(),
                owner: Some(12),
            }
        );
        assert_eq!(
            list[0].said(),
            "main quarantines it (failed then passed at abc; #12 owns it)"
        );
        assert_eq!(list[1].said(), "main quarantines it (no reason given)");
        assert_eq!(
            parse("[]").expect("it reads"),
            [] as [crate::quarantine::Held; 0]
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
        assert_eq!(find(&list, "vm-a").map(|held| held.owner), Some(Some(12)));
        assert!(find(&list, "vm-a.gallery").is_some());
        assert!(find(&list, "vm-ab").is_none());
        assert!(find(&list, "vm-c").is_none());
    }
}
