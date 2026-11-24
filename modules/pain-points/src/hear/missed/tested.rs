//! Names the test that would catch a miss again, on the miss's line of
//! Joel's misses ledger, as `test`: `REPO PATH NAME`, the repository as
//! cadence calls it, the path the test is in, and words of the test that
//! find it there, such as its name. Cadence's hour asks each miss's owner
//! for one, and keeps the miss on its card until it finds NAME in PATH at
//! the revision of REPO that runs here, which that repository's own checks
//! ran before it merged: every miss gets a check that runs, as the
//! orchestrator ruled on 23 November. A pain heard as a miss that wasn't
//! one, such as his reassurance, can't name a test, so its owner says why
//! as `none` instead, which the hour shows for a day: as a finding closes
//! on its owner's word of why it's expected, a miss closes only on its
//! owner's word, never by going quiet. pain-points stays the ledger's only
//! writer, so the line is changed under the ledger's lock and the ledger
//! written whole.

use super::held;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use terminal_kit::{Exit, Failure, Help};

/// The flags `test` takes, each with a value.
pub const FLAGS: [&str; 4] = ["--miss", "--test", "--none", "--ledgers"];

/// What `pain-points test --help` says.
#[must_use]
pub fn help() -> Help {
    Help::new("test", "names the test that would catch a miss again")
        .example(
            "pain-points test --miss AT --test \"core modules/x/src/y.rs a_test\"",
            "as a miss's owner runs it",
        )
        .example(
            "pain-points test --miss AT --none \"he was reassuring, not hurt\"",
            "as its owner says a pain heard as a miss was none",
        )
        .option("--miss AT", "the miss's time, as its line has it")
        .option("--test TEST", "its test, as REPO PATH NAME")
        .option("--none WHY", "why it was no miss, in place of a test")
        .option("--ledgers DIR", "its ledger's folder, or the mission's")
        .exit(
            Exit::Usage,
            "no miss, a test and why it's none both or neither, a test not REPO PATH NAME",
        )
        .exit(Exit::NotFound, "no ledger, or no one miss at that time")
        .exit(Exit::TryAgain, "the ledger stayed locked")
        .exit(Exit::Failed, "the ledger couldn't be read or written")
}

/// What an owner says of a miss: the test that would catch it again, or
/// why it was no miss, and when that was said.
#[derive(Debug, Clone, Copy)]
enum Said<'a> {
    Test(&'a str),
    NoMiss { why: &'a str, at_ms: i64 },
}

impl Said<'_> {
    /// Puts this on a miss's `line`, in place of what was said before, and
    /// says whether the line said it already, with nothing else said
    /// beside it. A reason said again keeps when it was first said.
    fn on(self, line: &mut Value) -> bool {
        let (field, said, rivals) = match self {
            Said::Test(test) => ("test", test, &["none", "none_at_ms"][..]),
            Said::NoMiss { why, .. } => ("none", why, &["test"][..]),
        };
        let same = line[field].as_str() == Some(said);
        let already = same && rivals.iter().all(|rival| line.get(rival).is_none());
        if !already {
            if let Some(fields) = line.as_object_mut() {
                for rival in rivals {
                    fields.remove(*rival);
                }
            }
            if !same {
                line[field] = json!(said);
                if let Said::NoMiss { at_ms, .. } = self {
                    line["none_at_ms"] = json!(at_ms);
                }
            }
        }
        already
    }

    /// How the miss reads once this is said.
    fn says(self) -> String {
        match self {
            Said::Test(test) => format!("names its test, {test}"),
            Said::NoMiss { why, .. } => format!("says it was no miss: {why}"),
        }
    }
}

/// Names the test `--test` gives, or why `--none` says it was no miss, on
/// the line of the miss at `--miss`, in the ledger in `--ledgers`, or the
/// mission's, and says so.
///
/// # Errors
///
/// As `help` says of each exit.
pub fn named(given: &BTreeMap<&str, String>) -> Result<String, Failure> {
    let wrongly = |what: &str| Failure::new(Exit::Usage, what).try_this("pain-points test --help");
    let at = given
        .get("--miss")
        .ok_or_else(|| wrongly("--miss names the miss"))?;
    let said = match (given.get("--test"), given.get("--none")) {
        (Some(test), None) => Said::Test(test.trim()),
        (None, Some(why)) => Said::NoMiss {
            why: why.trim(),
            at_ms: crate::millis_now(),
        },
        _ => return Err(wrongly("give --test, or --none with why it was no miss")),
    };
    match said {
        Said::Test(test) if !shaped(test) => {
            return Err(wrongly(&format!(
                "\"{test}\" isn't REPO PATH NAME, such as \"core modules/x/src/y.rs a_test\""
            )));
        }
        Said::NoMiss { why, .. } if why.is_empty() || why.contains(char::is_control) => {
            return Err(wrongly("--none says why on one line"));
        }
        _ => {}
    }
    let folder = given
        .get("--ledgers")
        .map_or_else(|| crate::state_home().join("mission"), PathBuf::from);
    name_in(&folder.join("misses.jsonl"), at, said)
}

/// Whether `test` reads as cadence reads one: a repository, a path git
/// won't read as an option, and words to find in it, none breaking its
/// line on the card.
fn shaped(test: &str) -> bool {
    let mut parts = test.splitn(3, ' ');
    let (repo, path, name) = (parts.next(), parts.next(), parts.next());
    !test.contains(char::is_control)
        && repo.is_some_and(|repo| repo.contains(|c: char| c.is_ascii_alphanumeric()))
        && path.is_some_and(|path| !path.is_empty() && !path.starts_with('-'))
        && name.is_some_and(|name| !name.trim().is_empty())
}

/// Puts what's `said` on the line of the one miss at `at` in `ledger`.
fn name_in(ledger: &Path, at: &str, said: Said<'_>) -> Result<String, Failure> {
    if !ledger.is_file() {
        let what = format!("no misses ledger at {}", ledger.display());
        return Err(Failure::new(Exit::NotFound, what).try_this("the mission's folder"));
    }
    let (_lock, name) = held(ledger).map_err(|why| {
        Failure::new(Exit::TryAgain, "the misses ledger couldn't be held").because(why)
    })?;
    let failed = |why: String| Failure::new(Exit::Failed, "the misses ledger").because(why);
    let read = || fs::read_to_string(ledger).map_err(|e| failed(format!("{name}: {e}")));
    let before = read()?;
    let mut found = 0;
    let mut already = false;
    let mut lines: Vec<String> = before
        .lines()
        .map(|line| {
            let Ok(mut read) = serde_json::from_str::<Value>(line) else {
                return line.to_owned();
            };
            if read["at"].as_str() != Some(at) {
                return line.to_owned();
            }
            found += 1;
            already = said.on(&mut read);
            read.to_string()
        })
        .collect();
    if found != 1 {
        let what = format!("{found} misses in {name} are at {at}, not one");
        return Err(Failure::new(Exit::NotFound, what).try_this("its time as its line has it"));
    }
    if already {
        return Ok(format!(
            "pain-points: the miss at {at} already {}",
            said.says()
        ));
    }
    lines.push(String::new());
    // Read again before it's written whole, as `write` does, so a line a
    // hand added without the lock isn't lost.
    if read()? != before {
        let what = format!("{name} changed as its miss was named");
        return Err(Failure::new(Exit::TryAgain, what).try_this("pain-points test again"));
    }
    crate::write_whole(ledger, &lines.join("\n")).map_err(failed)?;
    Ok(format!("pain-points: the miss at {at} now {}", said.says()))
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
        let written = format!("{other}\n{{\"at\":\"{AT}\",\"owner\":\"XY9H5C\"}}\nnot json\n");
        let path = ledger("named", &written);
        let test = "core modules/cat/src/air.rs the_cat_never_waits";
        let said = name_in(&path, AT, Said::Test(test)).expect("named");
        assert!(said.contains(test), "it didn't say what it named: {said}");
        let kept = fs::read_to_string(&path).expect("the ledger");
        let lines: Vec<&str> = kept.lines().collect();
        assert_eq!(lines[0], other, "another miss's line changed");
        assert_eq!(lines[2], "not json", "a line it couldn't read was dropped");
        let named: Value = serde_json::from_str(lines[1]).expect("the miss's line");
        assert_eq!(named["test"], test, "the miss doesn't name its test");
        assert_eq!(named["owner"], "XY9H5C", "the miss lost its owner");
        assert!(kept.ends_with('\n'), "the ledger lost its last line's end");
        let again = name_in(&path, AT, Said::Test(test)).expect("named again");
        assert!(
            again.contains("already"),
            "naming it twice said nothing of it: {again}"
        );
    }

    #[test]
    fn an_owner_saying_it_was_no_miss_takes_the_place_of_a_test_until_one_is_named() {
        let path = ledger(
            "none",
            &format!("{{\"at\":\"{AT}\",\"test\":\"core a b\"}}\n"),
        );
        let why = "he was reassuring, not hurt";
        let none = Said::NoMiss { why, at_ms: 1 };
        let said = name_in(&path, AT, none).expect("said");
        assert!(said.contains(why), "it didn't say why: {said}");
        let read = |path: &Path| -> Value {
            let kept = fs::read_to_string(path).expect("the ledger");
            serde_json::from_str(kept.trim()).expect("the miss's line")
        };
        let line = read(&path);
        assert_eq!(
            (&line["none"], &line["none_at_ms"], &line["test"]),
            (&json!(why), &json!(1), &Value::Null),
            "its owner's word didn't take the test's place: {line}"
        );
        let again = Said::NoMiss { why, at_ms: 2 };
        let said = name_in(&path, AT, again).expect("said again");
        assert!(
            said.contains("already"),
            "saying it twice said nothing of it: {said}"
        );
        assert_eq!(
            read(&path)["none_at_ms"],
            json!(1),
            "saying it again moved when it was said"
        );
        name_in(&path, AT, Said::Test("core a b")).expect("named");
        let line = read(&path);
        assert_eq!(
            (&line["test"], &line["none"], &line["none_at_ms"]),
            (&json!("core a b"), &Value::Null, &Value::Null),
            "a test named after didn't take its place: {line}"
        );
    }

    #[test]
    fn naming_the_test_a_line_holds_beside_a_reason_takes_the_reason_off() {
        let path = ledger(
            "both",
            &format!(r#"{{"at":"{AT}","test":"core a b","none":"x","none_at_ms":1}}"#),
        );
        let said = name_in(&path, AT, Said::Test("core a b")).expect("named");
        assert!(
            !said.contains("already"),
            "a line saying both was taken as said: {said}"
        );
        let kept = fs::read_to_string(&path).expect("the ledger");
        let line: Value = serde_json::from_str(kept.trim()).expect("the miss's line");
        assert_eq!(
            (&line["test"], &line["none"], &line["none_at_ms"]),
            (&json!("core a b"), &Value::Null, &Value::Null),
            "the reason beside the test stayed: {line}"
        );
    }

    #[test]
    fn a_test_and_a_reason_both_or_neither_or_a_reason_not_on_one_line_is_refused() {
        let nowhere = std::env::temp_dir().join(format!("tested-refused-{}", std::process::id()));
        for (given, what) in [
            (&[("--test", "core a"), ("--none", "no pain")][..], "both"),
            (&[][..], "neither"),
            (&[("--none", "  ")][..], "an empty reason"),
            (&[("--none", "a\nb")][..], "a reason of two lines"),
        ] {
            let mut flags: BTreeMap<&str, String> = given
                .iter()
                .map(|&(flag, value)| (flag, value.to_owned()))
                .collect();
            flags.insert("--miss", AT.to_owned());
            flags.insert("--ledgers", nowhere.display().to_string());
            let failed = named(&flags).expect_err("it was taken");
            assert!(
                matches!(failed.exit, Exit::Usage),
                "{what} wasn't refused as usage"
            );
        }
    }

    #[test]
    fn a_time_with_no_miss_or_two_names_none() {
        let one = format!("{{\"at\":\"{AT}\"}}\n");
        for (text, what) in [("", "no miss"), (&*one.repeat(2), "two misses")] {
            let path = ledger("unnamed", text);
            let failed = name_in(&path, AT, Said::Test("core a b")).expect_err("named a test");
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
            "core a.rs a\nb",
            "\"\" a.rs a",
        ] {
            assert!(!shaped(test), "\"{test}\" was taken as a test");
        }
    }
}
