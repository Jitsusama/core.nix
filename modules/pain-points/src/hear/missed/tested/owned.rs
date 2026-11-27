//! A miss nothing heard, written by its owner: one a cell owns up to that
//! Joel said nothing of, so pain-points heard nothing to write. Its line
//! holds what went wrong in its owner's words, at the time it happened,
//! and keeps no `words`, which the card quotes as his; it is marked heard
//! by its owner, so nothing reads it as carried or closed by a landing.
//! As no landing names it, its owner names the change that fixed it, as
//! its `check`, which the hour reads as it does any miss's: closed once
//! that change has merged and runs here. Said again with a test or a fix
//! it lacked, the same miss takes them; a time another miss holds is
//! refused, as an owner names its miss by its time. pain-points can't
//! tell which cell runs it, so the card names the owner given, and one
//! given a miss wrongly says it was none, as of any miss, or moves it to
//! the cell whose fix it waits on with `--owner` alone.

use super::{Saying, changed_in};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;
use terminal_kit::{Exit, Failure};

/// What heard a miss its owner wrote, as its line's `heard_by` says.
pub(super) const OWNER: &str = "its owner";

/// A miss nothing heard, as its owner writes it: whose it is, and what
/// went wrong for him in the owner's words.
#[derive(Debug, Clone, Copy)]
pub(super) struct Owned<'a> {
    owner: &'a str,
    what: &'a str,
}

impl<'a> Owned<'a> {
    /// The cell the miss is written for.
    pub(super) fn owner(self) -> &'a str {
        self.owner
    }
}

/// The miss `--owner` and `--what` write at `at`, if they're given, or why
/// they can't: each said plainly on one line, and at a time as the ledger
/// writes one. `--what` goes with `--owner`; `--owner` alone moves a miss
/// rather than writing one, as `moved` reads it.
pub(super) fn given<'a>(
    flags: &'a BTreeMap<&str, String>,
    at: &str,
) -> Result<Option<Owned<'a>>, String> {
    let (owner, what) = match (flags.get("--owner"), flags.get("--what")) {
        (Some(owner), Some(what)) => (owner.trim(), what.trim()),
        (_, None) => return Ok(None),
        (None, Some(_)) => return Err("--owner and --what write a miss together".to_owned()),
    };
    if !plain(owner) || !plain(what) {
        return Err("--owner and --what each say theirs plainly, on one line".to_owned());
    }
    if !stamped(at) {
        return Err(format!(
            "--miss takes when it happened as 2026-11-24T20:38:19-05:00, not {at}"
        ));
    }
    Ok(Some(Owned { owner, what }))
}

/// The cell `--owner` alone moves a miss to, if it's given without
/// `--what`, which would write one instead, or why it can't be one: said
/// plainly on one line.
pub(super) fn moved<'a>(flags: &'a BTreeMap<&str, String>) -> Result<Option<&'a str>, String> {
    if flags.contains_key("--what") {
        return Ok(None);
    }
    let Some(owner) = flags.get("--owner").map(|owner| owner.trim()) else {
        return Ok(None);
    };
    if plain(owner) {
        Ok(Some(owner))
    } else {
        Err("--owner says the cell a miss moves to plainly, on one line".to_owned())
    }
}

/// The change `--fix` names, if it's given, or why it can't be one: a
/// repository and its number after `#`, as cadence reads a check.
pub(super) fn fix<'a>(flags: &'a BTreeMap<&str, String>) -> Result<Option<&'a str>, String> {
    let Some(fix) = flags.get("--fix").map(|fix| fix.trim()) else {
        return Ok(None);
    };
    let (repo, number) = fix.rsplit_once('#').unwrap_or_default();
    let named = plain(fix)
        && repo.contains(|c: char| c.is_ascii_alphanumeric())
        && !number.is_empty()
        && number.chars().all(|c| c.is_ascii_digit());
    if named {
        Ok(Some(fix))
    } else {
        Err(format!(
            "\"{fix}\" isn't a change as REPO #N, such as \"core #6470\""
        ))
    }
}

/// Whether `said` holds something and reads as it shows: no control,
/// line or paragraph break, nor a mark turning text around on the card.
fn plain(said: &str) -> bool {
    !said.is_empty()
        && !said.chars().any(|c| {
            c.is_control()
                || matches!(
                    c,
                    '\u{2028}'
                        | '\u{2029}'
                        | '\u{200e}'
                        | '\u{200f}'
                        | '\u{202a}'..='\u{202e}'
                        | '\u{2066}'..='\u{2069}'
                )
        })
}

/// Whether `at` is a time as the ledger writes one, to the second with its
/// offset and each part in its range, so cadence reads it and the owner
/// can name it again.
fn stamped(at: &str) -> bool {
    let shape = "dddd-dd-ddTdd:dd:dd+dd:dd";
    let shaped = at.len() == shape.len()
        && at.chars().zip(shape.chars()).all(|(c, s)| match s {
            'd' => c.is_ascii_digit(),
            '+' => c == '+' || c == '-',
            _ => c == s,
        });
    let part = |from: usize| {
        at.get(from..from + 2)
            .and_then(|part| part.parse::<u8>().ok())
            .unwrap_or(u8::MAX)
    };
    shaped
        && (1..=12).contains(&part(5))
        && (1..=31).contains(&part(8))
        && part(11) < 24
        && part(14) < 60
        && part(17) < 60
        && part(20) < 24
        && part(23) < 60
}

/// Writes `miss`, nothing heard, at `at` in `ledger`, started if it isn't
/// there yet, with what `saying` says of it. The same miss written again
/// takes what it lacked; a time another miss holds is refused.
pub(super) fn own_in(
    ledger: &Path,
    at: &str,
    miss: Owned<'_>,
    saying: Saying<'_>,
) -> Result<String, Failure> {
    let Owned { owner, what } = miss;
    changed_in(ledger, true, |before, name| {
        let mut lines: Vec<String> = before.lines().map(str::to_owned).collect();
        let mut taken: Vec<(usize, Value)> = lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| Some((index, serde_json::from_str::<Value>(line).ok()?)))
            .filter(|(_, line)| line["at"].as_str() == Some(at))
            .collect();
        let same = |line: &Value| {
            line["heard_by"] == OWNER && line["owner"] == owner && line["what"] == what
        };
        let said = match (taken.pop(), taken.is_empty()) {
            (None, _) => {
                let mut line = json!({"at": at, "what": what, "owner": owner, "heard_by": OWNER});
                saying.on(&mut line);
                lines.push(line.to_string());
                match saying.says(false) {
                    more if more.is_empty() => {
                        format!("pain-points: wrote {owner}'s miss at {at}, owing its test")
                    }
                    more => format!("pain-points: wrote {owner}'s miss at {at}, which {more}"),
                }
            }
            (Some((index, mut line)), true) if same(&line) => {
                if saying.on(&mut line) {
                    let said = format!("pain-points: {owner}'s miss at {at} is written already");
                    return Ok((None, said));
                }
                lines[index] = line.to_string();
                format!(
                    "pain-points: {owner}'s miss at {at} now {}",
                    saying.says(false)
                )
            }
            _ => {
                let refused = format!("a miss in {name} is at {at} already");
                let failed = Failure::new(Exit::Usage, refused);
                return Err(failed.try_this("its own time, or that miss's by --miss alone"));
            }
        };
        lines.push(String::new());
        Ok((Some(lines.join("\n")), said))
    })
}

#[cfg(test)]
mod tests {
    use super::super::tests::{AT, ledger};
    use super::super::{Said, name_in, named};
    use super::*;
    use std::fs;

    const DU: Owned<'static> = Owned {
        owner: "0FX11G",
        what: "a du walked his guarded folders",
    };

    const NOTHING: Saying<'static> = Saying {
        said: None,
        fix: None,
        owner: None,
    };

    /// What it says of a miss: `test` as its test, and `fix` as its fix.
    fn saying<'a>(test: Option<&'a str>, fix: Option<&'a str>) -> Saying<'a> {
        Saying {
            said: test.map(Said::Test),
            fix,
            owner: None,
        }
    }

    /// Each line of the ledger at `path`, read.
    fn read(path: &Path) -> Vec<Value> {
        fs::read_to_string(path)
            .expect("the ledger")
            .lines()
            .map(|line| serde_json::from_str(line).expect("a line of the ledger"))
            .collect()
    }

    #[test]
    fn an_owner_writes_a_miss_nothing_heard_in_its_own_words_never_as_his() {
        let other = json!({"at": "2026-11-15T00:28:00-05:00", "owner": "5FWIUS"});
        let path = ledger("owned", &other.to_string());
        let said = own_in(&path, AT, DU, NOTHING).expect("written");
        assert!(
            said.contains("owing its test"),
            "it didn't say a test is owed: {said}"
        );
        let kept = fs::read_to_string(&path).expect("the ledger");
        let lines = read(&path);
        assert_eq!(lines.len(), 2, "it didn't add one line: {kept}");
        assert_eq!(lines[0], other, "another miss's line changed");
        let mine = json!({"at": AT, "what": DU.what, "owner": DU.owner, "heard_by": OWNER});
        assert_eq!(
            lines[1], mine,
            "the miss isn't its owner's alone, at its time"
        );
        assert!(kept.ends_with('\n'), "the ledger lost its last line's end");
        let again = own_in(&path, AT, DU, NOTHING).expect("written again");
        assert!(
            again.contains("already"),
            "writing it twice said nothing of it: {again}"
        );
        assert_eq!(
            fs::read_to_string(&path).expect("the ledger"),
            kept,
            "writing it twice changed the ledger"
        );
    }

    #[test]
    fn the_same_miss_written_again_takes_the_test_and_fix_it_lacked() {
        let path = ledger("owned-again", "");
        own_in(&path, AT, DU, NOTHING).expect("written");
        let tested = own_in(&path, AT, DU, saying(Some("core a b"), None)).expect("tested");
        assert!(
            tested.contains("now names its test, core a b"),
            "it didn't say it named its test: {tested}"
        );
        let fixed = own_in(&path, AT, DU, saying(None, Some("core #6470"))).expect("fixed");
        assert!(
            fixed.contains("now names its fix, core #6470"),
            "it didn't say it named its fix: {fixed}"
        );
        let lines = read(&path);
        assert_eq!(
            (lines.len(), &lines[0]["test"], &lines[0]["check"]),
            (1, &json!("core a b"), &json!("core #6470")),
            "the miss didn't take its test and fix on its one line"
        );
    }

    #[test]
    fn a_ledger_not_yet_there_is_started_in_a_folder_that_is() {
        let path = ledger("owned-started", "");
        fs::remove_file(&path).expect("the ledger gone");
        let said = own_in(&path, AT, DU, saying(Some("core a b"), None)).expect("started");
        assert!(
            said.contains("which names its test"),
            "it didn't say what it wrote: {said}"
        );
        assert_eq!(
            read(&path).len(),
            1,
            "the started ledger doesn't hold the one miss"
        );
        let nowhere = path.with_file_name("no-folder").join("misses.jsonl");
        let failed = own_in(&nowhere, AT, DU, NOTHING).expect_err("started nowhere");
        assert!(
            matches!(failed.exit, Exit::NotFound),
            "a ledger in no folder wasn't refused as not found"
        );
    }

    #[test]
    fn a_miss_its_owner_writes_at_a_time_another_holds_is_refused() {
        let heard = json!({"at": AT, "words": "stop"});
        let another = json!({"at": AT, "owner": DU.owner, "what": "else", "heard_by": OWNER});
        for (line, what) in [(heard, "a heard miss"), (another, "another of its own")] {
            let text = format!("{line}\n");
            let path = ledger("owned-taken", &text);
            let failed = own_in(&path, AT, DU, NOTHING).expect_err("written");
            assert!(
                matches!(failed.exit, Exit::Usage),
                "{what} at its time wasn't refused as usage"
            );
            assert_eq!(
                fs::read_to_string(&path).expect("the ledger"),
                text,
                "{what}: the ledger was changed"
            );
        }
    }

    #[test]
    fn a_miss_its_owner_wrote_takes_a_fix_or_a_none_and_a_heard_one_no_fix() {
        let heard_at = "2026-11-15T00:28:00-05:00";
        let heard = json!({"at": heard_at, "words": "stop", "heard_by": "pain-points"});
        let path = ledger("owned-fix", &format!("{heard}\n"));
        own_in(&path, AT, DU, NOTHING).expect("written");
        name_in(&path, AT, saying(None, Some("core #6470"))).expect("its fix named");
        assert_eq!(
            read(&path)[1]["check"],
            "core #6470",
            "the owner's miss didn't take its fix"
        );
        let before = fs::read_to_string(&path).expect("the ledger");
        let failed = name_in(&path, heard_at, saying(None, Some("core #1"))).expect_err("fixed");
        assert!(
            matches!(failed.exit, Exit::Usage),
            "a heard miss given a fix wasn't refused as usage"
        );
        assert_eq!(
            fs::read_to_string(&path).expect("the ledger"),
            before,
            "a heard miss given a fix changed the ledger"
        );
        let none = Said::NoMiss {
            why: "written at the wrong time",
            at_ms: 1,
        };
        name_in(&path, AT, none).expect("said none");
        assert_eq!(
            read(&path)[1]["none"],
            "written at the wrong time",
            "the owner couldn't say its own miss was none"
        );
    }

    #[test]
    fn a_miss_written_half_said_said_none_or_not_plainly_is_refused_for_that_alone() {
        let nowhere = std::env::temp_dir().join(format!("owned-refused-{}", std::process::id()));
        let (owner, what) = (("--owner", "0FX11G"), ("--what", "a du walked his folders"));
        let test = ("--test", "core a b");
        for (at, given, says) in [
            (AT, &[what, test][..], "together"),
            (AT, &[("--owner", "a\nb")][..], "plainly"),
            (AT, &[("--owner", " "), test][..], "plainly"),
            (AT, &[owner, what, ("--none", "x")][..], "once it's written"),
            (AT, &[("--owner", " "), what, test][..], "plainly"),
            (AT, &[owner, ("--what", "a\nb"), test][..], "plainly"),
            (AT, &[owner, ("--what", "a\u{202e}b"), test][..], "plainly"),
            ("2026-11-24 20:38", &[owner, what, test][..], "--miss takes"),
            (
                "2026-13-24T20:38:19-05:00",
                &[owner, what][..],
                "--miss takes",
            ),
            (
                AT,
                &[owner, what, ("--fix", "core 6470")][..],
                "isn't a change",
            ),
            (AT, &[owner, what, ("--fix", "#6470")][..], "isn't a change"),
        ] {
            let mut flags: BTreeMap<&str, String> = given
                .iter()
                .map(|&(flag, value)| (flag, value.to_owned()))
                .collect();
            flags.insert("--miss", at.to_owned());
            flags.insert("--ledgers", nowhere.display().to_string());
            let failed = named(&flags).expect_err("it was taken");
            assert!(
                matches!(failed.exit, Exit::Usage) && failed.what.contains(says),
                "{given:?} at {at} wasn't refused as usage saying \"{says}\": {}",
                failed.what
            );
        }
    }

    #[test]
    fn a_miss_is_at_a_time_to_the_second_with_its_offset_and_each_part_in_range() {
        for at in [AT, "2026-11-24T20:38:19+01:00"] {
            assert!(stamped(at), "{at} was refused");
        }
        for at in [
            "",
            "2026-11-24T20:38-05:00",
            "2026-11-24T20:38:19Z",
            "2026-11-24 20:38:19-05:00",
            "2026-11-24T20:38:19-05:00 ",
            "2026-00-24T20:38:19-05:00",
            "2026-11-32T20:38:19-05:00",
            "2026-11-24T24:38:19-05:00",
            "2026-11-24T20:60:19-05:00",
            "2026-11-24T20:38:19-05:60",
        ] {
            assert!(!stamped(at), "\"{at}\" was taken as a time");
        }
    }
}
