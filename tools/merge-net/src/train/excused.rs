//! A red that says nothing of its riders excuses its candidate. When its
//! gate ran whole, each check it broke must be excused, as main's own or
//! as a flake, or the red holds. On 22 November at 16:11 and 16:46 two
//! candidates in a row fell on nested-niri checks neither rider touched,
//! which the watch saw both fail and pass at the base; each rider then ran
//! again alone.
//!
//! A flake passes when run again alone at the candidate. That holds the
//! candidate to the bar a rider sent back alone meets, one pass of its
//! gate, for the checks it broke. Main's own red fails at the base the
//! candidate was cut on, alike in every run seen there, and no run passes
//! there.
//!
//! Failing alike is printing the same lines: those nix prefixes with the
//! check's name, and for a pytest file only its failed subtests and their
//! `E` lines. They are compared as a multiset, equal both ways, with store
//! hashes masked and each number but a lone 0 read as one. So a count that
//! differs still matches, but none at all, a new error, file or subtest,
//! or a line fewer doesn't. A build that printed nothing is never excused.
//!
//! A red names a check by its derivation's name, which a rider chooses:
//! it could break a derivation named after a healthy check, or after one
//! main fails. So each derivation that failed must be its check's own at
//! the candidate, evaluated on a lent builder, or a part the check takes
//! in, and a check whose name isn't a plain attribute's, or that doesn't
//! evaluate, holds the red.
//!
//! The base's word comes first from the watch's runs there and what the
//! train keeps, a pass in any of them meaning the check isn't main's own;
//! a run main's quarantine excused is read as the failure it was.
//! The check runs again at the candidate next. Only when the base has
//! said nothing is one lent run there asked, kept in
//! `train/base/<base>/<check>.json` for the next candidate on that base.
//! Each run stops `ASKS_AT_MOST` after the first began, or once
//! `train/no-excuse` appears. A red the gate's words don't wholly explain
//! by checks that failed to build of themselves, one breaking more than
//! `MOST` checks, or one breaking a VM test, whose derivation isn't named
//! as its check is, is never excused; nor is a red whose gate nix-offload
//! didn't say ran every part to its end, or one beside a nearby machine's
//! part that didn't pass, whose checks only that machine runs. A file
//! `train/no-excuse` turns every excuse off.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use serde_json::{Value, json};

use crate::state::{Ran, State, short};

/// The most checks a red may break and still be excused: a red breaking
/// more is far likelier its riders', and each check asked costs a
/// builder's run while the train waits.
const MOST: usize = 4;

/// The most lines a check's failure may print and still be compared and
/// kept: a VM test's log runs to tens of megabytes, and one this long is
/// never one the excuse is for.
const MOST_LINES: usize = 20_000;

/// How long what the train asked of a base or a candidate is kept, as long
/// as the watch keeps a run's log, since a base main has moved past is
/// never asked again.
const KEPT_FOR: Duration = Duration::from_hours(72);

/// The longest the train waits on what it asks for one red, from its first
/// run: the nested-niri checks that fell on 22 November ran 2 to 8
/// minutes, and the riders sent back alone would have their gates by then.
const ASKS_AT_MOST: Duration = Duration::from_mins(40);

/// The system the train's gate checks, whose checks the base runs.
const SYSTEM: &str = "x86_64-linux";

/// What nix says of a derivation it couldn't build.
const CANNOT_BUILD: &str = "error: Cannot build '/nix/store/";

/// Why nix says a derivation failed of itself.
const BUILDER_FAILED: &str = "Reason: builder failed";

/// The start of nix's name for a NixOS VM test's derivation, which is
/// `vm-test-run-` and the test's name, not its check's.
const VM_TEST: &str = "vm-test-run-";

/// What a check split in parts names each, `<check>-part-<number>`, the
/// parts linked together as the check.
const PART: &str = "-part-";

/// The errors nix sums a failed build up with, which name nothing more
/// than its `Cannot build` lines already do.
const SUMMARIES: [&str; 2] = [
    "error: Build failed due to failed dependency",
    "error: build of '",
];

/// The lines a check's failed build printed, masked, each with how many
/// times it printed it.
pub(super) type Said = BTreeMap<String, usize>;

/// What runs `nix-offload` for the train: with its arguments, the commit
/// it builds at, the log its words go to, and when to let it go; how it
/// exited, none when it didn't end of itself. It may borrow for as long
/// as it is lent, which an alias without a lifetime would hold to `'static`.
pub(super) type Runner<'a> =
    dyn Fn(&[String], &str, &Path, &dyn Fn() -> bool) -> Option<i32> + Sync + 'a;

/// What a failed check did at the candidate's base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum AtBase {
    /// It failed there in every run seen, each printing these lines.
    Fails(Vec<Said>),
    /// It passed there, at least once.
    Passes,
}

/// Why a check a candidate broke says nothing of its riders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Excuse {
    /// It fails at the base alike.
    MainsOwn,
    /// It passed when run again alone at the candidate.
    PassedAgain,
}

/// What the excuse asks of a broken check, by its name.
pub(super) struct Asks<'a> {
    /// What the base already said, from the watch's runs and what the
    /// train keeps, running nothing.
    pub known: &'a (dyn Fn(&str) -> Option<AtBase> + Sync),
    /// Whether it passed when run again at the candidate; none when no
    /// run said.
    pub again: &'a (dyn Fn(&str) -> Option<bool> + Sync),
    /// What one lent run at the base says, kept for the next candidate.
    pub asked: &'a (dyn Fn(&str) -> Option<AtBase> + Sync),
    /// Its derivation at the candidate, evaluated on a lent builder; none
    /// when it couldn't be.
    pub named: &'a (dyn Fn(&str) -> Option<Named> + Sync),
}

/// The checks a red gate's `words` say failed of themselves, each by its
/// check's name, a part read as the check it is a part of, leaving out
/// those that failed only for a dependency; none when its words hold an
/// error no failed build explains, or a `Cannot build` for any other
/// reason, so that nothing they say can be excused.
pub(super) fn broken(words: &str) -> Option<Vec<String>> {
    Some(failures(words)?.into_keys().collect())
}

/// The derivations a red gate's `words` say failed of themselves, by the
/// check each is read as, as `broken` reads them: what the excuse holds
/// to the check's own derivation at the candidate, since a rider can name
/// any derivation after any check.
fn failures(words: &str) -> Option<BTreeMap<String, BTreeSet<String>>> {
    let lines: Vec<&str> = words.lines().map(str::trim).collect();
    let mut failed: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    let mut following = BTreeSet::new();
    for (at, line) in lines.iter().enumerate() {
        if let Some(rest) = line.strip_prefix(CANNOT_BUILD) {
            let (path, _) = rest.split_once(".drv'")?;
            let (_, name) = path.split_once('-')?;
            if !store_named(name) {
                return None;
            }
            let reason = lines.get(at + 1).copied().unwrap_or_default();
            if reason.starts_with(BUILDER_FAILED) {
                failed
                    .entry(name)
                    .or_default()
                    .insert(format!("/nix/store/{path}.drv"));
            } else if reason.starts_with("Reason: ") && reason.contains("dependenc") {
                following.insert(name);
            } else {
                return None;
            }
        } else if line.starts_with("error:")
            && !SUMMARIES.iter().any(|summary| line.starts_with(summary))
        {
            return None;
        }
    }
    let mut checks: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (name, paths) in failed {
        let check = part_of(name)
            .filter(|check| following.contains(check))
            .unwrap_or(name);
        checks.entry(check.to_owned()).or_default().extend(paths);
    }
    Some(checks)
}

/// Whether `name` is one a check of the flake's `checks.<system>` can be
/// asked by as it stands: letters, digits, dashes and underscores, never
/// a dot, which would name an attribute inside another.
fn attribute_named(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
}

/// A check's own derivation at the candidate, and the derivations it
/// takes in, among them its parts when it is split in parts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Named {
    /// The check's own derivation.
    pub own: String,
    /// The derivations it takes in.
    pub inputs: BTreeSet<String>,
}

/// Why `failed`, the derivations a red said failed under `check`'s name,
/// aren't `check`'s own at the candidate, as `asks` evaluates it: each
/// must be its derivation, or a part of it that it takes in. A rider
/// could otherwise break a derivation named after a healthy check, or one
/// main fails, and have the red excused as that check's.
fn stands_for(check: &str, failed: &BTreeSet<String>, asks: &Asks) -> Result<(), String> {
    let named = (asks.named)(check)
        .ok_or_else(|| format!("{check}'s derivation at the candidate couldn't be evaluated"))?;
    let part = |path: &str| {
        let name = path
            .strip_prefix("/nix/store/")
            .and_then(|rest| rest.split_once('-'))
            .and_then(|(_, name)| name.strip_suffix(".drv"))
            .unwrap_or_default();
        part_of(name) == Some(check) && named.inputs.contains(path)
    };
    match failed
        .iter()
        .find(|path| **path != named.own && !part(path))
    {
        Some(path) => Err(format!(
            "{path} failed under {check}'s name, but isn't its derivation at the candidate"
        )),
        None => Ok(()),
    }
}

/// Whether `name` is one nix gives a derivation: only its store-name
/// characters, never starting with a dot. A check's name goes into its
/// log's path, and a rider's evaluation can print any line, a `Cannot
/// build` naming `/home/u/notes` or `../x` among them, so a red naming
/// any other is none the train can read.
fn store_named(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"+._?=-".contains(&byte))
}

/// The check `name` is a part of, when it is named as a part is.
fn part_of(name: &str) -> Option<&str> {
    let (check, number) = name.rsplit_once(PART)?;
    (!number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())).then_some(check)
}

/// The name nix gives a derivation called `name` before its lines: up to
/// the first dash that a letter doesn't follow, the rest being what nix
/// reads as its version.
fn nix_name(name: &str) -> &str {
    name.char_indices()
        .find(|&(at, character)| {
            character == '-'
                && name[at + 1..]
                    .chars()
                    .next()
                    .is_some_and(|next| !next.is_ascii_alphabetic())
        })
        .map_or(name, |(at, _)| &name[..at])
}

/// `line` with each store path's hash masked, and each run of digits read
/// as one, but a lone 0, so two runs failing alike read alike and a run
/// that counted nothing doesn't read as one that counted some. A word
/// naming a test as pytest does, `path::name[params]`, keeps its digits,
/// so a test that failed at 390px doesn't read as one at 1440px (round
/// 4's F1).
fn masked(line: &str) -> String {
    unhashed(line)
        .split(' ')
        .map(|word| {
            if word.contains("::") {
                word.to_owned()
            } else {
                counted(word)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `line`, trimmed at its end, with each store path's hash masked.
fn unhashed(line: &str) -> String {
    const STORE: &str = "/nix/store/";
    const HASH: usize = 32;
    let mut unhashed = String::with_capacity(line.len());
    let mut rest = line.trim_end();
    while let Some(at) = rest.find(STORE) {
        let (before, after) = rest.split_at(at + STORE.len());
        unhashed.push_str(before);
        let hashed = after.len() >= HASH
            && after.as_bytes()[..HASH]
                .iter()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit());
        if hashed {
            unhashed.push('*');
            rest = &after[HASH..];
        } else {
            rest = after;
        }
    }
    unhashed.push_str(rest);
    unhashed
}

/// `word` with each run of digits read as one, but a lone 0.
fn counted(word: &str) -> String {
    let mut out = String::with_capacity(word.len());
    let mut characters = word.chars().peekable();
    while let Some(character) = characters.next() {
        if character.is_ascii_digit() {
            let mut zero = character == '0';
            while characters.next_if(char::is_ascii_digit).is_some() {
                zero = false;
            }
            out.push(if zero { '0' } else { '#' });
        } else {
            out.push(character);
        }
    }
    out
}

/// Whether `line`, one a check printed, is pytest's word on a subtest that
/// failed, or a line of why it failed: `FAILED path.py::name`, `ERROR
/// path.py::name` or `path.py::name FAILED`, and its `E` lines. A log
/// line such as `ERROR niri::backend::winit: ...` is none, so it can't
/// stand for the check's failure in place of what differed (round 4's
/// F2).
fn says_a_subtest_failed(line: &str) -> bool {
    let names_a_test = |word: &str| {
        word.split_once("::").is_some_and(|(path, name)| {
            // pytest names its files `.py` as they are, so `.PY` is
            // no test of its.
            Path::new(path).extension().is_some_and(|ext| ext == "py") && !name.is_empty()
        })
    };
    let mut words = line.split_whitespace();
    let first = words.next().unwrap_or_default();
    let second = words.next().unwrap_or_default();
    let verdict = |word: &str| matches!(word, "FAILED" | "ERROR");
    let named =
        (verdict(first) && names_a_test(second)) || (names_a_test(first) && verdict(second));
    named || line == "E" || line.starts_with("E ")
}

/// Why `check`'s failure in `words` can't be compared.
#[derive(Debug, PartialEq, Eq)]
enum Unsaid {
    /// It printed nothing.
    Nothing,
    /// It printed more than `MOST_LINES` lines.
    TooMuch(usize),
}

/// The lines `words` say `check`'s build printed, those nix prefixes with
/// its name or its parts', masked, blank ones left out; and when any is
/// pytest's word on a failed subtest, only those that are.
fn said_by(words: &str, check: &str) -> Result<Said, Unsaid> {
    let prefixes = [
        format!("{}> ", nix_name(check)),
        format!("{check}{}> ", PART.trim_end_matches('-')),
    ];
    let lines: Vec<String> = words
        .lines()
        .filter_map(|line| {
            prefixes
                .iter()
                .find_map(|prefix| line.strip_prefix(prefix.as_str()))
        })
        .map(masked)
        .filter(|line| !line.trim().is_empty())
        .collect();
    if lines.is_empty() {
        return Err(Unsaid::Nothing);
    }
    if lines.len() > MOST_LINES {
        return Err(Unsaid::TooMuch(lines.len()));
    }
    let subtests = lines.iter().any(|line| says_a_subtest_failed(line));
    let mut said = Said::new();
    for line in lines {
        if !subtests || says_a_subtest_failed(&line) {
            *said.entry(line).or_default() += 1;
        }
    }
    Ok(said)
}

/// Whether every one of `runs` printed what `said` holds, line for line.
fn alike(runs: &[Said], said: &Said) -> bool {
    !runs.is_empty() && runs.iter().all(|run| run == said)
}

/// The checks that excuse a red gate whose words are `words`, each with
/// why, as `asks` says of each; why not, when any isn't excused, or when
/// the red can't be excused at all.
pub(super) fn excused(words: &str, asks: &Asks) -> Result<Vec<(String, Excuse)>, String> {
    let failed = failures(words).ok_or("its words hold an error no failed build explains")?;
    let checks: Vec<String> = failed.keys().cloned().collect();
    if checks.is_empty() {
        return Err("no check failed of itself".to_owned());
    }
    if checks.len() > MOST {
        return Err(format!(
            "it broke {} checks, more than the {MOST} the train excuses",
            checks.len()
        ));
    }
    if let Some(test) = checks.iter().find(|check| check.starts_with(VM_TEST)) {
        return Err(format!(
            "{test} is a VM test, whose derivation isn't named as its check is"
        ));
    }
    if let Some(name) = checks.iter().find(|check| !attribute_named(check)) {
        return Err(format!("{name} isn't named as a check of the flake is"));
    }
    let mut said = Vec::with_capacity(checks.len());
    for check in &checks {
        match said_by(words, check) {
            Ok(lines) => said.push(lines),
            Err(Unsaid::Nothing) => {
                return Err(format!("{check}'s build printed nothing to compare"));
            }
            Err(Unsaid::TooMuch(count)) => {
                return Err(format!(
                    "{check}'s build printed {count} lines, more than the {MOST_LINES} compared"
                ));
            }
        }
    }
    let judged: Vec<Result<Excuse, String>> = std::thread::scope(|scope| {
        let asked: Vec<_> = checks
            .iter()
            .zip(&said)
            .zip(failed.values())
            .map(|((check, said), paths)| {
                scope.spawn(move || {
                    stands_for(check, paths, asks).and_then(|()| judge(check, said, asks))
                })
            })
            .collect();
        asked
            .into_iter()
            .map(|judging| {
                judging
                    .join()
                    .unwrap_or_else(|_| Err("asking of it failed".to_owned()))
            })
            .collect()
    });
    checks
        .into_iter()
        .zip(judged)
        .map(|(check, excuse)| excuse.map(|excuse| (check, excuse)))
        .collect()
}

/// Why `check`, which printed `said` at the candidate, says nothing of its
/// riders, as `asks` says: what the base already said, then a run again at
/// the candidate, then, when the base had said nothing, a run there.
fn judge(check: &str, said: &Said, asks: &Asks) -> Result<Excuse, String> {
    let known = (asks.known)(check);
    if matches!(&known, Some(AtBase::Fails(runs)) if alike(runs, said)) {
        return Ok(Excuse::MainsOwn);
    }
    // A pass at the base says the failure is the riders', flaky or not:
    // a check a rider made fail half the time would otherwise land half
    // the time on one rerun's pass.
    if known == Some(AtBase::Passes) {
        return Err(format!(
            "{check} passed at its base, so its failure is the riders'"
        ));
    }
    if (asks.again)(check) == Some(true) {
        return Ok(Excuse::PassedAgain);
    }
    let base = match known {
        Some(answer) => answer,
        None => (asks.asked)(check).ok_or_else(|| {
            format!("{check} didn't pass again at the candidate, and its base said nothing of it")
        })?,
    };
    match base {
        AtBase::Fails(runs) if alike(&runs, said) => Ok(Excuse::MainsOwn),
        AtBase::Fails(_) => Err(format!(
            "{check} didn't pass again at the candidate, and failed otherwise at its base"
        )),
        AtBase::Passes => Err(format!(
            "{check} didn't pass again at the candidate, and passed at its base"
        )),
    }
}

/// The folder what the train's `folder` keeps of `base` is in.
fn base_folder(folder: &Path, base: &str) -> PathBuf {
    folder.join("base").join(base)
}

/// The folder the train's `folder` keeps `candidate`'s runs again in.
fn again_folder(folder: &Path, candidate: &str) -> PathBuf {
    folder.join("again").join(candidate)
}

/// What the train's `folder` keeps of `check` at `base`.
fn kept(folder: &Path, base: &str, check: &str) -> Option<AtBase> {
    let path = base_folder(folder, base).join(format!("{check}.json"));
    let answer: Value = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    if answer["fails"] != true {
        return Some(AtBase::Passes);
    }
    let runs = answer["runs"]
        .as_array()?
        .iter()
        .map(|run| {
            run.as_object()?
                .iter()
                .map(|(line, times)| Some((line.clone(), usize::try_from(times.as_u64()?).ok()?)))
                .collect::<Option<Said>>()
        })
        .collect::<Option<Vec<Said>>>()?;
    Some(AtBase::Fails(runs))
}

/// Keep `answer` for `check` at `base` in the train's `folder`, a file a
/// check written whole, so checks answered at once never write over each
/// other and a reader never sees half of one.
fn keep(folder: &Path, base: &str, check: &str, answer: &AtBase) {
    let at = base_folder(folder, base);
    let written = match answer {
        AtBase::Fails(runs) => json!({ "fails": true, "runs": runs }),
        AtBase::Passes => json!({ "fails": false }),
    };
    let kept = fs::create_dir_all(&at)
        .map_err(|error| error.to_string())
        .and_then(|()| {
            crate::whole::write(&at.join(format!("{check}.json")), &format!("{written}\n"))
        });
    if let Err(error) = kept {
        eprintln!("merge-net: {check}'s run at {base} wasn't kept: {error}");
    }
}

/// Remove what the train's `folder` keeps of each base, and each
/// candidate's runs again, last written more than `KEPT_FOR` ago.
fn age_out(folder: &Path, now: SystemTime) {
    for kind in ["base", "again"] {
        let Ok(commits) = fs::read_dir(folder.join(kind)) else {
            continue;
        };
        for commit in commits.flatten() {
            let written = commit.metadata().and_then(|meta| meta.modified());
            if written.is_ok_and(|at| now.duration_since(at).is_ok_and(|age| age > KEPT_FOR)) {
                // One left is tried again on the next red, and holds
                // nothing a later run reads as its own.
                let _ = fs::remove_dir_all(commit.path());
            }
        }
    }
}

/// What a failed run whose words are `words` says of `check`: the lines it
/// printed, when they say the check itself failed and can be compared;
/// nothing when it failed for anything else.
fn failure_of(words: &str, check: &str) -> Option<Said> {
    if !broken(words)?.iter().any(|name| name == check) {
        return None;
    }
    said_by(words, check).ok()
}

/// What the watch's runs of `check` at `base` say: that it passed there,
/// once is enough, or that it failed there, the lines each failed run
/// printed; nothing when it ended no run there whose words say the check
/// itself failed, a run whose log is gone or says otherwise left out. A
/// run main's quarantine excused counts as passed for the watch, but
/// failed all the same, and is read as the failure it was: a check main
/// quarantines fails at the base as it fails in the red.
fn watched(state: &State, base: &str, check: &str) -> Option<AtBase> {
    let excused = state.excused_logs(base, check);
    let passed = state
        .runs(base, check)
        .iter()
        .filter(|ran| **ran == Ran::Passed)
        .count();
    if passed > excused.len() {
        return Some(AtBase::Passes);
    }
    let runs: Vec<Said> = state
        .failed_logs(base, check)
        .into_iter()
        .chain(excused)
        .filter_map(|log| {
            let words = crate::world::lines_of(&log).collect::<Vec<_>>().join("\n");
            failure_of(&words, check)
        })
        .collect();
    (!runs.is_empty()).then_some(AtBase::Fails(runs))
}

/// What `check` already did at `base`, running nothing: a pass in the
/// watch's runs in `state` or in what the train's `folder` keeps, or else
/// the failures of both. A Mac's comparison runs nowhere but its own net,
/// so it says nothing.
fn known(folder: &Path, state: &State, base: &str, check: &str) -> Option<AtBase> {
    if crate::macs::base_of(check).is_some() {
        return None;
    }
    let answers: Vec<AtBase> = [watched(state, base, check), kept(folder, base, check)]
        .into_iter()
        .flatten()
        .collect();
    if answers.contains(&AtBase::Passes) {
        return Some(AtBase::Passes);
    }
    let runs: Vec<Said> = answers
        .into_iter()
        .flat_map(|answer| match answer {
            AtBase::Fails(runs) => runs,
            AtBase::Passes => Vec::new(),
        })
        .collect();
    (!runs.is_empty()).then_some(AtBase::Fails(runs))
}

/// The arguments that build `check` of `flake` on a lent builder, its log
/// printed.
fn building(flake: &str, check: &str) -> Vec<String> {
    [
        "there",
        "--lent-only",
        "-L",
        flake,
        &format!("checks.{SYSTEM}.{check}"),
    ]
    .map(str::to_owned)
    .into()
}

/// A run's log at `log`, emptied first, so its words are this run's alone.
fn fresh(log: &Path) -> Option<()> {
    match fs::remove_file(log) {
        Ok(()) => Some(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(()),
        Err(_) => None,
    }
}

/// What a lent builder's run of `check` at `base` of the repository `slug`
/// says, `run` running it into a log under the train's `folder`: nothing
/// when no builder took it, it was let go, or it failed for anything but
/// the check itself.
fn ran(
    folder: &Path,
    slug: &str,
    base: &str,
    check: &str,
    run: &Runner<'_>,
    dropped: &dyn Fn() -> bool,
) -> Option<AtBase> {
    if crate::macs::base_of(check).is_some() {
        return None;
    }
    let log = base_folder(folder, base).join(format!("{check}.log"));
    fresh(&log)?;
    let code = run(
        &building(&format!("github:{slug}/{base}"), check),
        base,
        &log,
        dropped,
    )?;
    if code == 0 {
        return Some(AtBase::Passes);
    }
    if crate::watch::untaken(Some(code)) {
        return None;
    }
    let words = String::from_utf8_lossy(&fs::read(&log).ok()?).into_owned();
    failure_of(&words, check).map(|said| AtBase::Fails(vec![said]))
}

/// Whether `check` passed when run again alone at `candidate`, checked out
/// at `checkout`, `run` running it into a log under the train's `folder`:
/// nothing when no builder took it or it was let go.
fn again(
    folder: &Path,
    checkout: &Path,
    candidate: &str,
    check: &str,
    run: &Runner<'_>,
    dropped: &dyn Fn() -> bool,
) -> Option<bool> {
    let log = again_folder(folder, candidate).join(format!("{check}.log"));
    fresh(&log)?;
    let flake = checkout.display().to_string();
    let code = run(&building(&flake, check), candidate, &log, dropped)?;
    (!crate::watch::untaken(Some(code))).then_some(code == 0)
}

/// The arguments that print `check`'s derivation in the checkout `--run`
/// is asked from, at its commit, a line `own` and its inputs a line
/// `input` each, evaluated on a lent builder, the only kind `--run`
/// sends to, rather than here: an evaluation holds most of a GB of the
/// person's machine. `check` is
/// read as a plain attribute name already, so it quotes as it stands.
fn naming(check: &str) -> Vec<String> {
    let script = format!(
        "own=$(nix eval --raw '.#checks.{SYSTEM}.{check}.drvPath') && echo \"own $own\" && \
         nix-store --query --references \"$own\" | sed 's/^/input /'"
    );
    [
        "there",
        "--run",
        "--longest",
        NAMING_AT_MOST,
        "--",
        "sh",
        "-c",
        &script,
    ]
    .map(str::to_owned)
    .into()
}

/// The longest, in seconds, one check's derivation is given to evaluate.
const NAMING_AT_MOST: &str = "600";

/// What `words`, a naming run's, say `check`'s derivation is: none
/// unless one line names its own, a store derivation, and the rest name
/// its inputs.
fn named_in(words: &str) -> Option<Named> {
    let derivation =
        |path: &str| path.starts_with("/nix/store/") && path.ends_with(".drv") && store_path(path);
    let mut own = None;
    let mut inputs = BTreeSet::new();
    for line in words.lines() {
        if let Some(path) = line.strip_prefix("own ") {
            if own.is_some() || !derivation(path) {
                return None;
            }
            own = Some(path.to_owned());
        } else if let Some(path) = line.strip_prefix("input ") {
            inputs.insert(path.to_owned());
        }
    }
    Some(Named { own: own?, inputs })
}

/// Whether `path` is a store path with no more than a store name after
/// its hash.
fn store_path(path: &str) -> bool {
    path.strip_prefix("/nix/store/")
        .and_then(|rest| rest.split_once('-'))
        .is_some_and(|(hash, name)| {
            !hash.is_empty()
                && hash.bytes().all(|byte| byte.is_ascii_alphanumeric())
                && store_named(name)
        })
}

/// `check`'s derivation at `candidate`, as `run` evaluates it on a lent
/// builder from the train's checkout there; none when it couldn't.
fn named(
    folder: &Path,
    candidate: &str,
    check: &str,
    run: &Runner<'_>,
    dropped: &dyn Fn() -> bool,
) -> Option<Named> {
    let log = again_folder(folder, candidate).join(format!("{check}.named.log"));
    fresh(&log)?;
    let code = run(&naming(check), candidate, &log, dropped)?;
    if code != 0 {
        return None;
    }
    named_in(&crate::world::lines_of(&log).collect::<Vec<_>>().join("\n"))
}

/// Where the train's red is and runs: its net's state folder, the
/// repository, the checkout it was cut in, its base and its candidate.
pub(super) struct Red<'a> {
    /// The net's state folder.
    pub root: &'a Path,
    /// The repository on GitHub, as `owner/name`.
    pub slug: &'a str,
    /// The train's checkout, at the candidate.
    pub checkout: &'a Path,
    /// The commit the candidate was cut on.
    pub base: &'a str,
    /// The candidate.
    pub candidate: &'a str,
    /// The candidate's gate's log.
    pub log: &'a Path,
}

/// The checks a red candidate's riders didn't break, when its gate ran
/// whole and each fails alike at main or passes alone; nothing when its
/// gate didn't run whole. Why not is said in the net's log, as every
/// refusal is, and given back for the riders that fall, so a red that
/// holds where main's own failure would be excused reads why.
pub(super) fn excuse_of(red: &Red, whole: bool) -> Result<Vec<String>, String> {
    if !whole {
        let why = "nix-offload didn't say every part of its gate ran to its end";
        State::at(red.root.to_path_buf()).say(&format!(
            "the candidate {}'s red isn't excused at main's {}: {why}",
            short(red.candidate),
            short(red.base)
        ));
        return Err(why.to_owned());
    }
    excuse(red, &|args, commit, log, dropped| {
        crate::world::bounded(red.checkout, args, commit, log, dropped)
    })
}

/// The checks that excuse `red`, each failing at its base alike or passing
/// when run again at its candidate, `run` running each ask; said in the
/// net's log as the train lands it, and why not when they don't, said
/// there too.
pub(super) fn excuse(red: &Red, run: &Runner<'_>) -> Result<Vec<String>, String> {
    let state = State::at(red.root.to_path_buf());
    let train = super::folder(red.root);
    let off = train.join("no-excuse");
    let refused = |why: &str| {
        state.say(&format!(
            "the candidate {}'s red isn't excused at main's {}: {why}",
            short(red.candidate),
            short(red.base)
        ));
        Err(why.to_owned())
    };
    if off.exists() {
        return refused("train/no-excuse turns excuses off");
    }
    age_out(&train, SystemTime::now());
    let began = Instant::now();
    let dropped = || began.elapsed() > ASKS_AT_MOST || off.exists();
    let known = |check: &str| known(&train, &state, red.base, check);
    let again = |check: &str| again(&train, red.checkout, red.candidate, check, run, &dropped);
    let asked = |check: &str| {
        let answer = ran(&train, red.slug, red.base, check, run, &dropped)?;
        keep(&train, red.base, check, &answer);
        Some(answer)
    };
    let named = |check: &str| named(&train, red.candidate, check, run, &dropped);
    let asks = Asks {
        known: &known,
        again: &again,
        asked: &asked,
        named: &named,
    };
    let checks = match excused(&super::gated::words_of(red.log), &asks) {
        Ok(checks) => checks,
        Err(why) => return refused(&why),
    };
    if off.exists() {
        return refused("train/no-excuse turned excuses off while it asked");
    }
    let said: Vec<String> = checks
        .iter()
        .map(|(check, excuse)| match excuse {
            Excuse::MainsOwn => format!("{check}, which fails at main alike"),
            Excuse::PassedAgain => format!("{check}, which passed run again"),
        })
        .collect();
    state.say(&format!(
        "the candidate {} failed changed-checks at main's {} only on {}, so the train lands it",
        short(red.candidate),
        short(red.base),
        said.join("; ")
    ));
    Ok(checks.into_iter().map(|(check, _)| check).collect())
}

#[cfg(test)]
mod tests;
