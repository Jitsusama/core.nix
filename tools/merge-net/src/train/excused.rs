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
//! The base's word comes first from the watch's runs there and what the
//! train keeps, a pass in any of them meaning the check isn't main's own.
//! The check runs again at the candidate next. Only when the base has
//! said nothing is one lent run there asked, kept in
//! `train/base/<base>/<check>.json` for the next candidate on that base.
//! Each run stops `ASKS_AT_MOST` after the first began, or once
//! `train/no-excuse` appears. A red the gate's words don't wholly explain
//! by checks that failed to build of themselves, one breaking more than
//! `MOST` checks, or one breaking a VM test, whose derivation isn't named
//! as its check is, is never excused. A file `train/no-excuse` turns
//! every excuse off.

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
/// exited, none when it didn't end of itself.
pub(super) type Runner = dyn Fn(&[String], &str, &Path, &dyn Fn() -> bool) -> Option<i32> + Sync;

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
}

/// The checks a red gate's `words` say failed of themselves, each by its
/// check's name, a part read as the check it is a part of, leaving out
/// those that failed only for a dependency; none when its words hold an
/// error no failed build explains, or a `Cannot build` for any other
/// reason, so that nothing they say can be excused.
pub(super) fn broken(words: &str) -> Option<Vec<String>> {
    let lines: Vec<&str> = words.lines().map(str::trim).collect();
    let mut failed = BTreeSet::new();
    let mut following = BTreeSet::new();
    for (at, line) in lines.iter().enumerate() {
        if let Some(rest) = line.strip_prefix(CANNOT_BUILD) {
            let (path, _) = rest.split_once(".drv'")?;
            let (_, name) = path.split_once('-')?;
            let reason = lines.get(at + 1).copied().unwrap_or_default();
            if reason.starts_with(BUILDER_FAILED) {
                failed.insert(name);
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
    let checks: BTreeSet<String> = failed
        .into_iter()
        .map(|name| {
            part_of(name)
                .filter(|check| following.contains(check))
                .unwrap_or(name)
                .to_owned()
        })
        .collect();
    Some(checks.into_iter().collect())
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
/// that counted nothing doesn't read as one that counted some.
fn masked(line: &str) -> String {
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
    let mut out = String::with_capacity(unhashed.len());
    let mut characters = unhashed.chars().peekable();
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
/// failed, or a line of why it failed.
fn says_a_subtest_failed(line: &str) -> bool {
    let named = line.contains("::")
        && (line.starts_with("FAILED ")
            || line.starts_with("ERROR ")
            || line.contains(" FAILED")
            || line.contains(" ERROR"));
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
    let checks = broken(words).ok_or("its words hold an error no failed build explains")?;
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
            .map(|(check, said)| scope.spawn(move || judge(check, said, asks)))
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
/// itself failed, a run whose log is gone or says otherwise left out.
fn watched(state: &State, base: &str, check: &str) -> Option<AtBase> {
    if state.runs(base, check).contains(&Ran::Passed) {
        return Some(AtBase::Passes);
    }
    let runs: Vec<Said> = state
        .failed_logs(base, check)
        .into_iter()
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
    run: &Runner,
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
    run: &Runner,
    dropped: &dyn Fn() -> bool,
) -> Option<bool> {
    let log = again_folder(folder, candidate).join(format!("{check}.log"));
    fresh(&log)?;
    let flake = checkout.display().to_string();
    let code = run(&building(&flake, check), candidate, &log, dropped)?;
    (!crate::watch::untaken(Some(code))).then_some(code == 0)
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

/// The checks that excuse `red`, each failing at its base alike or passing
/// when run again at its candidate, `run` running each ask; said in the
/// net's log as the train lands it, and none when they don't, with why
/// said there too.
pub(super) fn excuse(red: &Red, run: &Runner) -> Option<Vec<String>> {
    let state = State::at(red.root.to_path_buf());
    let train = super::folder(red.root);
    let off = train.join("no-excuse");
    let refused = |why: &str| {
        state.say(&format!(
            "the candidate {}'s red isn't excused at main's {}: {why}",
            short(red.candidate),
            short(red.base)
        ));
        None
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
    let asks = Asks {
        known: &known,
        again: &again,
        asked: &asked,
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
    Some(checks.into_iter().map(|(check, _)| check).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fmt::Write as _;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    /// The words of a gate in which each of `failed` failed to build,
    /// printing the lines named, and each of `following` failed only for
    /// one of them.
    fn red(failed: &[(&str, &[&str])], following: &[&str]) -> String {
        let mut words = String::from("changed-checks: evaluates 3 of x86_64-linux's 9 checks\n");
        for (check, printed) in failed {
            for line in *printed {
                let _ = writeln!(words, "{}> {line}", nix_name(check));
            }
            let _ = writeln!(words, "{}", cannot_build(check));
        }
        for check in following {
            let _ = writeln!(
                words,
                "error: Cannot build '/nix/store/j5fh-{check}.drv'.\n       Reason: 1 \
                 dependency failed."
            );
        }
        words.push_str("error: Build failed due to failed dependency\n");
        words.push_str("error: build of '/nix/store/0x1c-x.drv^*' failed\n");
        words.push_str("\u{2715} a check the change touched failed\n");
        words
    }

    /// What nix says of `check` failing to build of itself.
    fn cannot_build(check: &str) -> String {
        format!(
            "error: Cannot build '/nix/store/0x1c-{check}.drv'.\n       Reason: builder failed \
             with exit code 1.\n       Output paths:"
        )
    }

    /// `lines`, each printed once, as a check's failure says them.
    fn said(lines: &[&str]) -> Said {
        let mut said = Said::new();
        for &line in lines {
            *said.entry(line.to_owned()).or_default() += 1;
        }
        said
    }

    /// A base that fails every check, printing `lines` once.
    fn fails(lines: &[&str]) -> AtBase {
        AtBase::Fails(vec![said(lines)])
    }

    /// Asks of which each answers as its closure says.
    fn asks<'a>(
        known: &'a (dyn Fn(&str) -> Option<AtBase> + Sync),
        again: &'a (dyn Fn(&str) -> Option<bool> + Sync),
        asked: &'a (dyn Fn(&str) -> Option<AtBase> + Sync),
    ) -> Asks<'a> {
        Asks {
            known,
            again,
            asked,
        }
    }

    fn nothing(_: &str) -> Option<AtBase> {
        None
    }

    fn unsaid(_: &str) -> Option<bool> {
        None
    }

    fn failed_again(_: &str) -> Option<bool> {
        Some(false)
    }

    fn passed_again(_: &str) -> Option<bool> {
        Some(true)
    }

    /// A run that must not be made.
    fn never(_: &[String], _: &str, _: &Path, _: &dyn Fn() -> bool) -> Option<i32> {
        panic!("a run was asked for when nothing should have run");
    }

    #[test]
    fn a_red_whose_checks_print_what_the_base_printed_is_main_s_own() {
        let words = red(
            &[
                ("cat-on-niri", &["counted 12 frames of 30"]),
                ("launcher", &["starting niri", "no window in 9 s"]),
            ],
            &["all"],
        );
        let known = |check: &str| {
            Some(if check == "cat-on-niri" {
                fails(&["counted # frames of #"])
            } else {
                fails(&["starting niri", "no window in # s"])
            })
        };
        assert_eq!(
            excused(&words, &asks(&known, &unsaid, &nothing)),
            Ok(vec![
                ("cat-on-niri".to_owned(), Excuse::MainsOwn),
                ("launcher".to_owned(), Excuse::MainsOwn),
            ]),
            "a red failing at main's own commit alike held its riders, or asked of a check \
             that failed only for another"
        );
    }

    /// A count of none is another failure than a count of some, and a
    /// failure printing fewer lines than main's, or more, isn't main's.
    #[test]
    fn a_failure_unlike_the_base_s_isn_t_main_s_own() {
        let cases: [(&[&str], &[&str]); 4] = [
            (&["counted 0 frames of 30"], &["counted 12 frames of 30"]),
            (&["a.nix runs 900 lines", "b.nix runs 812 lines"], &["a.nix runs 900 lines"]),
            (&["a.nix runs 900 lines"], &["a.nix runs 900 lines", "b.nix runs 812 lines"]),
            (&["x", "x"], &["x"]),
        ];
        for (candidate, base) in cases {
            let words = red(&[("cat", candidate)], &[]);
            let base_words = red(&[("cat", base)], &[]);
            let at_base = failure_of(&base_words, "cat").unwrap();
            let known = move |_: &str| Some(AtBase::Fails(vec![at_base.clone()]));
            assert!(
                excused(&words, &asks(&known, &failed_again, &nothing)).is_err(),
                "{candidate:?} was excused by main's {base:?}"
            );
        }
    }

    /// A base that failed differently in two runs says no one failure is
    /// main's.
    #[test]
    fn every_run_at_the_base_must_fail_alike() {
        let words = red(&[("cat", &["x"])], &[]);
        let known = |_: &str| Some(AtBase::Fails(vec![said(&["x"]), said(&["y"])]));
        assert!(
            excused(&words, &asks(&known, &failed_again, &nothing)).is_err(),
            "a failure one base run printed and another didn't was main's own"
        );
    }

    #[test]
    fn a_check_that_passes_run_again_at_the_candidate_is_a_flake() {
        let words = red(&[("cat-on-niri", &["drew 4 frames"])], &[]);
        for known in [None, Some(AtBase::Passes), Some(fails(&["other"]))] {
            let base = move |_: &str| known.clone();
            assert_eq!(
                excused(&words, &asks(&base, &passed_again, &nothing)),
                Ok(vec![("cat-on-niri".to_owned(), Excuse::PassedAgain)]),
                "a check that passed run again at the candidate held its riders"
            );
        }
    }

    #[test]
    fn a_check_that_fails_again_and_passes_at_the_base_holds_the_red() {
        let words = red(&[("cat", &["x"]), ("long-files", &["x"])], &[]);
        let known = |check: &str| {
            Some(if check == "long-files" {
                AtBase::Passes
            } else {
                fails(&["x"])
            })
        };
        assert!(
            excused(&words, &asks(&known, &failed_again, &nothing)).is_err(),
            "a check the riders broke was excused"
        );
    }

    /// Only a base that said nothing is run, and it is asked only once
    /// the check failed again at the candidate.
    #[test]
    fn a_base_that_said_nothing_is_run_once_the_check_fails_again() {
        let words = red(&[("cat", &["x"])], &[]);
        let ran = AtomicUsize::new(0);
        let asked = |_: &str| {
            ran.fetch_add(1, Ordering::SeqCst);
            Some(fails(&["x"]))
        };
        assert_eq!(
            excused(&words, &asks(&nothing, &failed_again, &asked)),
            Ok(vec![("cat".to_owned(), Excuse::MainsOwn)]),
            "a lent run at the base failing alike didn't excuse the check"
        );
        assert_eq!(ran.load(Ordering::SeqCst), 1, "the base wasn't run once");
        let unasked = |_: &str| -> Option<AtBase> {
            panic!("the base was run when it had already answered");
        };
        let known = |_: &str| Some(AtBase::Passes);
        assert!(
            excused(&words, &asks(&known, &failed_again, &unasked)).is_err(),
            "a check that passed at the base was excused"
        );
        assert!(
            excused(&words, &asks(&nothing, &unsaid, &nothing)).is_err(),
            "a check no run spoke for was excused"
        );
    }

    /// A pytest file prints its daemons' noise beside its verdict, which
    /// differs from run to run: only its failed subtests and their `E`
    /// lines are compared.
    #[test]
    fn a_pytest_failure_is_read_by_its_failed_subtests() {
        let printed = |noise: &str, subtest: &str| {
            red(
                &[(
                    "cat-on-niri",
                    &[
                        noise,
                        &format!("t.py::{subtest} FAILED [ 14%]"),
                        "E       AssertionError: drew 5 frames",
                        "E       assert 5 <= 1",
                        &format!("FAILED t.py::{subtest} - AssertionError: drew 5"),
                    ],
                )],
                &[],
            )
        };
        let base = failure_of(&printed("WARN broken pipe", "test_a_walk"), "cat-on-niri");
        let base = base.unwrap();
        let known = move |_: &str| Some(AtBase::Fails(vec![base.clone()]));
        assert_eq!(
            excused(
                &printed("WARN no cursor", "test_a_walk"),
                &asks(&known, &failed_again, &nothing)
            ),
            Ok(vec![("cat-on-niri".to_owned(), Excuse::MainsOwn)]),
            "a daemon's noise kept a pytest failure from matching main's"
        );
        assert!(
            excused(
                &printed("WARN no cursor", "test_a_hold"),
                &asks(&known, &failed_again, &nothing)
            )
            .is_err(),
            "a subtest main passes was excused"
        );
    }

    /// A check split in parts fails as a part, which nix names before its
    /// lines without the part's number, and the watch runs as the check.
    #[test]
    fn a_part_that_failed_is_read_as_its_check() {
        let words = format!(
            "launcher-part> t.py::test_row FAILED\n{}\n{}\nerror: Cannot build \
             '/nix/store/j5fh-launcher.drv'.\n       Reason: 1 dependency failed.\n",
            cannot_build("launcher-part-6"),
            "error: Build failed due to failed dependency"
        );
        let asked = std::sync::Mutex::new(Vec::new());
        let again = |check: &str| {
            asked.lock().unwrap().push(check.to_owned());
            Some(true)
        };
        assert_eq!(
            excused(&words, &asks(&nothing, &again, &nothing)),
            Ok(vec![("launcher".to_owned(), Excuse::PassedAgain)]),
            "a part's failure wasn't read as its check's"
        );
        assert_eq!(
            asked.into_inner().unwrap(),
            vec!["launcher".to_owned()],
            "the part, not its check, was run again"
        );
    }

    /// Each asks nothing: an evaluation's error, a build nix gave up on
    /// for a reason the excuse can't read, a VM test, more checks than
    /// `MOST`, a build that printed nothing, and one that printed more
    /// than `MOST_LINES`.
    #[test]
    fn a_red_the_excuse_cant_read_asks_nothing() {
        let failed: Vec<(&str, &[&str])> = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|&check| (check, &["x"] as &[&str]))
            .collect();
        let long: Vec<String> = (0..=MOST_LINES).map(|at| format!("line {at}")).collect();
        let long: Vec<&str> = long.iter().map(String::as_str).collect();
        let reds = [
            format!(
                "{}error: attribute 'x' missing\n",
                red(&[("cat", &["x"])], &[])
            ),
            "cat> x\nerror: Cannot build '/nix/store/0x1c-cat.drv'.\n       Reason: timed out \
             after 3600 seconds of silence\n"
                .to_owned(),
            red(&[("vm-test-run-desktop-looks", &["x"])], &[]),
            red(&failed, &[]),
            red(&[("cat", &["x"]), ("quiet", &[])], &[]),
            red(&[("cat", &long)], &[]),
        ];
        for words in reds {
            let asked = AtomicBool::new(false);
            let ask = |_: &str| {
                asked.store(true, Ordering::SeqCst);
                Some(fails(&["x"]))
            };
            let again = |_: &str| {
                asked.store(true, Ordering::SeqCst);
                Some(true)
            };
            let excused = excused(&words, &asks(&ask, &again, &ask));
            assert!(excused.is_err(), "a red the excuse can't read was excused");
            assert!(
                !asked.load(Ordering::SeqCst),
                "a red that can't be excused asked of a run: {}",
                excused.unwrap_err()
            );
        }
    }

    #[test]
    fn a_line_is_read_with_its_numbers_and_store_hashes_masked() {
        assert_eq!(
            masked("took 12.5 s in /nix/store/0123456789abcdfghijklmnpqrsvwxyz-niri/bin  "),
            "took #.# s in /nix/store/*-niri/bin",
            "a count or a hash that differs between runs failing alike kept them apart"
        );
        assert_eq!(
            masked("counted 0 of 10, 0.25 s, 007"),
            "counted 0 of #, 0.# s, #",
            "a lone 0 read as any number, or a number holding a 0 read as none"
        );
        assert_eq!(
            masked("/nix/store/short-x"),
            "/nix/store/short-x",
            "a store path with no hash lost its name"
        );
    }

    #[test]
    fn nix_names_a_derivation_without_what_it_reads_as_a_version() {
        for (name, named) in [
            ("launcher-on-niri-part-6", "launcher-on-niri-part"),
            ("the-margin-cat-follows-the-keys-on-niri", "the-margin-cat-follows-the-keys-on-niri"),
            ("niri-26.04", "niri"),
        ] {
            assert_eq!(nix_name(name), named, "nix's name for {name} was misread");
        }
    }

    #[test]
    fn answers_kept_at_once_for_a_base_are_all_read_back() {
        let folder = super::super::tests::root("excused-kept");
        let state = State::at(folder.join("state"));
        std::thread::scope(|scope| {
            scope.spawn(|| keep(&folder, "b1", "cat", &fails(&["x", "x", "y"])));
            scope.spawn(|| keep(&folder, "b1", "long-files", &AtBase::Passes));
        });
        assert_eq!(
            known(&folder, &state, "b1", "cat"),
            Some(fails(&["x", "x", "y"])),
            "a kept failure wasn't read back, or lost a line or its count"
        );
        assert_eq!(
            known(&folder, &state, "b1", "long-files"),
            Some(AtBase::Passes),
            "a pass kept at the same time as a failure wasn't read back"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn what_the_train_keeps_goes_once_main_is_long_past_it() {
        let folder = super::super::tests::root("excused-aged");
        keep(&folder, "b1", "cat", &AtBase::Passes);
        fs::create_dir_all(again_folder(&folder, "c1")).unwrap();
        age_out(&folder, SystemTime::now());
        assert!(
            kept(&folder, "b1", "cat").is_some() && again_folder(&folder, "c1").exists(),
            "a fresh answer was aged out"
        );
        age_out(
            &folder,
            SystemTime::now() + KEPT_FOR + Duration::from_secs(60),
        );
        assert!(
            kept(&folder, "b1", "cat").is_none(),
            "an answer for a base three days gone was kept"
        );
        assert!(
            !again_folder(&folder, "c1").exists(),
            "a candidate's runs again from three days ago were kept"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    /// A watch at `folder` that ran `check` at `b1` once each as `runs`
    /// says, a failed run's log holding its words, or gone when it is none.
    fn watch_of(folder: &Path, runs: &[(Ran, Option<&str>)]) -> State {
        let state = State::at(folder.join("state"));
        state.make().unwrap();
        for (at, (ran, words)) in runs.iter().enumerate() {
            let log = folder.join(format!("cat-{at}.log"));
            if let Some(words) = words {
                fs::write(&log, words).unwrap();
            }
            state.record("b1", "cat", *ran, Some(1), &log, 60).unwrap();
        }
        state
    }

    /// A failed run whose words don't say the check itself failed, as a
    /// dependency's failure, or whose log is gone, is left out rather
    /// than losing the runs beside it.
    #[test]
    fn the_watch_s_failed_runs_at_the_base_are_each_read() {
        let folder = super::super::tests::root("excused-watched");
        let first = format!("cat> counted 3 frames\ncat> slow\n{}\n", cannot_build("cat"));
        let followed = "error: Cannot build '/nix/store/0x1c-cat.drv'.\n       Reason: 1 \
                        dependency failed.\n";
        let state = watch_of(
            &folder,
            &[(Ran::Failed, Some(&first)), (Ran::Failed, Some(followed)), (Ran::Failed, None)],
        );
        assert_eq!(
            known(&folder, &state, "b1", "cat"),
            Some(fails(&["counted # frames", "slow"])),
            "a failed run beside one that said nothing of the check wasn't read"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    /// A pass anywhere it is known, the watch's or a lent run's, says the
    /// check isn't main's own, whatever failed beside it.
    #[test]
    fn a_pass_the_base_ever_had_wins() {
        let folder = super::super::tests::root("excused-passes");
        let failed = format!("cat> x\n{}\n", cannot_build("cat"));
        let state = watch_of(&folder, &[(Ran::Failed, Some(&failed)), (Ran::Passed, None)]);
        assert_eq!(
            known(&folder, &state, "b1", "cat"),
            Some(AtBase::Passes),
            "a check the watch saw pass at the base read as failing there"
        );
        let _ = fs::remove_dir_all(&folder);
        let state = watch_of(&folder, &[(Ran::Failed, Some(&failed))]);
        keep(&folder, "b1", "cat", &AtBase::Passes);
        assert_eq!(
            known(&folder, &state, "b1", "cat"),
            Some(AtBase::Passes),
            "a kept pass lost to the watch's failure"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_mac_s_comparison_asks_nothing() {
        let folder = super::super::tests::root("excused-mac");
        let state = State::at(folder.join("state"));
        let check = crate::macs::check("b1");
        keep(&folder, "b1", &check, &AtBase::Passes);
        assert_eq!(known(&folder, &state, "b1", &check), None, "a Mac's check was read");
        assert_eq!(
            ran(&folder, "o/r", "b1", &check, &never, &|| false),
            None,
            "a Mac's check was run"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    /// A runner that appends `words` to its log, as nix-offload's file
    /// is opened, and exits `code`, after checking it was asked to build
    /// `wanted` at `commit`.
    fn appending<'a>(
        wanted: &'a [String],
        commit: &'a str,
        words: &'a str,
        code: Option<i32>,
    ) -> impl Fn(&[String], &str, &Path, &dyn Fn() -> bool) -> Option<i32> + Sync + 'a {
        move |args, at, log, _| {
            assert_eq!(args, wanted, "the run was asked for something else");
            assert_eq!(at, commit, "the run was asked at another commit");
            fs::create_dir_all(log.parent().unwrap()).unwrap();
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log)
                .unwrap();
            std::io::Write::write_all(&mut file, words.as_bytes()).unwrap();
            code
        }
    }

    #[test]
    fn a_lent_run_at_the_base_reads_its_own_words_alone() {
        let folder = super::super::tests::root("excused-lent");
        let wanted: Vec<String> = ["there", "--lent-only", "-L", "github:o/r/b1"]
            .into_iter()
            .map(str::to_owned)
            .chain([format!("checks.{SYSTEM}.cat")])
            .collect();
        let failing = format!("cat> counted 4 frames\n{}\n", cannot_build("cat"));
        let cases: [(&str, Option<i32>, Option<AtBase>); 4] = [
            ("", Some(0), Some(AtBase::Passes)),
            (&failing, Some(1), Some(fails(&["counted # frames"]))),
            ("", Some(1), None),
            ("", Some(75), None),
        ];
        for (words, code, answer) in cases {
            let run = appending(&wanted, "b1", words, code);
            assert_eq!(
                ran(&folder, "o/r", "b1", "cat", &run, &|| false),
                answer,
                "a lent run that exited {code:?} was misread, or read an earlier run's words"
            );
        }
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_check_runs_again_from_the_candidate_s_checkout() {
        let folder = super::super::tests::root("excused-again");
        let checkout = folder.join("checkout");
        let wanted: Vec<String> = ["there", "--lent-only", "-L"]
            .into_iter()
            .map(str::to_owned)
            .chain([checkout.display().to_string(), format!("checks.{SYSTEM}.cat")])
            .collect();
        for (code, answer) in [
            (Some(0), Some(true)),
            (Some(1), Some(false)),
            (Some(100), Some(false)),
            (Some(75), None),
            (Some(69), None),
            (None, None),
        ] {
            let run = appending(&wanted, "c1", "", code);
            assert_eq!(
                again(&folder, &checkout, "c1", "cat", &run, &|| false),
                answer,
                "a run again that exited {code:?} was misread"
            );
        }
        let _ = fs::remove_dir_all(&folder);
    }

    /// A net whose base would excuse the red, given the candidate's log,
    /// its train folder and the red to excuse.
    fn excusable(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = super::super::tests::root(name);
        let train = super::super::folder(&root);
        keep(&train, "b1", "cat", &fails(&["x"]));
        let log = root.join("candidate.log");
        fs::write(&log, red(&[("cat", &["x"])], &[])).unwrap();
        (root, train, log)
    }

    #[test]
    fn a_net_told_no_excuse_excuses_nothing() {
        let (root, train, log) = excusable("excused-off");
        let red = Red {
            root: &root,
            slug: "o/r",
            checkout: &root,
            base: "b1",
            candidate: "c1",
            log: &log,
        };
        assert_eq!(
            excuse(&red, &never),
            Some(vec!["cat".to_owned()]),
            "a red its base excuses wasn't excused"
        );
        fs::write(train.join("no-excuse"), "").unwrap();
        assert_eq!(excuse(&red, &never), None, "a red was excused with excuses turned off");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn excuses_turned_off_while_it_asks_let_its_runs_go() {
        let (root, train, log) = excusable("excused-turned-off");
        fs::write(&log, red(&[("cat", &["y"])], &[])).unwrap();
        let run = |_: &[String], _: &str, _: &Path, dropped: &dyn Fn() -> bool| {
            assert!(!dropped(), "a run was let go before excuses were turned off");
            fs::write(train.join("no-excuse"), "").unwrap();
            assert!(dropped(), "a run wasn't let go once excuses were turned off");
            Some(0)
        };
        let red = Red {
            root: &root,
            slug: "o/r",
            checkout: &root,
            base: "b1",
            candidate: "c1",
            log: &log,
        };
        assert_eq!(
            excuse(&red, &run),
            None,
            "a red was excused after excuses were turned off"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
