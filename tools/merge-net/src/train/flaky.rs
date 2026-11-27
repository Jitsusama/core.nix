//! A red candidate whose every failed check is no rider's lands rather
//! than splits. On 26 November main's own formatting red (Presented.js,
//! past biome's 500 lines after #6727) split candidate a8f33ede, and the
//! train was stopped by hand so it wouldn't send riders back to run alone
//! for it; on 26 November too #6524 touched no margin-cat file and its gate
//! failed the-margin-cat-follows-the-keys-on-niri, a launcher check on a
//! live compositor that had failed now and then on main.
//!
//! Each check the gate's words say a builder failed is built once more, at
//! the commit the candidate was cut on, on a lent builder, so like is
//! compared with like; a fresh main tests something else, and the train
//! cuts again when main moves anyway. One of `REPORTS_EVERY_FAILURE` that
//! fails there too, its builder saying nothing at the candidate it didn't
//! say there, is main's own red, unless the riders moved what a rule runs
//! (`moves_a_rule`), since the candidate builds its own copy of the rule.
//! Only those: a check that stops at its
//! first error says the same at both whatever the riders broke after it,
//! so a rider's break would land hidden behind main's. So `formatting`,
//! which began this, isn't one: treefmt stops at the first file no
//! formatter reads, zsh-parses at the first file it can't parse, and the
//! check diffs what the formatters rewrote only once all of them pass, so
//! main's formatting red still splits. One that passes
//! there is built once more at the candidate itself: passing
//! at the same commit it failed at, it is a flake, whatever kind of check
//! it is, and a second red there is the riders'. Which files a check reads
//! isn't asked: the train's gate builds no VM tests, which are all that
//! `owners` reads, and it counts a role's test against the role's folder
//! alone, on purpose, so it can't say a check is untouched.
//!
//! A red the gate didn't build whole, a part no builder took beside it,
//! is never excused, nor one whose words say an error nix or changed-checks
//! didn't explain as a failed build. Before one lands, every check the gate
//! built but main's own reds is built once more together at the candidate,
//! so each is known built: a nix killed mid-run says nothing of the checks
//! it never reached, and a failed derivation named like a check may have
//! been another's; the VM tests changed-checks lists but never builds
//! for the gate are left to the net, as ever. Nor is a red excused whose
//! gate ran again on its own base, whose words name a check with a dot,
//! which Nix would read as another's path, or whose rule crashed, which
//! said nothing of what it never reached. A derivation that failed for another reason than its
//! builder or a dependency, one no check names, a check that no longer evaluates, a
//! build no builder took, more failed checks than `MOST_REBUILT`, and a
//! check excused as a flake twice already in the ride count as the
//! riders', so the train splits on them as before, its log saying why of
//! each, so a judging that couldn't finish reads apart from a rider's
//! break. Each check excused is told to its owner as a finding naming the
//! gate's log and the rebuild's, and each flake is a line in the train's
//! log, naming the check and both commits, so flakes can be counted.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;

pub(super) use self::lent::{Lenders, Lent};
use super::gated::{Gated, RUNS_AGAIN, gated};
use crate::state::{State, now, short};

/// Whose a failed check's red is. Only a flake's and main's land; each
/// other is taken as the riders', but says why, so a red judging couldn't
/// finish reads apart from one a rider broke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Blame {
    /// It passed at the base and failed again at the candidate, or fails
    /// at the base otherwise than at the candidate: the riders broke it.
    Riders,
    /// It fails at the base too, but may stop at its first error, so a
    /// rider's break could hide behind main's red.
    Masked,
    /// It fails at the base too, but the riders moved what a rule runs,
    /// so their copy of the rule may print main's lines over a break.
    RuleMoved,
    /// No builder took a build that would judge it.
    Untaken,
    /// The ride excused it as a flake `MOST_EXCUSED` times already.
    Spent,
    /// It wasn't built again, since a check before it was the riders'.
    Unbuilt,
    /// It passed at the base, and at the candidate when built again.
    Flaky,
    /// It fails at the base too, and only as it fails there.
    Main,
}

impl Blame {
    /// Whether a red of this check lands rather than splits.
    fn lands(self) -> bool {
        matches!(self, Self::Flaky | Self::Main)
    }
}

/// Why a red wasn't judged check by check, so it is the riders'.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Declined {
    /// Nix said an error no failed build explains, or named no failed
    /// derivation.
    Unexplained,
    /// More checks failed than `MOST_REBUILT`.
    TooMany,
    /// A check the riders touched no longer evaluates.
    NoLongerEvaluates,
    /// The gate ran again on its own base, or its words say it did.
    RanAgain,
    /// changed-checks didn't say its builds ended with a failed check.
    NoVerdict,
    /// The gate built a check named with more than a plain name.
    NotPlain,
    /// A failed derivation is no check the gate built.
    Unplaced,
    /// The checks the gate built didn't all build again together.
    WholeRed,
}

impl Declined {
    /// How the train says why it declined.
    fn said(self) -> &'static str {
        match self {
            Self::Unexplained => "nix said an error no failed build explains",
            Self::TooMany => "more checks failed than the train builds again",
            Self::NoLongerEvaluates => "a check the change touched no longer evaluates",
            Self::RanAgain => "its gate's words say it ran again on its own base",
            Self::NoVerdict => "changed-checks didn't say a check it built failed",
            Self::NotPlain => "a check it built isn't plainly named",
            Self::Unplaced => "a failed derivation is no check its gate built",
            Self::WholeRed => "the checks its gate built didn't all build again",
        }
    }
}

/// How a red is judged: what each part asks of the world.
pub(super) struct Judging<'a> {
    /// The check each derivation name builds, for names no check has.
    pub named: &'a dyn Fn(&[String]) -> BTreeMap<String, String>,
    /// How the check built at the base, and the words it built with.
    pub at_base: &'a mut dyn FnMut(&str) -> (Gated, String),
    /// How the check built when built again at the candidate.
    pub again: &'a mut dyn FnMut(&str) -> Gated,
    /// How the checks built when built once more together at the
    /// candidate, which proves each of them built.
    pub whole: &'a mut dyn FnMut(&[String]) -> Gated,
    /// How many of the check's reds were excused as flakes in this ride.
    pub excused_before: &'a dyn Fn(&str) -> usize,
    /// Whether the riders moved what a rule runs, as `moves_a_rule` says.
    pub rules_moved: bool,
}

/// The names of the derivations whose builders failed, as the gate's
/// `words` say: `treefmt-check` for nix's "Cannot build
/// '/nix/store/<hash>-treefmt-check.drv'" followed by its reason, "builder
/// failed". One that couldn't build for a dependency names no check of its
/// own: its dependency's line does. None when one couldn't build for any
/// other reason, a timeout or a hash that doesn't match, since what broke
/// it can't then be told; and None when nix said an error of another kind,
/// a builder lost or a copy that failed, changed-checks or nix-offload
/// gave up with a `✕` of their own, or nix's closing list names a build no
/// line says failed, since a check may then never have been built.
pub(super) fn failed_derivations(words: &str) -> Option<Vec<String>> {
    let mut failed = Vec::new();
    let mut listed_failed = BTreeSet::new();
    let mut said_failed = BTreeSet::new();
    let mut lines = words.lines().peekable();
    while let Some(line) = lines.next() {
        if let Some(listed) = line.strip_prefix("error: build of ") {
            listed_failed.extend(listed.split("'/nix/store/").skip(1).filter_map(|path| {
                let (path, _) = path.split_once(".drv")?;
                Some(
                    path.split_once('-')
                        .map_or(path, |(_, name)| name)
                        .to_owned(),
                )
            }));
            continue;
        }
        // A traced error opens with a bare `error:`, its words below it.
        let another_error = line.starts_with("error:")
            && !line.starts_with("error: Cannot build '")
            && line != "error: Build failed due to failed dependency";
        if another_error || (line.starts_with('\u{2715}') && line != CHANGED_FAILED) {
            return None;
        }
        let Some(path) = line
            .split_once("Cannot build '/nix/store/")
            .and_then(|(_, rest)| rest.split_once(".drv'"))
            .map(|(path, _)| path)
        else {
            continue;
        };
        let reason = lines.peek().map_or("", |next| next.trim_start());
        let name = path.split_once('-').map_or(path, |(_, name)| name);
        said_failed.insert(name.to_owned());
        if reason.starts_with("Reason: builder failed") {
            if !failed.iter().any(|known| known == name) {
                failed.push(name.to_owned());
            }
        } else if !(reason.starts_with("Reason: ") && reason.ends_with(" failed.")) {
            // Neither its builder nor a dependency failed it.
            return None;
        }
    }
    listed_failed.is_subset(&said_failed).then_some(failed)
}

/// The most failed checks a red's judging builds again: past them the
/// red is the riders', as before, rather than holding the train while a
/// red across main builds once more one check at a time.
const MOST_REBUILT: usize = 4;

/// The most reds of one check a ride excuses as flakes: its next is the
/// riders', since a check that keeps failing isn't failing by chance.
const MOST_EXCUSED: usize = 2;

/// The checks whose red can be main's: each says every failure it finds
/// before it fails, so a rider's break says a line the base's red doesn't.
/// `formatting` isn't one, as the module's doc says.
const REPORTS_EVERY_FAILURE: &[&str] = &[
    // Each rule gathers what it says for every file, then exits once.
    "every-assert-says-what-was-wrong",
    "every-long-file-says-why",
    "every-fixed-wait-says-why",
    "every-suppression-says-why",
    "every-test-runs-or-fails",
];

/// Whether the riders, moving `files`, moved what a rule runs besides the
/// tree it reads: a rule's script, wrapper or waiting list, under
/// `tests/rules/`; the flake, which wires each rule and gives the checks
/// their `pkgs`; the root `checks.nix`, which hands every folder's checks
/// that `pkgs` and `self`, picks the folders and makes each check; or the
/// lock that gives them Python. A rule reads nothing else of the
/// repository but the tree it judges. A rider's rule that stops early, or
/// says a new failure as main's, would print main's lines and hide its
/// break behind main's red, so then no red is main's. None, the files
/// unread, moved them all.
pub(super) fn moves_a_rule(files: Option<&[String]>) -> bool {
    files.is_none_or(|files| files.iter().any(|file| runs_a_rule(file)))
}

/// Whether `file` is one of those `moves_a_rule` names.
fn runs_a_rule(file: &str) -> bool {
    file.starts_with("tests/rules/") || ["flake.nix", "flake.lock", "checks.nix"].contains(&file)
}

/// The checks the gate's `words` name as added or changed, which it built,
/// a row each of changed-checks' table: its kind, then its name.
pub(super) fn built(words: &str) -> Vec<String> {
    words
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let kind = words.next()?;
            let name = words.next()?;
            matches!(kind, "added" | "changed").then(|| name.to_owned())
        })
        .collect()
}

/// The checks whose failure the gate's `words` say main's quarantine
/// excused: changed-checks built each alone, and its red blocked nothing,
/// so the failure its build left in the words isn't the red's. Judging it
/// again would build a check broken at every base and blame the riders.
/// A builder can't say it, since nix's `-L` puts the builder's name before
/// each line it prints.
pub(super) fn quarantined(words: &str) -> BTreeSet<String> {
    words
        .lines()
        .filter_map(|line| {
            let said = line
                .strip_prefix("changed-checks: ")?
                .strip_suffix(", so it blocks nothing")?;
            let (name, _) = said.split_once(" failed, but ")?;
            plainly_named(name).then(|| name.to_owned())
        })
        .collect()
}

/// What changed-checks says when the builds it ran ended and one failed:
/// the one way it gives up for which each failure is a build's.
const CHANGED_FAILED: &str = "\u{2715} a check the change touched failed";

/// What changed-checks says when a check the riders touched no longer
/// evaluates, which no derivation's failure names, so it is theirs.
const NO_LONGER_EVALUATES: &str = "a check the change touched no longer evaluates";

/// A VM test's derivation is named for its run: `vm-test-run-vm-pi`.
const VM_RUN: &str = "vm-test-run-";

/// How a VM test's check is named. changed-checks lists the VM tests a
/// change moves but builds them only when asked, which the train's gate
/// never asks, so the net runs them after the merge.
const VM_TEST: &str = "vm-";

/// What a Python rule says when it crashes. A rule that crashed stopped
/// partway, so it said nothing of what it never reached, and its red
/// can't be read as main's however alike it reads.
const CRASHED: &str = "Traceback (most recent call last):";

/// The check each of the `failed` derivations builds, of the checks
/// `built`: the one of its name, the VM test it runs, or the one `named`
/// says, asked only of names neither gives; none when one of them names
/// no check, since what broke can't then be told.
pub(super) fn checks_of(
    failed: &[String],
    built: &[String],
    named: &dyn Fn(&[String]) -> BTreeMap<String, String>,
) -> Option<Vec<String>> {
    let by_name = |name: &str| {
        let check = name.strip_prefix(VM_RUN).unwrap_or(name);
        built
            .iter()
            .any(|one| one == check)
            .then(|| check.to_owned())
    };
    let unplaced: Vec<String> = failed
        .iter()
        .filter(|name| by_name(name).is_none())
        .cloned()
        .collect();
    let others: Vec<String> = built
        .iter()
        .filter(|check| {
            !failed
                .iter()
                .any(|name| by_name(name).as_deref() == Some(check))
        })
        .cloned()
        .collect();
    let named = if unplaced.is_empty() {
        BTreeMap::new()
    } else {
        named(&others)
    };
    failed
        .iter()
        .map(|name| by_name(name).or_else(|| named.get(name).cloned()))
        .collect()
}

/// What the builder of `derivation` said in `words`, as nix's `-L` prefixes
/// it, each line with its store hashes and its durations taken out, so a
/// build's time doesn't make the same failure differ. Other numbers stay:
/// a failure of subtest 7 isn't one of subtest 3.
fn said_by(words: &str, derivation: &str) -> BTreeSet<String> {
    let prefix = format!("{derivation}> ");
    words
        .lines()
        .filter_map(|line| line.strip_prefix(&prefix))
        .map(|line| {
            let mut kept = String::new();
            let mut rest = line;
            while let Some(at) = rest.find("/nix/store/") {
                kept.push_str(&rest[..at]);
                kept.push_str("/nix/store/");
                rest = &rest[at + "/nix/store/".len()..];
                rest = rest.split_once('-').map_or(rest, |(_, after)| after);
            }
            kept.push_str(rest);
            without_durations(&kept)
        })
        .collect()
}

/// The units a duration is said in, longest first, so `ms` isn't read as
/// `m` then `s`.
const DURATION_UNITS: &[&str] = &["ms", "us", "ns", "\u{b5}s", "s", "m", "h"];

/// `line` with each duration, `1799ms` or `2.5s`, said as `#ms` or `#s`.
fn without_durations(line: &str) -> String {
    let mut said = String::new();
    let mut rest = line;
    while let Some(at) = rest.find(|character: char| character.is_ascii_digit()) {
        let before = &rest[..at];
        let number = rest[at..]
            .find(|character: char| !character.is_ascii_digit() && character != '.')
            .map_or(rest.len(), |end| at + end);
        let after = &rest[number..];
        let unit = DURATION_UNITS.iter().find(|unit| {
            after.strip_prefix(**unit).is_some_and(|then| {
                !then.starts_with(|character: char| character.is_alphanumeric())
            })
        });
        // A number in a word, as in `vm2`, is part of the word.
        let in_word = before.ends_with(|character: char| character.is_alphanumeric());
        said.push_str(before);
        match unit {
            Some(unit) if !in_word => {
                said.push('#');
                said.push_str(unit);
                rest = &after[unit.len()..];
            }
            _ => {
                said.push_str(&rest[at..number]);
                rest = after;
            }
        }
    }
    said.push_str(rest);
    said
}

/// Whether `derivation` failed in the candidate's `words` only as it fails
/// in the base's: nothing its builder said there is new. A rider's second
/// break of a check main already fails says something new, so it isn't
/// hidden behind main's.
pub(super) fn fails_as_at_base(words: &str, base: &str, derivation: &str) -> bool {
    let said = said_by(words, derivation);
    !said.is_empty()
        && !said.iter().any(|line| line.contains(CRASHED))
        && said.is_subset(&said_by(base, derivation))
}

/// Whose the red of `check`, which `derivation` builds, is, when the
/// candidate's gate said `words`: built again at the base, then, if it
/// passes there, at the candidate. A check the ride can excuse no more
/// is built at the base only where its red there could still be main's.
fn blame(words: &str, check: &str, derivation: &str, judging: &mut Judging<'_>) -> Blame {
    let rule = REPORTS_EVERY_FAILURE.contains(&check);
    let spent = (judging.excused_before)(check) >= MOST_EXCUSED;
    if spent && !rule {
        return Blame::Spent;
    }
    let (at_base, base_words) = (judging.at_base)(check);
    match at_base {
        Gated::Red if !rule => Blame::Masked,
        Gated::Red if judging.rules_moved => Blame::RuleMoved,
        Gated::Red if fails_as_at_base(words, &base_words, derivation) => Blame::Main,
        Gated::Red => Blame::Riders,
        Gated::Unsent => Blame::Untaken,
        Gated::Passed if spent => Blame::Spent,
        Gated::Passed => match (judging.again)(check) {
            Gated::Passed => Blame::Flaky,
            Gated::Red => Blame::Riders,
            Gated::Unsent => Blame::Untaken,
        },
    }
}

/// Each check the candidate failed, as its gate's whole `words` say, with
/// whose red it is; or why it wasn't judged check by check, the words
/// naming no failed check or one no check builds, so the red is the
/// riders' as before. Once one is the riders', the red is, so those after
/// it are counted theirs unbuilt.
pub(super) fn judge(
    words: &str,
    judging: &mut Judging<'_>,
) -> Result<Vec<(String, Blame)>, Declined> {
    let quarantined = quarantined(words);
    let failed: Vec<String> = failed_derivations(words)
        .map(|failed| {
            failed
                .into_iter()
                .filter(|name| !quarantined.contains(name.strip_prefix(VM_RUN).unwrap_or(name)))
                .collect()
        })
        .filter(|failed: &Vec<String>| !failed.is_empty())
        .ok_or(Declined::Unexplained)?;
    if failed.len() > MOST_REBUILT {
        return Err(Declined::TooMany);
    }
    if words.contains(NO_LONGER_EVALUATES) {
        return Err(Declined::NoLongerEvaluates);
    }
    // A gate that ran again on its own base said twice what it built,
    // and a rider's build may print the words that say so to cut its
    // own break out of what is judged; the train's gates never replay.
    if words.contains(RUNS_AGAIN) {
        return Err(Declined::RanAgain);
    }
    if !words.lines().any(|line| line == CHANGED_FAILED) {
        return Err(Declined::NoVerdict);
    }
    let gate_built = built(words);
    // A check named with a dot would reach Nix as another check's path.
    if !gate_built.iter().all(|check| plainly_named(check)) {
        return Err(Declined::NotPlain);
    }
    let checks = checks_of(&failed, &gate_built, judging.named).ok_or(Declined::Unplaced)?;
    let mut riders = false;
    let judged: Vec<(String, Blame)> = checks
        .into_iter()
        .zip(&failed)
        .map(|(check, derivation)| {
            if riders {
                return (check, Blame::Unbuilt);
            }
            let blamed = blame(words, &check, derivation, judging);
            riders = !blamed.lands();
            (check, blamed)
        })
        .collect();
    if excused(&judged) {
        let mains = |check: &String| {
            judged
                .iter()
                .any(|(main, blamed)| main == check && *blamed == Blame::Main)
        };
        let rest: Vec<String> = gate_built
            .into_iter()
            .filter(|check| {
                !mains(check) && !check.starts_with(VM_TEST) && !quarantined.contains(check)
            })
            .collect();
        if !rest.is_empty() && (judging.whole)(&rest) != Gated::Passed {
            return Err(Declined::WholeRed);
        }
    }
    Ok(judged)
}

/// Whether a red whose failed checks are `judged` so is excused: some
/// failed, and each of them is main's or a flake's.
pub(super) fn excused(judged: &[(String, Blame)]) -> bool {
    !judged.is_empty() && judged.iter().all(|(_, blamed)| blamed.lands())
}

/// The reds of each check a ride has excused as flakes.
#[derive(Default)]
pub(super) struct Flakes(BTreeMap<String, usize>);

impl Flakes {
    /// How many of `check`'s reds this ride excused as flakes.
    fn of(&self, check: &str) -> usize {
        self.0.get(check).copied().unwrap_or(0)
    }

    /// Count each flake of an excused red's `judged` checks: those checks.
    fn count<'a>(&mut self, judged: &'a [(String, Blame)]) -> Vec<&'a str> {
        let flaked: Vec<&str> = judged
            .iter()
            .filter(|(_, blamed)| *blamed == Blame::Flaky)
            .map(|(check, _)| check.as_str())
            .collect();
        for check in &flaked {
            *self.0.entry((*check).to_owned()).or_default() += 1;
        }
        flaked
    }
}

/// The candidate a red is judged for: where it was cut, and what its gate
/// said.
pub(super) struct Red<'a> {
    /// The train's checkout, at the candidate.
    pub checkout: &'a Path,
    /// The commit the candidate was cut on.
    pub base: &'a str,
    /// The candidate.
    pub candidate: &'a str,
    /// Where the candidate's gate wrote its words.
    pub log: &'a Path,
}

/// After the `red` candidate failed its gate, under the net's state
/// folder `root`: whether every check it failed is main's or a flake's,
/// each built again on a builder `lent`, so it lands rather than splits.
/// The base is the repository at `url`'s; `flakes` counts this ride's.
/// Each check excused is told to its owner as a finding.
pub(super) fn excuses(
    root: &Path,
    url: &str,
    red: &Red<'_>,
    flakes: &mut Flakes,
    lent: &mut dyn Lent,
) -> bool {
    // All the gate said, not only what follows a replay, so a replay's
    // words, real or printed by a rider's build, are seen and refused.
    let words = String::from_utf8_lossy(&fs::read(red.log).unwrap_or_default()).into_owned();
    let flake = format!("github:{}/{}", super::slug_of(url), red.base);
    // Each build asks the builders in turn, never two at once.
    let lent = std::cell::RefCell::new(lent);
    let mut base_build = |check: &str| {
        let log = built_log(root, red.candidate, check, "at-base");
        let (code, words) = lent
            .borrow_mut()
            .build(red.checkout, &flake, &log, &[check], red.base);
        (at_base(code, &words), words)
    };
    let mut again = |check: &str| {
        let log = built_log(root, red.candidate, check, "again");
        let (code, words) =
            lent.borrow_mut()
                .build(red.checkout, ".", &log, &[check], red.candidate);
        gated(code, &words)
    };
    let mut whole = |checks: &[String]| {
        let log = built_log(root, red.candidate, "every-check", "again");
        let checks: Vec<&str> = checks.iter().map(String::as_str).collect();
        let (code, words) =
            lent.borrow_mut()
                .build(red.checkout, ".", &log, &checks, red.candidate);
        gated(code, &words)
    };
    let counted = |check: &str| flakes.of(check);
    // Named at the candidate, whose build named the failed derivations,
    // so a check the riders added doesn't sink the evaluation.
    let named = |checks: &[String]| lent.borrow().named(red.checkout, ".", checks);
    let mut judging = Judging {
        named: &named,
        at_base: &mut base_build,
        again: &mut again,
        whole: &mut whole,
        excused_before: &counted,
        rules_moved: moves_a_rule(moved_between(red.checkout, red.base, red.candidate).as_deref()),
    };
    let judged = judge(&words, &mut judging);
    let shown = short(red.candidate);
    let judged = match judged {
        Ok(judged) => judged,
        Err(declined) => {
            println!(
                "merge-net: the candidate {shown}'s red wasn't judged check by check: {}, \
                 so it's its riders'",
                declined.said()
            );
            return false;
        }
    };
    let said = judged
        .iter()
        .map(|(check, blamed)| format!("{check} {}", said(*blamed)))
        .collect::<Vec<_>>()
        .join("; ");
    if excused(&judged) {
        // Only a red that lands is told: one that splits is its riders'.
        tell(root, url, red, &judged);
        for check in flakes.count(&judged) {
            println!(
                "merge-net: flake {check} at {} on {}: red, then passed when built again",
                red.candidate, red.base
            );
        }
        println!(
            "merge-net: the candidate {shown} failed only checks that aren't its riders': \
             {said}, so it lands"
        );
        true
    } else {
        println!("merge-net: the candidate {shown} failed changed-checks: {said}");
        false
    }
}

/// Write a finding for each check of `judged` that isn't the riders', for
/// its owner, of the repository at `url`, under the net's state `root`.
fn tell(root: &Path, url: &str, red: &Red<'_>, judged: &[(String, Blame)]) {
    let failed: Vec<String> = judged.iter().map(|(check, _)| check.clone()).collect();
    let state = State::at(root.to_path_buf());
    for (check, blamed) in judged {
        let (kind, rebuilt) = match blamed {
            // Kinds of the train's own, since the watch takes a `flaky`
            // or `failed-before` finding at a commit as its own already
            // made there, and would then list nothing.
            Blame::Flaky => (
                "train-flaky",
                built_log(root, red.candidate, check, "again"),
            ),
            Blame::Main => (
                "train-failed-before",
                built_log(root, red.candidate, check, "at-base"),
            ),
            Blame::Riders
            | Blame::Masked
            | Blame::RuleMoved
            | Blame::Untaken
            | Blame::Spent
            | Blame::Unbuilt => {
                continue;
            }
        };
        let owner = owner_in(root, url, red, check, &failed);
        if owner.is_none() {
            println!("merge-net: no merge since the last passed head owns {check}");
        }
        // The red's own words are the gate's: a flake's rebuild passed.
        let finding = json!({
            "kind": kind,
            "test": check,
            "commit": red.base,
            "candidate": red.candidate,
            "owner_pr": owner,
            "log": red.log.display().to_string(),
            "rebuilt_log": rebuilt.display().to_string(),
            "at": now(),
        });
        if let Err(why) = state.find(&finding) {
            println!("merge-net: the finding for {check} wasn't written: {why}");
        }
    }
}

/// The PR that owns `check`, failed beside `failed`, as the net's watch
/// reads an owner: the newest merge from the head it last passed to the
/// candidate's base that moved the check, at the repository `url`, under
/// the state folder `root`; none where it passed nothing yet, or finds no
/// such merge. The merges are read in the train's checkout, which has just
/// fetched the base, rather than the watch's clone, which may not hold it.
fn owner_in(root: &Path, url: &str, red: &Red<'_>, check: &str, failed: &[String]) -> Option<u64> {
    let passed = State::at(root.to_path_buf()).heads().0?;
    let world = crate::world::Real {
        state: root.to_path_buf(),
        checkout: red.checkout.to_path_buf(),
        url: url.to_owned(),
        system: crate::SYSTEM.to_owned(),
        booted: 0,
        branch: "main".to_owned(),
        moving: std::sync::Mutex::new(()),
        dropped: std::sync::Mutex::default(),
        follow: None,
    };
    crate::watch::owner_of(&world, &passed, red.base, check, failed)
}

/// How the train says whose a red is.
fn said(blamed: Blame) -> &'static str {
    match blamed {
        Blame::Riders => "is its riders'",
        Blame::Masked => {
            "fails at the base too, but may stop at its first error, so it's taken as its \
             riders'"
        }
        Blame::RuleMoved => {
            "fails at the base too, but its riders moved what the rules run, so it's taken as \
             its riders'"
        }
        Blame::Untaken => "wasn't built again, no builder taking it, so it's taken as its riders'",
        Blame::Spent => {
            "was excused as a flake as often as a ride excuses one, so it's taken as its riders'"
        }
        Blame::Unbuilt => "wasn't built again, a check before it being its riders'",
        Blame::Flaky => "passed at the base and when built again, so it's a flake",
        Blame::Main => "fails at the base too, and only as it fails there, so it's main's",
    }
}

/// The files moved from `base` to `candidate` in the train's `checkout`;
/// none where git can't say.
fn moved_between(checkout: &Path, base: &str, candidate: &str) -> Option<Vec<String>> {
    let ran = Command::new("git")
        // Each name whole and unquoted, ended by a NUL, since git quotes a
        // name with an unusual byte otherwise, and a quoted name under
        // tests/rules/ wouldn't start with it.
        .args(["diff", "--name-only", "-z", "--no-renames", base, candidate])
        .current_dir(checkout)
        .output()
        .ok()
        .filter(|ran| ran.status.success())?;
    Some(
        String::from_utf8_lossy(&ran.stdout)
            .split('\0')
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect(),
    )
}

/// Where a build of `check` for a candidate writes its words, `how` saying
/// which build it is.
fn built_log(root: &Path, candidate: &str, check: &str, how: &str) -> PathBuf {
    super::folder(root).join(format!("{candidate}-{check}-{how}.log"))
}

/// How a check built at the base ended, from its exit `code` and `words`:
/// red only where a builder failed, since then main itself fails it. Any
/// other failure, an evaluation's or a fetch's, says nothing of main's
/// check, and a check the riders add isn't at the base to evaluate, so it
/// reads as untaken and the red stays the riders'.
pub(super) fn at_base(code: Option<i32>, words: &str) -> Gated {
    match code {
        Some(0) => Gated::Passed,
        _ if failed_derivations(words).is_some_and(|failed| !failed.is_empty()) => Gated::Red,
        _ => Gated::Unsent,
    }
}

/// Whether `check` is a name Nix reads as written inside a string, so
/// none can splice an expression into the evaluation that names checks.
fn plainly_named(check: &str) -> bool {
    !check.is_empty()
        && check
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_'".contains(character))
}

#[path = "flaky/lent.rs"]
mod lent;

#[cfg(test)]
#[path = "flaky/owner_tests.rs"]
mod owner_tests;

#[cfg(test)]
#[path = "flaky/quarantined_tests.rs"]
mod quarantined_tests;
#[cfg(test)]
#[path = "flaky/tests.rs"]
mod tests;
