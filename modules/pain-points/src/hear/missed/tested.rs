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
//! owner's word, never by going quiet. A miss nothing heard, such as one a
//! cell owns up to that Joel said nothing of, is written by its owner
//! (owned.rs), which names its fix too, as nothing lands against it. A
//! miss whose fix another cell holds moves to that cell, as the hour asks
//! its owner for the test, and its line keeps whose it was before.
//! pain-points stays the ledger's only writer, so the line is changed
//! under the ledger's lock and the ledger written whole.

mod owned;

use super::held;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use terminal_kit::{Exit, Failure, Help};

/// The flags `test` takes, each with a value.
pub const FLAGS: [&str; 7] = [
    "--miss",
    "--test",
    "--none",
    "--owner",
    "--what",
    "--fix",
    "--ledgers",
];

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
        .example(
            "pain-points test --miss AT --owner CELL --what \"a search walked his folders\"",
            "as a cell writes a miss of its own that nothing heard",
        )
        .example(
            "pain-points test --miss AT --owner CELL",
            "as a miss moves to the cell holding its fix",
        )
        .option("--miss AT", "the miss's time, as its line has it")
        .option("--test TEST", "its test, as REPO PATH NAME")
        .option("--none WHY", "why it was no miss, in place of a test")
        .option(
            "--owner CELL",
            "whose miss it is: with --what, to write one; alone, to move one",
        )
        .example(
            "pain-points test --miss AT --fix \"core #6470\"",
            "as a cell names the change that fixed a miss it wrote",
        )
        .option("--what WORDS", "what went wrong for him, with --owner")
        .option("--fix CHANGE", "the change fixing a miss its owner wrote")
        .option("--ledgers DIR", "its ledger's folder, or the mission's")
        .exit(
            Exit::Usage,
            "no miss, a test and why it's none both or neither, a test not REPO PATH NAME, \
             a fix not REPO #N or on a heard miss, or a miss its owner writes half said, \
             said none or at a time taken",
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

/// All an owner says of a miss at once: a test or why it was none, the
/// change that fixed one it wrote, and the cell it moves to.
#[derive(Debug, Clone, Copy)]
struct Saying<'a> {
    said: Option<Said<'a>>,
    fix: Option<&'a str>,
    owner: Option<&'a str>,
}

impl Saying<'_> {
    /// Puts this on a miss's `line`, and says whether it said it all
    /// already.
    fn on(self, line: &mut Value) -> bool {
        let said = self.said.is_none_or(|said| said.on(line));
        let fixed = self.fix.is_none_or(|fix| {
            let same = line["check"].as_str() == Some(fix);
            line["check"] = json!(fix);
            same
        });
        let moved = self.owner.is_none_or(|owner| moved_to(line, owner));
        said && fixed && moved
    }

    /// How the miss reads once this is said, or nothing for nothing said.
    fn says(self) -> String {
        let fixed = self.fix.map(|fix| format!("names its fix, {fix}"));
        let said = self.said.map(Said::says);
        let moved = self.owner.map(|owner| format!("is {owner}'s"));
        said.into_iter()
            .chain(fixed)
            .chain(moved)
            .collect::<Vec<_>>()
            .join(" and ")
    }
}

/// Moves the miss on `line` to `owner`, adding whose it was to its
/// `owned_before`, and says whether it was `owner`'s already.
fn moved_to(line: &mut Value, owner: &str) -> bool {
    let was = line["owner"].as_str().map(str::to_owned);
    if was.as_deref() == Some(owner) {
        return true;
    }
    if let Some(was) = was {
        match line.get_mut("owned_before").and_then(Value::as_array_mut) {
            Some(before) => before.push(json!(was)),
            None => line["owned_before"] = json!([was]),
        }
    }
    line["owner"] = json!(owner);
    false
}

impl<'a> From<Said<'a>> for Saying<'a> {
    fn from(said: Said<'a>) -> Self {
        Saying {
            said: Some(said),
            fix: None,
            owner: None,
        }
    }
}

/// The ledger as a change leaves it, when it changes, and what to say.
type Changed = Result<(Option<String>, String), Failure>;

/// Names the test `--test` gives, or why `--none` says it was no miss, and
/// the fix `--fix` gives, on the line of the miss at `--miss`, in the
/// ledger in `--ledgers`, or the mission's, and says so. With `--owner`
/// and `--what`, it writes the miss at `--miss` first, which nothing
/// heard, with its test and fix if they're given; with `--owner` alone,
/// it moves the miss there to that cell.
///
/// # Errors
///
/// As `help` says of each exit.
pub fn named(given: &BTreeMap<&str, String>) -> Result<String, Failure> {
    let wrongly = |what: &str| Failure::new(Exit::Usage, what).try_this("pain-points test --help");
    let at = given
        .get("--miss")
        .ok_or_else(|| wrongly("--miss names the miss"))?;
    let written = owned::given(given, at).map_err(|what| wrongly(&what))?;
    let fix = owned::fix(given).map_err(|what| wrongly(&what))?;
    let owner = owned::moved(given).map_err(|what| wrongly(&what))?;
    let neither = "give --test, --none with why it was no miss, --fix, or --owner to move it";
    let said = match (given.get("--test"), given.get("--none")) {
        (Some(test), None) => Some(Said::Test(test.trim())),
        (None, Some(why)) => Some(Said::NoMiss {
            why: why.trim(),
            at_ms: crate::millis_now(),
        }),
        (None, None) => None,
        (Some(_), Some(_)) => return Err(wrongly(neither)),
    };
    match said {
        Some(Said::Test(test)) if !shaped(test) => {
            return Err(wrongly(&format!(
                "\"{test}\" isn't REPO PATH NAME, such as \"core modules/x/src/y.rs a_test\""
            )));
        }
        Some(Said::NoMiss { why, .. }) if why.is_empty() || why.contains(char::is_control) => {
            return Err(wrongly("--none says why on one line"));
        }
        _ => {}
    }
    let folder = given
        .get("--ledgers")
        .map_or_else(|| crate::state_home().join("mission"), PathBuf::from);
    let ledger = folder.join("misses.jsonl");
    let saying = Saying { said, fix, owner };
    match (written, said) {
        (Some(_), Some(Said::NoMiss { .. })) => Err(wrongly(
            "a miss its owner writes is one; say it was none once it's written",
        )),
        (Some(miss), _) => owned::own_in(&ledger, at, miss, saying),
        (None, None) if fix.is_none() && owner.is_none() => Err(wrongly(neither)),
        (None, _) => name_in(&ledger, at, saying),
    }
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

/// Puts what's `said` on the line of the one miss at `at` in `ledger`. A
/// fix goes only on a miss its owner wrote: a heard one closes on the
/// landing pain-points finds, and the keeper's lines name their own.
fn name_in<'a>(ledger: &Path, at: &str, said: impl Into<Saying<'a>>) -> Result<String, Failure> {
    let said = said.into();
    changed_in(ledger, false, |before, name| {
        let mut found = 0;
        let mut heard = false;
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
                heard = read["heard_by"] != owned::OWNER;
                already = said.on(&mut read);
                read.to_string()
            })
            .collect();
        if found != 1 {
            let what = format!("{found} misses in {name} are at {at}, not one");
            let failed = Failure::new(Exit::NotFound, what);
            return Err(failed.try_this("its time as its line has it"));
        }
        if heard && said.fix.is_some() {
            let what = format!("the miss at {at} wasn't written by its owner, so it takes no fix");
            let failed = Failure::new(Exit::Usage, what);
            return Err(failed.try_this("its test, as a heard miss closes on its landing"));
        }
        if already {
            let said = format!("pain-points: the miss at {at} already {}", said.says());
            return Ok((None, said));
        }
        lines.push(String::new());
        let said = format!("pain-points: the miss at {at} now {}", said.says());
        Ok((Some(lines.join("\n")), said))
    })
}

/// Changes `ledger` as `change` says, given what it holds and its name,
/// under its lock, and says what `change` says. It is written whole only
/// when it changes, once it reads again as it did before, as `write` does,
/// so a line a hand added without the lock isn't lost. A ledger not yet
/// there is started when `start` says to, in a folder that is.
fn changed_in(
    ledger: &Path,
    start: bool,
    change: impl FnOnce(&str, &str) -> Changed,
) -> Result<String, Failure> {
    let room = start && ledger.parent().is_some_and(Path::is_dir);
    if !ledger.is_file() && !room {
        let what = format!("no misses ledger at {}", ledger.display());
        return Err(Failure::new(Exit::NotFound, what).try_this("the mission's folder"));
    }
    let (_lock, name) = held(ledger).map_err(|why| {
        Failure::new(Exit::TryAgain, "the misses ledger couldn't be held").because(why)
    })?;
    let failed = |why: String| Failure::new(Exit::Failed, "the misses ledger").because(why);
    let read = || match fs::read_to_string(ledger) {
        Err(e) if room && e.kind() == ErrorKind::NotFound => Ok(String::new()),
        read => read.map_err(|e| failed(format!("{name}: {e}"))),
    };
    let before = read()?;
    let (after, said) = change(&before, &name)?;
    let Some(after) = after else {
        return Ok(said);
    };
    if read()? != before {
        let what = format!("{name} changed as its miss was written");
        return Err(Failure::new(Exit::TryAgain, what).try_this("pain-points test again"));
    }
    crate::write_whole(ledger, &after).map_err(failed)?;
    Ok(said)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) const AT: &str = "2026-11-14T17:24:00-05:00";

    /// A ledger of the test's own, holding `text`, emptied first.
    pub(super) fn ledger(name: &str, text: &str) -> PathBuf {
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

    /// The flags of `pain-points test` for the miss at `AT` in the ledger
    /// at `path`, with `given`.
    fn flags(path: &Path, given: &[(&'static str, &str)]) -> BTreeMap<&'static str, String> {
        let folder = path.parent().expect("the ledger's folder");
        let mut flags: BTreeMap<&str, String> = given
            .iter()
            .map(|&(flag, value)| (flag, value.to_owned()))
            .collect();
        flags.insert("--miss", AT.to_owned());
        flags.insert("--ledgers", folder.display().to_string());
        flags
    }

    #[test]
    fn a_heard_miss_moves_to_the_cell_holding_its_fix_and_keeps_whose_it_was() {
        let heard = json!({"at": AT, "words": "stop", "owner": "WCJQSG", "heard_by": "x"});
        let path = ledger("moved", &format!("{heard}\n"));
        let read = |path: &Path| -> Value {
            let kept = fs::read_to_string(path).expect("the ledger");
            serde_json::from_str(kept.trim()).expect("the miss's line")
        };
        let said = named(&flags(&path, &[("--owner", "KP0L6A")])).expect("moved");
        assert!(
            said.contains("now is KP0L6A's"),
            "it didn't say where the miss moved: {said}"
        );
        let line = read(&path);
        assert_eq!(
            (&line["owner"], &line["owned_before"], &line["words"]),
            (&json!("KP0L6A"), &json!(["WCJQSG"]), &json!("stop")),
            "the miss didn't move, keeping whose it was and his words: {line}"
        );
        let again = named(&flags(&path, &[("--owner", "KP0L6A")])).expect("moved again");
        assert!(
            again.contains("already"),
            "moving it twice said nothing of it: {again}"
        );
        let test = "core modules/x/src/y.rs a_test";
        named(&flags(&path, &[("--owner", "85PKB2"), ("--test", test)])).expect("tested");
        let line = read(&path);
        assert_eq!(
            (&line["owner"], &line["owned_before"], &line["test"]),
            (&json!("85PKB2"), &json!(["WCJQSG", "KP0L6A"]), &json!(test)),
            "a move with its test didn't take both, keeping each owner before: {line}"
        );
    }

    #[test]
    fn a_miss_with_no_owner_moves_with_none_before() {
        let path = ledger("moved-unowned", &format!("{{\"at\":\"{AT}\"}}\n"));
        named(&flags(&path, &[("--owner", "KP0L6A")])).expect("moved");
        let kept = fs::read_to_string(&path).expect("the ledger");
        let line: Value = serde_json::from_str(kept.trim()).expect("the miss's line");
        assert_eq!(
            (&line["owner"], &line["owned_before"]),
            (&json!("KP0L6A"), &Value::Null),
            "a miss owned by no one gained an owner before: {line}"
        );
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
