//! Whether each miss Joel found himself names a test that runs here to
//! catch it again. A miss's check was a sentence, and a change it named
//! merging said only that something landed: on 23 November every check in
//! the ledger was words, none a test that runs. A miss's line now names
//! its test as `test`, `REPO PATH NAME`, which its owner gives through
//! `pain-points test`, and the hour finds NAME in PATH at the revision of
//! REPO that runs here, which that repository's own checks ran before it
//! merged. A miss stays on this measure, open or closed, until it does.

use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;

use super::super::{Class, Measure, count, first_owner, lines};
use super::running::{RUN_HERE, running};
use super::{Checkouts, main_of, named_as, repo_word};

/// The measure's key, which its drag is filed under for each owner.
pub const UNTESTED: &str = "misses-untested";

/// What a miss's `test` names: its repository, as cadence calls it, the
/// path the test is in, and the words that find it there, its name.
#[derive(Debug, PartialEq, Eq)]
struct Test<'a> {
    repo: String,
    path: &'a str,
    name: &'a str,
}

/// The test `said` names, as `REPO PATH NAME`, or none when it isn't one:
/// a path git would read as an option names none.
fn test_of(said: &str) -> Option<Test<'_>> {
    let mut parts = said.trim().splitn(3, ' ');
    let repo = repo_word(parts.next()?);
    let path = parts.next()?;
    let name = parts.next()?.trim();
    let named = !repo.is_empty() && !path.is_empty() && !path.starts_with('-');
    (named && !name.is_empty()).then_some(Test { repo, path, name })
}

/// How the hour found a miss's test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Look {
    /// In its repository as it runs here.
    Runs,
    /// Not there: never merged, renamed, or named wrongly.
    Missing,
    /// In a repository cadence has no checkout of.
    Unknown,
    /// The hour couldn't look: its own blindness, not the owner's to move.
    Blind,
}

/// The misses in `text`, each judged by whether the test its line names
/// is in its repository as it runs here: a repository cadence follows to
/// what runs, `RUN_HERE`, at the last switch's revision or the harness
/// installed, and any other at its main. Its checkouts are core's and the
/// harness's, as `folder` and pi-packages keep them, and each of
/// `checkouts`, as `NAME=PATH`.
#[must_use]
pub fn untested(text: &str, folder: &Path, checkouts: &[String], switches: &str) -> Measure {
    let repos = super::checkouts_of(folder, checkouts);
    let revs = running(switches, &repos);
    judged(text, |test| {
        let Some(path) = repos.get(&test.repo) else {
            return Look::Unknown;
        };
        let at = path.to_string_lossy();
        let rev = match revs.get(&test.repo) {
            Some(rev) => rev.as_str(),
            None if RUN_HERE.contains(&test.repo.as_str()) => return Look::Blind,
            None => main_of(&at),
        };
        found(path, rev, test)
    })
}

/// Whether `test` is in the checkout at `repo` at `rev`, by git's grep of
/// its path for its name as fixed text.
fn found(repo: &Path, rev: &str, test: &Test<'_>) -> Look {
    let grep = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["grep", "-q", "-F", "-e", test.name, rev, "--", test.path])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match grep.map(|status| status.code()) {
        Ok(Some(0)) => Look::Runs,
        Ok(Some(1)) => Look::Missing,
        _ => Look::Blind,
    }
}

/// The measure of `text`'s misses as `look` finds each one's test. A line
/// the hour couldn't look for is left off, so its owner isn't asked about
/// the hour's blindness, and said in the measure's words; a ledger not
/// read whole judges none.
fn judged(text: &str, look: impl Fn(&Test<'_>) -> Look) -> Measure {
    let whole = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .all(|line| serde_json::from_str::<Value>(line).is_ok());
    let mut behind = Vec::new();
    let mut blind = 0;
    // A pain the keeper carried into the asks is tracked there, not here.
    for miss in lines(text).filter(|miss| miss["carried"].as_bool() != Some(true)) {
        let why = match standing(&miss, &look) {
            Untested::No => continue,
            Untested::Unseen => {
                blind += 1;
                continue;
            }
            Untested::Because(why) => why,
        };
        let at = miss["at"].as_str().unwrap_or("no time");
        let owner = first_owner(miss["owner"].as_str().unwrap_or_default());
        behind.push(format!("{} ({at}), {why} · {owner}", named_as(&miss)));
    }
    let unseen = if blind > 0 {
        format!(" ({blind} couldn't be looked for)")
    } else {
        String::new()
    };
    let unread = if whole { "" } else { super::UNREAD };
    Measure {
        key: UNTESTED.to_owned(),
        class: Class::Cells,
        says: format!("misses Joel found himself naming no test that runs here{unseen}{unread}"),
        now: count(behind.len()),
        target: whole.then_some(0.0),
        // The oldest untested miss's owner moves it first.
        owner: behind
            .first()
            .and_then(|line| line.rsplit_once(" · "))
            .map(|(_, owner)| owner.to_owned())
            .unwrap_or_default(),
        behind,
    }
}

/// Whether a miss names a test that runs here.
enum Untested {
    /// It does.
    No,
    /// The hour couldn't look.
    Unseen,
    /// It doesn't, and why, for its line.
    Because(String),
}

/// Whether `miss` names a test that runs here, as `look` finds it.
fn standing(miss: &Value, look: impl Fn(&Test<'_>) -> Look) -> Untested {
    let Some(said) = miss["test"].as_str() else {
        return Untested::Because("naming no test".to_owned());
    };
    let Some(test) = test_of(said) else {
        return Untested::Because(format!("its test \"{said}\" isn't REPO PATH NAME"));
    };
    match look(&test) {
        Look::Runs => Untested::No,
        Look::Blind => Untested::Unseen,
        Look::Unknown => Untested::Because(format!(
            "its test is in {}, which cadence has no checkout of",
            test.repo
        )),
        Look::Missing => Untested::Because(format!(
            "its test {} isn't in {}'s {} as it runs here",
            test.name, test.repo, test.path
        )),
    }
}

#[cfg(test)]
mod tests;
