//! The misses Joel found himself, and whether the check each one's owner
//! named runs here: a miss closes on the merge itself, read from the
//! repository's main, once this machine runs it, never on the owner's word
//! that a check exists. Long because its cases each replay a miss the card
//! misread, and they read best beside the rule each one pins.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Class, DAY, Measure, count, first_owner, lines, seconds};
use crate::{axes, command, write_whole};

mod held;
mod running;
mod symbiosis;

pub use held::{BLIND, HELD, held};
pub use running::{UNRUN, UNSEEN, hosts_started};
pub use symbiosis::{Findings, KEY as FINDINGS, findings, held_unseen};

/// How far before the oldest miss naming a repository its main is read: a
/// check may be one that merged a little before Joel found the miss. Two
/// days of a busy main is some 9,000 merges, read in about a second.
const BEFORE: i64 = 2 * DAY;

/// A change a check names, such as `core #4568`: its repository, as cadence
/// calls it, and its number there.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Named {
    pub repo: String,
    pub number: u64,
}

/// The checkouts whose mains cadence can read, by the name a check calls
/// each by, lowercased: core's and the harness's mirrors, and any the
/// machine names with `--checkout`.
pub type Checkouts = BTreeMap<String, PathBuf>;

/// The repository the word ending `before` names, as the word before a
/// `#N`: lowercased, its quotes and punctuation trimmed and any owner
/// before a `/` dropped, so "`core`", "Jitsusama/core.nix" and
/// "core.nix" are one.
fn repo_word(before: &str) -> String {
    let word = before
        .trim_end()
        .rsplit(|c: char| c.is_whitespace() || c == '(' || c == ',')
        .next()
        .unwrap_or_default()
        .trim_matches(|c: char| !c.is_ascii_alphanumeric())
        .to_lowercase();
    match word.rsplit('/').next().unwrap_or_default() {
        "core.nix" => "core".to_owned(),
        "agentic-harness.pi" => "harness".to_owned(),
        other => other.to_owned(),
    }
}

/// The changes `check` names in a repository cadence can read: each `#N`
/// after its name, as `core #4568`, `core#4500` or `harness #1592`. A
/// number after any other word, such as a memory's, names nothing here.
fn named(check: &str, repos: &Checkouts) -> Vec<Named> {
    check
        .match_indices('#')
        .filter_map(|(at, _)| {
            let digits: String = check[at + 1..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            let repo = repo_word(&check[..at]);
            repos.contains_key(&repo).then_some(())?;
            Some(Named {
                repo,
                number: digits.parse().ok()?,
            })
        })
        .collect()
}

/// Every change the misses' checks name in a repository cadence can read,
/// each with when its miss was found, so each main is read from there.
fn named_in(text: &str, repos: &Checkouts) -> Vec<(Named, i64)> {
    lines(text)
        .filter_map(|miss| {
            let at = seconds(&miss["at"])?;
            let check = miss["check"].as_str()?;
            Some(
                named(check, repos)
                    .into_iter()
                    .map(move |change| (change, at)),
            )
        })
        .flatten()
        .collect()
}

/// The pull requests a main's merge subjects say they merged: GitHub's
/// "Merge pull request #N from", or a squashed merge's trailing "(#N)". A
/// revert's subject ends with its own number, not the one it undoes.
fn merged_numbers(subjects: &str) -> BTreeSet<u64> {
    subjects
        .lines()
        .filter_map(|subject| {
            let number = match subject.strip_prefix("Merge pull request #") {
                Some(rest) => rest.split_once(" from ")?.0,
                None => subject.trim_end().strip_suffix(')')?.rsplit_once("(#")?.1,
            };
            number.parse().ok()
        })
        .collect()
}

/// Each repository whose main must be read for `wanted`, and from when:
/// `BEFORE` its oldest miss whose change isn't yet `known` merged.
fn walks(wanted: &[(Named, i64)], known: &BTreeSet<Named>) -> BTreeMap<String, i64> {
    let mut since: BTreeMap<String, i64> = BTreeMap::new();
    for (change, at) in wanted.iter().filter(|(change, _)| !known.contains(change)) {
        let from = since.entry(change.repo.clone()).or_insert(at - BEFORE);
        *from = (*from).min(at - BEFORE);
    }
    since
}

/// The changes of `wanted` merged, `known` among them, and the repositories
/// whose main couldn't be read: each repository still wanting one has its
/// main's first-parent subjects read once, from its `origin/main` where it
/// has one, else its `main`.
fn merged(
    wanted: &[(Named, i64)],
    repos: &Checkouts,
    known: &BTreeSet<Named>,
) -> (BTreeSet<Named>, BTreeSet<String>) {
    let mut found = known.clone();
    let mut unread = BTreeSet::new();
    for (repo, since) in walks(wanted, known) {
        let Some(path) = repos.get(&repo) else {
            continue;
        };
        let path = path.to_string_lossy();
        let main = main_of(&path);
        let subjects = command(
            "git",
            &[
                "-C",
                &path,
                "log",
                "--first-parent",
                &format!("--since=@{since}"),
                "--format=%s",
                main,
            ],
        )
        // An unread main leaves its misses open, which is safe, but says
        // why: at 10:10 on 15 November World's merge went unfound and
        // nothing told which step failed.
        .unwrap_or_else(|error| {
            eprintln!("cadence: couldn't read {repo}'s {main} at {path}: {error}");
            unread.insert(repo.clone());
            String::new()
        });
        let numbers = merged_numbers(&subjects);
        found.extend(
            wanted
                .iter()
                .filter(|(change, _)| change.repo == repo && numbers.contains(&change.number))
                .map(|(change, _)| change.clone()),
        );
    }
    (found, unread)
}

/// A checkout's main: its `origin/main` where it has one, else its `main`.
fn main_of(path: &str) -> &'static str {
    if command(
        "git",
        &["-C", path, "rev-parse", "-q", "--verify", "origin/main"],
    )
    .is_ok()
    {
        "origin/main"
    } else {
        "main"
    }
}

/// The misses in `text`, each closed once its check merged on a main
/// cadence reads, core's and the harness's, as `folder` and pi-packages
/// keep them, and each of `checkouts`, as `NAME=PATH`, and runs here, as
/// the last of `switches` and the pi hosts' runtimes say. The changes
/// found merged are kept in `folder`, so a main is never read again for
/// them, and so are those seen running, so a closed miss stays closed.
/// The `findings` of the work system are judged in the same pass, so the
/// mains are walked and what runs here is read once for both: it gives
/// the misses' measure, and the findings' as of `now`.
///
/// # Errors
///
/// Where the changes found merged can't be kept.
pub fn checked_beside(
    text: &str,
    folder: &Path,
    checkouts: &[String],
    switches: &str,
    findings: Option<&Findings>,
    now: i64,
) -> io::Result<(Measure, Option<Measure>)> {
    let beside = findings.map_or("", Findings::text);
    let both = if beside.is_empty() {
        text.to_owned()
    } else {
        format!("{text}\n{beside}")
    };
    // Each measure judges its own lines, never the two joined, so a
    // finding can't be read as a miss Joel found himself.
    judged(&both, folder, checkouts, switches, |repos, judged| {
        (
            open_misses(text, repos, judged),
            findings.map(|found| found.measured(repos, judged, now)),
        )
    })
}

/// What `then` makes of the checks of `text`'s lines, each shaped as a
/// miss, once they are judged as the misses' are.
fn judged<T>(
    text: &str,
    folder: &Path,
    checkouts: &[String],
    switches: &str,
    then: impl FnOnce(&Checkouts, &Judged<'_>) -> T,
) -> io::Result<T> {
    let mut repos = Checkouts::from([
        ("core".to_owned(), folder.join("core.git")),
        ("harness".to_owned(), axes::landings::harness_mirror()),
    ]);
    for named in checkouts {
        if let Some((name, path)) = named.split_once('=') {
            repos.insert(name.to_lowercase(), path.into());
        }
    }
    let kept_at = folder.join("merged.json");
    let known = std::fs::read_to_string(&kept_at)
        .ok()
        .and_then(|kept| serde_json::from_str(&kept).ok())
        .unwrap_or_default();
    let (found, unread) = merged(&named_in(text, &repos), &repos, &known);
    if found != known {
        write_whole(&kept_at, &serde_json::to_string(&found)?)?;
    }
    let wanted = wanted_merged(text, &repos, &found);
    let unrun = running::unrun(&wanted, &repos, switches, folder);
    let here = std::fs::read_to_string("/proc/sys/kernel/hostname").unwrap_or_default();
    let judged = Judged {
        merged: &found,
        unread: &unread,
        unrun: &unrun,
        here: here.trim(),
    };
    Ok(then(&repos, &judged))
}

/// The changes the misses' checks name now that have merged: only these
/// are asked whether they run, since `merged` keeps every one ever found.
fn wanted_merged(text: &str, repos: &Checkouts, merged: &BTreeSet<Named>) -> BTreeSet<Named> {
    named_in(text, repos)
        .into_iter()
        .map(|(change, _)| change)
        .filter(|change| merged.contains(change))
        .collect()
}

/// What each miss in `misses.jsonl` was, in Joel's own words and when he
/// said them, in `zone`, by the text the misses measure names it with. The
/// card tells him one is fixed in these words, since a cause in a
/// session's words tells him nothing he can place: on 16 November he asked
/// that everything reaching him say what it belongs to without his looking
/// it up. Two misses named alike keep the older's words, as the card can't
/// tell them apart either.
#[must_use]
pub fn said(text: &str, zone: &TimeZone) -> BTreeMap<String, String> {
    let mut said = BTreeMap::new();
    for miss in lines(text) {
        let Some(words) = miss["words"].as_str() else {
            continue;
        };
        let when = miss["at"]
            .as_str()
            .and_then(|at| at.parse::<Timestamp>().ok())
            .map(|at| {
                at.to_zoned(zone.clone())
                    .strftime(" on %-d %b at %H:%M")
                    .to_string()
            })
            .unwrap_or_default();
        // One line of the card each: a newline in his words would end it,
        // and a double quote inside would close the quote early.
        let words = words.split_whitespace().collect::<Vec<_>>().join(" ");
        let words = words.replace('"', "'");
        said.entry(named_as(&miss).to_owned())
            .or_insert_with(|| format!("what you ran into{when}, \"{words}\""));
    }
    said
}

/// The text a miss is named by on the card and in the hour it keeps: its
/// owner's account of what went wrong, else Joel's own words.
fn named_as(miss: &Value) -> &str {
    miss["what"]
        .as_str()
        .or_else(|| miss["words"].as_str())
        .unwrap_or_default()
}

/// Reads the misses ledger at `path`, with which of the misses open last
/// hour, `was_open` as the card listed them, it no longer holds: a miss
/// closes on its check and never leaves the ledger, so one gone was cut or
/// renamed, and the hour can't say whether it was fixed. None when it
/// can't be read at all, or is missing, empty or torn while misses were
/// open; one not yet written holds none. Why is said on stderr.
pub fn ledger(path: &Path, was_open: &[String]) -> (String, Option<Vec<String>>) {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => {
            eprintln!("cadence: couldn't read {}: {e}", path.display());
            return (String::new(), None);
        }
    };
    // One missing, emptied or torn under open misses is caught mid-write
    // or broken, not cut: it names none gone, so the hour carries them all.
    let torn = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .any(|line| serde_json::from_str::<Value>(line).is_err());
    if !was_open.is_empty() && (torn || text.trim().is_empty()) {
        eprintln!(
            "cadence: {} is missing, empty or torn under open misses",
            path.display()
        );
        return (text, None);
    }
    let held: BTreeSet<String> = lines(&text)
        .map(|miss| named_as(&miss).to_owned())
        .collect();
    let gone: Vec<String> = was_open
        .iter()
        .filter(|open| {
            let what = open
                .rsplit_once(" · ")
                .map_or(open.as_str(), |(what, _)| what);
            !held.contains(what)
        })
        .cloned()
        .collect();
    if !gone.is_empty() {
        eprintln!(
            "cadence: {} no longer holds {} of last hour's open misses",
            path.display(),
            gone.len()
        );
    }
    (text, Some(gone))
}

#[cfg(test)]
fn misses(
    text: &str,
    repos: &Checkouts,
    merged: &BTreeSet<Named>,
    unread: &BTreeSet<String>,
) -> Measure {
    let judged = Judged {
        merged,
        unread,
        unrun: &BTreeMap::new(),
        here: "here",
    };
    open_misses(text, repos, &judged)
}

/// What the hour knows of the misses' checks: which `merged`, the
/// repositories whose main went `unread`, which merged but don't run here
/// yet with why (`unrun`), and the machine it runs on, `here`.
#[derive(Clone, Copy)]
struct Judged<'a> {
    merged: &'a BTreeSet<Named>,
    unread: &'a BTreeSet<String>,
    unrun: &'a BTreeMap<Named, &'static str>,
    here: &'a str,
}

/// The misses Joel found himself, from the orchestrator's `misses.jsonl`:
/// one stays open until its owner names the check that would have caught
/// it and that check has merged, as `merged` holds them, and runs here, as
/// `unrun` says of those that don't yet, with why. A check naming no
/// change cadence can see merge, such as a memory, leaves it open, since
/// its words alone would tell Joel it's fixed before it is. One whose check
/// is in an `unread` main says so, since the hour, not its owner, is then
/// what keeps it open: at 12:10 on 15 November the card asked the
/// orchestrator to steer a miss whose check had merged in a main stock git
/// couldn't open. A miss whose line names another `machine` than this one
/// stays open until the line says when its check `runs` there, since the
/// hour sees only this machine: at 22:10 on 19 November it told Joel his
/// Mac's log-out miss fixed on harness #1860 installed here.
fn open_misses(text: &str, repos: &Checkouts, judged: &Judged<'_>) -> Measure {
    let Judged {
        merged,
        unread,
        unrun,
        here,
    } = *judged;
    let changes = |miss: &Value| {
        miss["check"]
            .as_str()
            .map(|check| named(check, repos))
            .unwrap_or_default()
    };
    let open: Vec<(String, String, String)> = lines(text)
        .filter_map(|miss| {
            let changes = changes(&miss);
            let elsewhere = miss["machine"].as_str().filter(|machine| {
                !machine.eq_ignore_ascii_case(here) && miss["runs"].as_str().is_none()
            });
            let ran = |change: &Named| merged.contains(change) && !unrun.contains_key(change);
            if !changes.is_empty() && changes.iter().all(ran) && elsewhere.is_none() {
                return None;
            }
            let what = named_as(&miss).to_owned();
            let owner = first_owner(miss["owner"].as_str().unwrap_or_default());
            // A check merged but not yet running is held, as a measure holds
            // one, so its owner isn't asked again which check would catch it.
            let note = changes
                .iter()
                .find(|change| unread.contains(&change.repo))
                .map(|change| {
                    format!(
                        ", though cadence couldn't read {}'s main to see its check",
                        change.repo
                    )
                })
                .or_else(|| {
                    let (change, why) = changes
                        .iter()
                        .find_map(|change| Some((change, unrun.get(change)?)))?;
                    Some(format!(
                        "{HELD}{} #{}{UNRUN}{why}",
                        change.repo, change.number
                    ))
                })
                .or_else(|| {
                    let (machine, change) = (elsewhere?, changes.iter().find(|c| ran(c))?);
                    Some(format!(
                        "{HELD}{} #{}{UNRUN}its miss was on {machine}, and no one has said \
                         it runs there",
                        change.repo, change.number
                    ))
                })
                .unwrap_or_default();
            Some((what, owner, note))
        })
        .collect();
    // A line half-written as the hour reads, or broken, would drop its miss
    // from the open list and tell Joel it's fixed, so the hour judges none.
    let whole = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .all(|line| serde_json::from_str::<Value>(line).is_ok());
    let mut says =
        "misses Joel found himself, with no check yet running here to catch the next".to_owned();
    if !whole {
        says.push_str(UNREAD);
    }
    Measure {
        key: "misses".to_owned(),
        class: Class::Miss,
        says,
        now: count(open.len()),
        target: whole.then_some(0.0),
        // The oldest open miss's owner moves it first.
        owner: open.first().map(|(_, o, _)| o.clone()).unwrap_or_default(),
        behind: open
            .iter()
            .map(|(what, owner, note)| format!("{what} · {owner}{note}"))
            .collect(),
    }
}

/// What the misses measure says after its words when the ledger wasn't
/// read whole, and so judges nothing.
pub const UNREAD: &str = " (misses.jsonl wasn't read whole)";

#[cfg(test)]
mod tests {
    use super::*;

    const AT: &str = "2026-11-14T17:34:00-05:00";
    const AT_SECONDS: i64 = 1_794_695_640;

    pub(super) fn change(repo: &str, number: u64) -> Named {
        Named {
            repo: repo.to_owned(),
            number,
        }
    }

    pub(super) fn repos() -> Checkouts {
        ["core", "harness", "world"]
            .into_iter()
            .map(|name| (name.to_owned(), PathBuf::from(format!("/{name}"))))
            .collect()
    }

    /// Joel's zone in November, fixed so a test reads the same anywhere.
    fn eastern() -> TimeZone {
        TimeZone::fixed(jiff::tz::offset(-5))
    }

    #[test]
    fn each_miss_is_said_in_joels_own_words_and_when() {
        let text = [
            miss("core #1"),
            serde_json::json!({"at": "2026-11-03T14:05:00Z",
                "words": "the dock lost my screens"})
            .to_string(),
            serde_json::json!({"at": "this morning", "what": "the bar",
                "words": "it said \"done\"\n  and hung"})
            .to_string(),
            serde_json::json!({"at": AT, "what": "no words kept"}).to_string(),
            serde_json::json!({"at": AT, "what": "the cat waits", "words": "later"}).to_string(),
        ]
        .join("\n");
        assert_eq!(
            said(&text, &eastern()),
            BTreeMap::from([
                (
                    "the cat waits".to_owned(),
                    "what you ran into on 14 Nov at 17:34, \"w\"".to_owned()
                ),
                (
                    "the dock lost my screens".to_owned(),
                    "what you ran into on 3 Nov at 09:05, \"the dock lost my screens\"".to_owned()
                ),
                (
                    "the bar".to_owned(),
                    "what you ran into, \"it said 'done' and hung\"".to_owned()
                ),
            ]),
            "a miss wasn't said by Joel's own words, on one line, and when in his zone, \
             or one without his words was, or a later one named alike took the words"
        );
    }

    #[test]
    fn a_miss_is_said_by_the_name_the_measure_gives_it() {
        let text = [
            miss("core #1"),
            serde_json::json!({"at": AT, "words": "only words", "owner": "KNURBN"}).to_string(),
        ]
        .join("\n");
        let open = misses(&text, &repos(), &BTreeSet::new(), &BTreeSet::new());
        let said = said(&text, &eastern());
        for line in &open.behind {
            let (name, _) = line.rsplit_once(" · ").unwrap_or((line, ""));
            assert!(
                said.contains_key(name),
                "the card names a miss {name:?} that has no words of Joel's: {said:?}"
            );
        }
    }

    pub(super) fn miss(check: &str) -> String {
        serde_json::json!({"at": AT, "words": "w", "what": "the cat waits", "owner": "XY9H5C",
            "check": check})
        .to_string()
    }

    #[test]
    fn a_miss_stays_open_until_a_check_is_named_for_it() {
        let text = concat!(
            r#"{"words":"w","what":"the map had no network","owner":"KNURBN (VKCF3Z)","check":null}"#,
            "\n",
            r#"{"words":"w","what":"History paused","owner":"RCTRF8","check":"core#4500"}"#,
            "\n",
        );
        let merged = BTreeSet::from([change("core", 4500)]);
        let m = misses(text, &repos(), &merged, &BTreeSet::new());
        assert!((m.now - 1.0).abs() < 0.5, "one open, got {}", m.now);
        assert_eq!(m.owner, "KNURBN", "its first owner moves it");
        assert_eq!(
            m.behind,
            ["the map had no network · KNURBN"],
            "it says what it was"
        );
    }

    /// At 21:10 on 19 November the card told Joel his 19:05 miss was fixed on
    /// core #5800's merge, which neither the last switch nor the pi hosts
    /// carried yet.
    #[test]
    fn a_merged_check_this_machine_doesnt_run_keeps_its_miss_open() {
        let text = format!(
            "{}\n{}\n",
            miss("core #5800's red case"),
            miss_of("core #4500")
        );
        let merged = BTreeSet::from([change("core", 5800), change("core", 4500)]);
        let unrun = BTreeMap::from([(change("core", 5800), "the pi hosts don't load it yet")]);
        let judged = Judged {
            merged: &merged,
            unread: &BTreeSet::new(),
            unrun: &unrun,
            here: "here",
        };
        let m = open_misses(&text, &repos(), &judged);
        assert!(
            (m.now - 1.0).abs() < 0.5,
            "the unrun one open, got {}",
            m.now
        );
        assert_eq!(
            m.behind,
            [format!(
                "the cat waits · XY9H5C{HELD}core #5800{UNRUN}the pi hosts don't load it yet"
            )],
            "it says its check merged and why it doesn't run yet"
        );
        assert_eq!(
            wanted_merged(&text, &repos(), &merged),
            merged,
            "a merged check wasn't asked whether it runs"
        );
        assert!(
            wanted_merged(&text, &repos(), &BTreeSet::new()).is_empty(),
            "a check not merged was asked whether it runs"
        );
    }

    fn miss_of(check: &str) -> String {
        serde_json::json!({"at": AT, "words": "x", "what": "the dock", "owner": "Y2YBTS",
            "check": check})
        .to_string()
    }

    /// At 22:10 on 19 November the card told Joel his Mac's log-out miss was
    /// fixed on harness #1860, installed here but not on the Mac.
    #[test]
    fn a_miss_on_another_machine_stays_open_until_its_check_runs_there() {
        let on = |machine: &str, runs: Option<&str>| {
            let mut miss: Value = serde_json::from_str(&miss("harness #1860")).expect("a miss");
            miss["machine"] = machine.into();
            if let Some(runs) = runs {
                miss["runs"] = runs.into();
            }
            format!("{miss}\n")
        };
        let merged = BTreeSet::from([change("harness", 1860)]);
        let judged = Judged {
            merged: &merged,
            unread: &BTreeSet::new(),
            unrun: &BTreeMap::new(),
            here: "here",
        };
        let m = open_misses(&on("yonder", None), &repos(), &judged);
        assert_eq!(
            m.behind,
            [format!(
                "the cat waits · XY9H5C{HELD}harness #1860{UNRUN}its miss was on yonder, and \
                 no one has said it runs there"
            )],
            "a miss on another machine closed on its check running here"
        );
        let runs = on("yonder", Some("2026-11-19T23:00:00-05:00"));
        assert!(
            open_misses(&runs, &repos(), &judged).behind.is_empty(),
            "a miss whose check the ledger says runs there stayed open"
        );
        assert!(
            open_misses(&on("Here", None), &repos(), &judged)
                .behind
                .is_empty(),
            "a miss on this machine waited on the ledger"
        );
    }

    #[test]
    fn a_miss_whose_check_has_not_merged_stays_open() {
        let two = format!(
            "{}\n{}\n",
            miss("core #4568 (the deleted-row check; merge ~07:30)"),
            miss("rule memory #5331, plus harness #1592 (mesh gate), merge ~06:00")
        );
        assert_eq!(
            named_in(&two, &repos()),
            [
                (change("core", 4568), AT_SECONDS),
                (change("harness", 1592), AT_SECONDS)
            ],
            "a check's changes in the repositories cadence reads weren't each found"
        );
        let open = misses(&two, &repos(), &BTreeSet::new(), &BTreeSet::new());
        assert_eq!(
            open.behind,
            ["the cat waits · XY9H5C", "the cat waits · XY9H5C"],
            "a miss closed on a check named but not merged"
        );
        let one = misses(
            &two,
            &repos(),
            &BTreeSet::from([change("harness", 1592)]),
            &BTreeSet::new(),
        );
        assert!(
            (one.now - 1.0).abs() < 0.5,
            "a merged check didn't close its miss, or an unmerged one did: {}",
            one.now
        );
    }

    #[test]
    fn a_check_in_a_checkout_the_machine_names_closes_once_it_merges() {
        let world = miss("a-backup-that-works-again-says-so (World #2450001, live switch949)");
        assert_eq!(
            named_in(&world, &repos()),
            [(change("world", 2_450_001), AT_SECONDS)],
            "a checkout's name, as the check capitalises it, wasn't read"
        );
        let mut unread = repos();
        unread.remove("world");
        let merged = BTreeSet::from([change("world", 2_450_001)]);
        assert!(
            (misses(&world, &unread, &merged, &BTreeSet::new()).now - 1.0).abs() < 0.5,
            "a check in a repository cadence can't read closed its miss"
        );
        assert!(
            misses(&world, &repos(), &merged, &BTreeSet::new()).now < 0.5,
            "a merged check in a named checkout didn't close its miss"
        );
    }

    #[test]
    fn a_miss_whose_main_went_unread_says_so_rather_than_wait_on_its_owner() {
        let world = miss("a-backup-that-works-again-says-so (World #2450001, live switch949)");
        let open = misses(
            &world,
            &repos(),
            &BTreeSet::new(),
            &BTreeSet::from(["world".to_owned()]),
        );
        assert_eq!(
            open.behind,
            ["the cat waits · XY9H5C, though cadence couldn't read world's main to see its check"],
            "an open miss blamed its owner for a main the hour itself couldn't read"
        );
    }

    /// A line half-written as the hour reads, or broken, would drop its
    /// miss from the open list and tell Joel it's fixed.
    #[test]
    fn a_ledger_line_that_doesnt_parse_leaves_the_misses_unjudged() {
        let text = format!("{}\n{{\"what\": \"half writ", miss("core #9"));
        let m = misses(&text, &repos(), &BTreeSet::new(), &BTreeSet::new());
        assert!(
            m.target.is_none() && m.says.contains("wasn't read whole"),
            "a ledger with a broken line was judged: {m:?}"
        );
        let spaced = format!("{}\n\n  \n", miss("core #9"));
        let m = misses(&spaced, &repos(), &BTreeSet::new(), &BTreeSet::new());
        assert!(
            m.target.is_some(),
            "a ledger with blank lines was read as torn: {m:?}"
        );
    }

    /// A ledger that can't be read, is missing, reads empty or has a torn
    /// line while last hour had misses open says nothing of which are
    /// gone, so the hour carries them all: misses never leave it, they close
    /// on their checks, and one dropped would never be told fixed.
    #[test]
    fn a_ledger_missing_emptied_or_torn_names_no_miss_gone() {
        let root = tempfile::tempdir().expect("a scratch folder");
        let open = ["the cat waits · XY9H5C".to_owned()];
        let file = root.path().join("misses.jsonl");
        assert_eq!(
            ledger(&file, &open).1,
            None,
            "a missing ledger named misses gone"
        );
        std::fs::write(&file, "\n").expect("the ledger written");
        assert_eq!(
            ledger(&file, &open).1,
            None,
            "an emptied ledger named misses gone"
        );
        let torn = format!("{}\n{{\"what\": \"the cat wa", miss("core #9"));
        std::fs::write(&file, torn).expect("the ledger written");
        assert_eq!(
            ledger(&file, &open).1,
            None,
            "a torn ledger named misses gone"
        );
    }

    /// A ledger that can't be read, or reads empty while last hour had
    /// misses open, isn't read whole.
    #[test]
    fn a_ledger_unread_or_gone_empty_under_open_misses_is_not_read_whole() {
        let root = tempfile::tempdir().expect("a scratch folder");
        let open = ["the cat waits · XY9H5C".to_owned()];
        let file = root.path().join("misses.jsonl");
        let whole = |path: &Path, was_open: &[String]| {
            ledger(path, was_open).1.is_some_and(|gone| gone.is_empty())
        };
        assert_eq!(
            ledger(root.path(), &open).1,
            None,
            "a folder read as a ledger"
        );
        let cases = [
            (
                file.clone(),
                &open[..],
                false,
                "a missing ledger under open misses",
            ),
            (file.clone(), &[][..], true, "a ledger not yet written"),
        ];
        for (path, was_open, read_whole, what) in cases {
            assert_eq!(
                whole(&path, was_open),
                read_whole,
                "{what} read whole: {}",
                !read_whole
            );
        }
        std::fs::write(&file, "\n").expect("the ledger written");
        assert!(
            !whole(&file, &open),
            "a ledger gone empty under open misses was read whole"
        );
        assert!(
            whole(&file, &[]),
            "an empty ledger with nothing open wasn't read whole"
        );
        let other = r#"{"words":"w","what":"the map drew cells alone","owner":"WCJQSG"}"#;
        std::fs::write(&file, other).expect("the ledger written");
        assert_eq!(
            ledger(&file, &open).1,
            Some(open.to_vec()),
            "a ledger cut of a miss open last hour didn't name it gone"
        );
        // Only the miss's whole words hold it, not words it starts with.
        let longer = ["the cat waits · XY9H5C · KNURBN".to_owned()];
        std::fs::write(&file, miss("core #9")).expect("the ledger written");
        assert!(
            !whole(&file, &longer),
            "a miss whose words only begin another's was read as held"
        );
        assert_eq!(
            ledger(&file, &open),
            (miss("core #9"), Some(Vec::new())),
            "a whole ledger wasn't read as it is"
        );
    }

    #[test]
    fn a_main_says_what_merged_in_either_form_and_a_revert_its_own() {
        let subjects = "Merge pull request #4602 from Jitsusama/jitsusama/stacked\n\
                        Take Back the Quest Backup's Failure Notice (#2450001)\n\
                        Revert \"Make a Folder\" (#2450100) (#2450236)\n\
                        fix: the cat waits on #12\n";
        assert_eq!(
            merged_numbers(subjects),
            BTreeSet::from([4602, 2_450_001, 2_450_236]),
            "a merge's number was missed, or a mention or an undone one taken"
        );
    }

    #[test]
    fn a_main_is_read_from_before_its_oldest_miss_not_yet_known_merged() {
        let wanted = [
            (change("world", 1), AT_SECONDS - 100),
            (change("world", 2), AT_SECONDS),
            (change("core", 3), AT_SECONDS - 50),
        ];
        let known = BTreeSet::from([change("world", 1)]);
        assert_eq!(
            walks(&wanted, &known),
            BTreeMap::from([
                ("core".to_owned(), AT_SECONDS - 50 - BEFORE),
                ("world".to_owned(), AT_SECONDS - BEFORE),
            ]),
            "a walk went further back than its oldest unknown change, or read a known one"
        );
        assert!(
            walks(&wanted[..1], &known).is_empty(),
            "a main whose every change is known merged was read again"
        );
    }
}
