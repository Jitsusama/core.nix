//! Names the test that would catch a miss again, on the miss's line of
//! Joel's misses ledger, as `test`: `REPO PATH NAME`, the repository as
//! cadence calls it, the path the test is in, and words of the test that
//! find it there, such as its name. Cadence's hour asks each miss's owner
//! for one, and keeps the miss on its card until it finds NAME in PATH at
//! the revision of REPO that runs here, which that repository's own checks
//! ran before it merged: every miss gets a check that runs, as the
//! orchestrator ruled on 23 November. pain-points stays the ledger's only
//! writer, so the line is changed under the ledger's lock and the ledger
//! written whole.

use super::held;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use terminal_kit::{Exit, Failure, Help};

/// The flags `test` takes, each with a value.
pub const FLAGS: [&str; 3] = ["--miss", "--test", "--ledgers"];

/// What `pain-points test --help` says.
#[must_use]
pub fn help() -> Help {
    Help::new("test", "names the test that would catch a miss again")
        .example(
            "pain-points test --miss AT --test \"core modules/x/src/y.rs a_test\"",
            "as a miss's owner runs it",
        )
        .option("--miss AT", "the miss's time, as its line has it")
        .option("--test TEST", "its test, as REPO PATH NAME")
        .option("--ledgers DIR", "its ledger's folder, or the mission's")
        .exit(Exit::Usage, "no miss or test, or a test not REPO PATH NAME")
        .exit(Exit::NotFound, "no one miss at that time")
        .exit(Exit::TryAgain, "the ledger stayed locked")
        .exit(Exit::Failed, "the ledger couldn't be read or written")
}

/// Names the test `--test` gives on the line of the miss at `--miss`, in
/// the ledger in `--ledgers`, or the mission's, and says so.
///
/// # Errors
///
/// As `help` says of each exit.
pub fn named(given: &BTreeMap<&str, String>) -> Result<String, Failure> {
    let wrongly = |what: &str| Failure::new(Exit::Usage, what).try_this("pain-points test --help");
    let at = given
        .get("--miss")
        .ok_or_else(|| wrongly("--miss names the miss"))?;
    let test = given
        .get("--test")
        .ok_or_else(|| wrongly("--test names its test"))?;
    let test = test.trim();
    if !shaped(test) {
        return Err(wrongly(&format!(
            "\"{test}\" isn't REPO PATH NAME, such as \"core modules/x/src/y.rs a_test\""
        )));
    }
    let folder = given
        .get("--ledgers")
        .map_or_else(|| crate::state_home().join("mission"), PathBuf::from);
    name_in(&folder.join("misses.jsonl"), at, test)
}

/// Whether `test` reads as cadence reads one: a repository, a path git
/// won't read as an option, and words to find in it.
fn shaped(test: &str) -> bool {
    let mut parts = test.splitn(3, ' ');
    let (repo, path, name) = (parts.next(), parts.next(), parts.next());
    repo.is_some_and(|repo| !repo.is_empty())
        && path.is_some_and(|path| !path.is_empty() && !path.starts_with('-'))
        && name.is_some_and(|name| !name.trim().is_empty())
}

/// Names `test` on the line of the one miss at `at` in `ledger`.
fn name_in(ledger: &Path, at: &str, test: &str) -> Result<String, Failure> {
    let (_lock, name) = held(ledger).map_err(|why| {
        Failure::new(Exit::TryAgain, "the misses ledger couldn't be held").because(why)
    })?;
    let failed = |why: String| Failure::new(Exit::Failed, "the misses ledger").because(why);
    let text = fs::read_to_string(ledger).map_err(|e| failed(format!("{name}: {e}")))?;
    let mut found = 0;
    let mut already = false;
    let mut lines: Vec<String> = text
        .lines()
        .map(|line| {
            let Ok(mut read) = serde_json::from_str::<Value>(line) else {
                return line.to_owned();
            };
            if read["at"].as_str() != Some(at) {
                return line.to_owned();
            }
            found += 1;
            already = read["test"].as_str() == Some(test);
            read["test"] = json!(test);
            read.to_string()
        })
        .collect();
    if found != 1 {
        let what = format!("{found} misses in {name} are at {at}, not one");
        return Err(Failure::new(Exit::NotFound, what).try_this("its time as its line has it"));
    }
    if already {
        return Ok(format!("pain-points: the miss at {at} already names {test}"));
    }
    lines.push(String::new());
    crate::write_whole(ledger, &lines.join("\n")).map_err(failed)?;
    Ok(format!("pain-points: the miss at {at} now names its test, {test}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT: &str = "2026-11-14T17:24:00-05:00";

    /// A ledger of the test's own, holding `text`, emptied first.
    fn ledger(name: &str, text: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tested-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a folder");
        let path = dir.join("misses.jsonl");
        fs::write(&path, text).expect("a ledger");
        path
    }

    #[test]
    fn a_test_is_named_on_its_miss_alone_and_every_other_line_kept() {
        let other = r#"{"at":"2026-11-15T00:28:00-05:00","owner":"5FWIUS"}"#;
        let text = format!("{other}\n{{\"at\":\"{AT}\",\"owner\":\"XY9H5C\"}}\nnot json\n");
        let path = ledger("named", &text);
        let test = "core modules/cat/src/air.rs the_cat_never_waits";
        let said = name_in(&path, AT, test).expect("named");
        assert!(said.contains(test), "it didn't say what it named: {said}");
        let kept = fs::read_to_string(&path).expect("the ledger");
        let lines: Vec<&str> = kept.lines().collect();
        assert_eq!(lines[0], other, "another miss's line changed");
        assert_eq!(lines[2], "not json", "a line it couldn't read was dropped");
        let named: Value = serde_json::from_str(lines[1]).expect("the miss's line");
        assert_eq!(named["test"], test, "the miss doesn't name its test");
        assert_eq!(named["owner"], "XY9H5C", "the miss lost its owner");
        assert!(kept.ends_with('\n'), "the ledger lost its last line's end");
        let again = name_in(&path, AT, test).expect("named again");
        assert!(
            again.contains("already"),
            "naming it twice said nothing of it: {again}"
        );
    }

    #[test]
    fn a_time_with_no_miss_or_two_names_none() {
        let one = format!("{{\"at\":\"{AT}\"}}\n");
        for (text, what) in [("", "no miss"), (&*one.repeat(2), "two misses")] {
            let path = ledger("unnamed", text);
            let failed = name_in(&path, AT, "core a b").expect_err("named a test");
            assert!(
                matches!(failed.exit, Exit::NotFound),
                "{what} at the time wasn't refused as not found"
            );
            assert_eq!(
                fs::read_to_string(&path).expect("the ledger"),
                text,
                "{what}: the ledger was changed"
            );
        }
    }

    #[test]
    fn a_test_is_repository_path_and_name() {
        assert!(
            shaped("core modules/x/src/y.rs a_test"),
            "a test was refused"
        );
        assert!(
            shaped("world areas/x/test.rb a test of many words"),
            "words were refused"
        );
        for test in [
            "",
            "core",
            "core modules/x.rs",
            "core --output=x a",
            "core  a",
        ] {
            assert!(!shaped(test), "\"{test}\" was taken as a test");
        }
    }
}
