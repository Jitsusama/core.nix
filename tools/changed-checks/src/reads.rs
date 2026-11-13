//! What a base's VM tests read of the repository as they were evaluated,
//! kept beside its derivations, so a gate whose change reaches none of it
//! gives each VM test the base's derivation rather than evaluating them.
//! On 12 November the VM tests took half of each gate's evaluating, and 95
//! of 152 gates that named anything named no VM test.
//!
//! Nix reads a git flake through git's object store, which says nothing of
//! the files it read, so a base's keep evaluates its VM tests from the
//! same source as a plain path (`path:` and its store path), under
//! strace, rather than from git. That names each file opened or looked
//! up and each folder listed: an import, `readFile`, `readDir`,
//! `pathExists`, a fileset, a folder as a source and a path in a string
//! each show as one of those (`reads/traced.txt` holds each, traced on
//! the lent builder, each with an empty cache, since a copy Nix finds in
//! its cache it reads nothing for). A file a change touches reaches the
//! VM tests when it was read, when looking it or a folder above it up
//! found nothing, or when it was
//! added or removed and a folder listed above it gained or lost an entry
//! (itself, or the folder it sits in under the listed one), since a
//! listing gives names alone; one removed, when any folder above it but
//! the root was looked at, since the folder may go with it. `flake.nix`
//! and `flake.lock` always reach them. A trace line under the source that
//! can't be put to a path, a flake input that's a local path, or a record
//! made from another tree leaves the VM tests evaluated as before.
//!
//! On 12 November the VM tests' 99 derivations came out the same from the
//! plain path as from git, and tracing took 15% longer than evaluating
//! from git, side by side on one builder. Were one to differ, each gate
//! on the base would build that test, since the working tree's comes from
//! git: work again, never a test passed over. A base a gate kept first,
//! from git alone, has its VM tests traced once more as it's kept, and its
//! record kept only when each comes to the derivation git gave.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::checks::{self, Derivations};
use crate::evaluating;
use crate::runs::{Evaluation, Run};

/// What evaluating the VM tests read, each by its path from the
/// repository's root.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Reads {
    /// Files and folders opened, looked up or followed, which were there.
    read: BTreeSet<String>,
    /// Paths looked up and not found.
    missing: BTreeSet<String>,
    /// Folders whose entries were listed.
    listed: BTreeSet<String>,
}

/// The calls strace is asked to trace: each that opens, looks up, follows
/// or lists a path, and each that moves where a relative path starts.
const TRACED: &str = "trace=openat,open,newfstatat,stat,lstat,statx,access,faccessat,\
                      faccessat2,readlink,readlinkat,getdents64,chdir,fchdir";

/// Where strace is named, when it's run: a stand-in's, in the tests.
const STRACE: &str = "CHANGED_CHECKS_STRACE";

/// strace, as the package names it, or the one on the path.
const STRACE_BUILT: &str = match option_env!("CHANGED_CHECKS_STRACE") {
    Some(strace) => strace,
    None => "strace",
};

/// The files every evaluation of a flake reads, whatever its checks read.
const EVERY: [&str; 2] = ["flake.nix", "flake.lock"];

/// How a change touched a file, as `git diff --name-status` says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Touched {
    Added,
    Changed,
    Removed,
}

impl Reads {
    /// What `trace`, strace's with `-y`, says was read under `source`, the
    /// flake's store path; nothing when a line naming it can't be put to a
    /// path.
    pub fn traced(trace: &str, source: &str) -> Option<Self> {
        let mut reads = Self::default();
        for line in trace.lines().filter(|line| line.contains(source)) {
            reads.take(line, source)?;
        }
        Some(reads)
    }

    /// Take what one line of the trace read, or say it can't be read.
    fn take(&mut self, line: &str, source: &str) -> Option<()> {
        // With -f each line opens with its process's number.
        let line = line
            .split_once(' ')
            .filter(|(pid, _)| pid.chars().all(|c| c.is_ascii_digit()))
            .map_or(line, |(_, rest)| rest);
        // The second half of a call strace split in two names no path; its
        // first half was taken as both read and missing.
        if line.starts_with("<...") {
            return Some(());
        }
        let (call, rest) = line.split_once('(')?;
        let first = rest.split(", ").next()?;
        match call {
            "getdents64" => {
                if let Some(folder) = under(source, annotated(first)?) {
                    self.listed.insert(folder);
                }
                Some(())
            }
            "chdir" | "fchdir" => None,
            _ => {
                let quoted = quoted(rest)?;
                let path = if quoted.starts_with('/') {
                    quoted.to_owned()
                } else {
                    // A relative path starts at the folder its first
                    // argument names, which only the *at calls give.
                    let folder = annotated(first).filter(|_| call.ends_with("at"))?;
                    format!("{folder}/{quoted}")
                };
                let Some(path) = under(source, &normal(&path)?) else {
                    return Some(());
                };
                let outcome = rest.rsplit_once(") = ").map(|(_, outcome)| outcome);
                match outcome {
                    None => {
                        self.read.insert(path.clone());
                        self.missing.insert(path);
                    }
                    Some(outcome) if outcome.starts_with("-1 ") => {
                        if outcome.contains("ENOENT") || outcome.contains("ENOTDIR") {
                            self.missing.insert(path);
                        } else {
                            self.read.insert(path);
                        }
                    }
                    Some(_) => {
                        if rest.contains("O_DIRECTORY") {
                            self.listed.insert(path.clone());
                        }
                        self.read.insert(path);
                    }
                }
                Some(())
            }
        }
    }

    /// Add what another evaluation read.
    fn join(&mut self, other: Self) {
        self.read.extend(other.read);
        self.missing.extend(other.missing);
        self.listed.extend(other.listed);
    }

    /// Whether a change to `file`, touched so, could move what was read;
    /// `came_or_went` says whether a folder above it came or went with it.
    pub fn reached_by(
        &self,
        file: &str,
        touched: Touched,
        came_or_went: &dyn Fn(&str) -> bool,
    ) -> bool {
        let above = folders_above(file);
        EVERY.contains(&file)
            || self.read.contains(file)
            || self.missing.contains(file)
            || above.iter().any(|folder| self.missing.contains(*folder))
            || (touched != Touched::Changed && self.entry_moved(&above, came_or_went))
            // The root, looked at by every evaluation, never goes.
            || (touched == Touched::Removed
                && above[1..].iter().any(|folder| self.read.contains(*folder)))
    }

    /// Whether a folder listed above a file added or removed had an entry
    /// come or go with it: the file itself, or the folder it sits in just
    /// under the listed one. A listing gives names and kinds alone, so a
    /// file changed in place moves none; a folder copied whole has each
    /// file in it read besides. The root and modules/ are listed, so
    /// otherwise any change would reach the VM tests.
    fn entry_moved(&self, above: &[&str], came_or_went: &dyn Fn(&str) -> bool) -> bool {
        above.iter().enumerate().any(|(at, folder)| {
            self.listed.contains(*folder)
                && above.get(at + 1).is_none_or(|entry| came_or_went(entry))
        })
    }

    /// The record as kept, for the tree it was made from.
    fn kept(&self, tree: &str) -> Value {
        json!({
            "tree": tree,
            "read": self.read,
            "missing": self.missing,
            "listed": self.listed,
        })
    }

    /// A record as kept, when it was made from `tree`.
    fn from_kept(text: &str, tree: &str) -> Option<Self> {
        let kept: Value = serde_json::from_str(text).ok()?;
        if kept.get("tree")?.as_str()? != tree {
            return None;
        }
        let paths = |key: &str| -> Option<BTreeSet<String>> {
            kept.get(key)?
                .as_array()?
                .iter()
                .map(|path| path.as_str().map(str::to_owned))
                .collect()
        };
        Some(Self {
            read: paths("read")?,
            missing: paths("missing")?,
            listed: paths("listed")?,
        })
    }
}

/// The folders above a path from the root, the root itself first.
fn folders_above(file: &str) -> Vec<&str> {
    let mut folders = vec![""];
    folders.extend(file.match_indices('/').map(|(at, _)| &file[..at]));
    folders
}

/// The path strace's `-y` gives after a descriptor, `7</nix/store/x>`.
fn annotated(argument: &str) -> Option<&str> {
    argument.split_once('<')?.1.strip_suffix('>')
}

/// The first quoted string of a call's arguments, when strace gave it
/// plainly: one with an escape in it can't be read as a path.
fn quoted(arguments: &str) -> Option<&str> {
    let (_, rest) = arguments.split_once('"')?;
    let (path, _) = rest.split_once('"')?;
    (!path.contains('\\')).then_some(path)
}

/// `path` with its empty and `.` parts dropped, or nothing when it climbs.
fn normal(path: &str) -> Option<String> {
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => return None,
            part => parts.push(part),
        }
    }
    Some(format!("/{}", parts.join("/")))
}

/// `path` from the root of `source`, when it's under it.
fn under(source: &str, path: &str) -> Option<String> {
    if path == source {
        return Some(String::new());
    }
    path.strip_prefix(source)?
        .strip_prefix('/')
        .map(str::to_owned)
}

/// Where what a base's VM tests read is kept, beside its derivations.
pub fn keeping(cache: &Path, base: &str, system: &str) -> PathBuf {
    cache
        .join("changed-checks")
        .join(format!("{base}-{system}.reads.json"))
}

/// A base being kept: its commit in the checkout at `root`, its flake from
/// git, and where what its VM tests read is kept.
struct Base<'a> {
    root: &'a str,
    commit: &'a str,
    system: &'a str,
    flake: String,
    at: PathBuf,
}

/// What a trace of the VM tests gave: their derivations, what they read,
/// the tree it was read from, and what each chunk took.
type Traced = Result<(Derivations, Reads, String, Evaluation), String>;

/// Keep a base's checks, as a keep of main's head asks: the ordinary ones
/// from git, and the VM tests from the plain path under strace, keeping
/// what they read beside them. On a builder made for the run, with nothing
/// kept yet, the two go side by side, each with half its room, and the
/// trace begins before the base's lock is waited on, so a gate evaluating
/// the same base holds back only the derivations: on 12 November three
/// keeps waited there 119 to 704 s before they began tracing, and one
/// after the other, each half waiting on its slowest chunk, a keep took
/// 679 s. Elsewhere, or with the derivations kept, they go one at a time.
pub fn keep_base(root: &str, commit: &str, system: &str, noted: &mut Run) -> Result<(), String> {
    let base = Base {
        root,
        commit,
        system,
        flake: format!("git+file://{root}?rev={commit}"),
        at: keeping(&crate::cache(), commit, system),
    };
    let path = checks::keeping(&crate::cache(), commit, system);
    let builder = std::env::var_os(crate::HERE).is_some();
    if !builder || path.exists() {
        let kept = checks::kept(&path, || {
            let (vm, ordinary) = split(&base.flake, system)?;
            one_at_a_time(&base, &vm, &ordinary, checks::at_once_here(builder), noted)
        })?;
        keep(root, commit, system, &kept, noted);
        return Ok(());
    }
    let (vm, ordinary) = split(&base.flake, system)?;
    if vm.is_empty() {
        checks::kept(&path, || {
            one_at_a_time(&base, &vm, &ordinary, checks::at_once_here(builder), noted)
        })?;
        return Ok(());
    }
    let at_once = checks::at_once_each_of_two_here();
    std::thread::scope(|scope| {
        let tracing = || traced(root, commit, system, &vm, &base.at, at_once);
        let mut beside = Some(scope.spawn(tracing));
        let kept = checks::kept(&path, || {
            let mut found = ordinarily(&base, &ordinary, at_once, noted)?;
            let traced = beside
                .take()
                .map_or_else(|| Err("the trace was taken".to_owned()), joined);
            found.extend(with_vm(&base, &vm, traced, noted)?);
            Ok(found)
        });
        // Still beside, the trace went unused: a gate kept the derivations
        // while it ran, or the ordinary checks failed.
        if let Some(beside) = beside {
            let traced = joined(beside);
            if let Ok(kept) = &kept
                && let Err(why) = checked(&base.at, kept, &vm, traced, noted)
            {
                unrecorded(system, commit, &why);
            }
        }
        kept.map(|_| ())
    })
}

/// A flake's VM tests and its other checks, by name.
fn split(flake: &str, system: &str) -> Result<(Vec<String>, Vec<String>), String> {
    Ok(evaluating::names(flake, system)?
        .into_iter()
        .partition(|name| name.starts_with("vm-")))
}

/// Evaluate the base's ordinary checks, `ordinary`, then trace its VM
/// tests, `vm`, each `at_once` chunks at a time.
fn one_at_a_time(
    base: &Base,
    vm: &[String],
    ordinary: &[String],
    at_once: usize,
    noted: &mut Run,
) -> Result<Derivations, String> {
    let mut found = ordinarily(base, ordinary, at_once, noted)?;
    if !vm.is_empty() {
        let traced = traced(base.root, base.commit, base.system, vm, &base.at, at_once);
        found.extend(with_vm(base, vm, traced, noted)?);
    }
    Ok(found)
}

/// Evaluate the base's ordinary checks from git, `at_once` chunks at a time.
fn ordinarily(
    base: &Base,
    ordinary: &[String],
    at_once: usize,
    noted: &mut Run,
) -> Result<Derivations, String> {
    let mut evaluation = Run::evaluating("base");
    let found =
        evaluating::evaluated_by(&base.flake, base.system, ordinary, &mut evaluation, at_once)?;
    noted.evaluated(evaluation);
    Ok(found)
}

/// Each VM test's derivation as a trace of `vm` came to it, keeping what
/// they read; or, when the trace or its record failed, each one's from
/// git, as a gate would evaluate it, saying why.
fn with_vm(
    base: &Base,
    vm: &[String],
    traced: Traced,
    noted: &mut Run,
) -> Result<Derivations, String> {
    let said = traced.and_then(|(said, reads, tree, evaluation)| {
        let said = vm
            .iter()
            .map(|name| {
                let derivation = said
                    .get(name)
                    .ok_or_else(|| format!("from a plain path, {name} isn't among the checks"))?;
                Ok((name.clone(), derivation.clone()))
            })
            .collect::<Result<Derivations, String>>()?;
        write(&base.at, &reads, &tree).map(|()| (said, evaluation))
    });
    match said {
        Ok((said, evaluation)) => {
            noted.evaluated(evaluation);
            Ok(said)
        }
        Err(why) => {
            let (system, short) = (base.system, base.commit.get(..12).unwrap_or(base.commit));
            eprintln!(
                "changed-checks: couldn't record what {system}'s VM tests read at {short}, so \
                 they're evaluated from git: {why}"
            );
            let mut evaluation = Run::evaluating("base");
            let found = evaluating::evaluated(&base.flake, system, vm, &mut evaluation)?;
            noted.evaluated(evaluation);
            Ok(found)
        }
    }
}

/// Join the trace beside, one that panicked having stopped.
fn joined(beside: std::thread::ScopedJoinHandle<'_, Traced>) -> Traced {
    beside
        .join()
        .unwrap_or_else(|_| Err("the trace stopped".to_owned()))
}

/// Keep what the VM tests among a base's `derivations` read, unless it's
/// kept already, as a base a gate kept first has not, saying why when it
/// can't be: they're then evaluated in each gate on that base, as before.
fn keep(root: &str, base: &str, system: &str, derivations: &Derivations, noted: &mut Run) {
    let at = keeping(&crate::cache(), base, system);
    if at.exists() || !derivations.keys().any(|name| name.starts_with("vm-")) {
        return;
    }
    if let Err(why) = record(root, base, system, derivations, &at, noted) {
        unrecorded(system, base, &why);
    }
}

/// Say why what a base's VM tests read went unrecorded, so each gate on
/// it evaluates them, as before.
fn unrecorded(system: &str, base: &str, why: &str) {
    let short = base.get(..12).unwrap_or(base);
    eprintln!("changed-checks: couldn't record what {system}'s VM tests read at {short}: {why}");
}

/// The base's derivation of each VM test, when the tree's VM tests, `vm`,
/// are the base's and the change from it reaches nothing they read.
pub fn unread(
    root: &str,
    base: &str,
    system: &str,
    before: &Derivations,
    vm: &[String],
) -> Option<Derivations> {
    let kept: Derivations = before
        .iter()
        .filter(|(name, _)| name.starts_with("vm-"))
        .map(|(name, derivation)| (name.clone(), derivation.clone()))
        .collect();
    let named: BTreeSet<&String> = vm.iter().collect();
    if !kept.keys().eq(named) || !untouched(&keeping(&crate::cache(), base, system), root, base) {
        return None;
    }
    let short = base.get(..12).unwrap_or(base);
    eprintln!(
        "changed-checks: the change reaches nothing {system}'s {} VM tests read at {short}, \
         so each keeps the base's derivation unevaluated",
        kept.len()
    );
    Some(kept)
}

/// The tree a commit holds, which a record is made from and used for.
fn tree(root: &str, commit: &str) -> Result<String, String> {
    crate::run(
        "git",
        &["-C", root, "rev-parse", &format!("{commit}^{{tree}}")],
    )
}

/// Evaluate the VM tests among `derivations`, the base's from git, once
/// more under strace, from the same source as a plain path, and keep what
/// they read at `at`. Each must come to the derivation it came to from
/// git, or the two sources differ and what one read says nothing of the
/// other.
fn record(
    root: &str,
    base: &str,
    system: &str,
    derivations: &Derivations,
    at: &Path,
    noted: &mut Run,
) -> Result<(), String> {
    let vm: Vec<String> = derivations
        .keys()
        .filter(|name| name.starts_with("vm-"))
        .cloned()
        .collect();
    let at_once = checks::at_once_here(std::env::var_os(crate::HERE).is_some());
    let traced = traced(root, base, system, &vm, at, at_once);
    checked(at, derivations, &vm, traced, noted)
}

/// Keep at `at` what a trace saw the VM tests `vm` read, when each came to
/// the derivation it came to from git, in `derivations`.
fn checked(
    at: &Path,
    derivations: &Derivations,
    vm: &[String],
    traced: Traced,
    noted: &mut Run,
) -> Result<(), String> {
    let (said, reads, tree, evaluation) = traced?;
    noted.evaluated(evaluation);
    if let Some(name) = vm
        .iter()
        .find(|name| said.get(*name) != derivations.get(*name))
    {
        return Err(format!(
            "from a plain path, {name} isn't the derivation it is from git"
        ));
    }
    write(at, &reads, &tree)
}

/// Evaluate the VM tests `vm` of `base` under strace, from the same source
/// as a plain path, giving their derivations, what they read and the tree
/// it was read from, and what each chunk took, `at_once` chunks at a time.
/// The traces are written beside `at` as they're made.
fn traced(
    root: &str,
    base: &str,
    system: &str,
    vm: &[String],
    at: &Path,
    at_once: usize,
) -> Result<(Derivations, Reads, String, Evaluation), String> {
    let tree = tree(root, base)?;
    let flake = format!("git+file://{root}?rev={base}");
    let expression = format!("(builtins.getFlake \"{flake}\").outPath");
    let asked = ["eval", "--raw", "--expr", &expression, evaluating::UNCACHED];
    let source = crate::run("nix", &evaluating::pure(&asked))?;
    if let Some(input) = local_input(&Path::new(&source).join("flake.lock"))? {
        return Err(format!(
            "{input}, a flake input, is a local path, which a trace of the source wouldn't follow"
        ));
    }
    let checks = format!("path:{source}#checks.{system}");
    let chunks: Vec<(usize, &[String])> = checks::chunks(vm).into_iter().enumerate().collect();
    let strace = std::env::var(STRACE).unwrap_or_else(|_| STRACE_BUILT.to_owned());
    let mut evaluation = Run::evaluating("reads");
    // Nix keeps what it copied of a source in its fetcher cache, and a copy
    // it finds there it reads nothing for: evaluated again on 12 November,
    // a folder used as a source showed no read at all. So the chunks share
    // a cache that starts empty, where each copy was made by one of them,
    // whose trace holds what making it read.
    let fresh = at.with_extension("cache");
    if let Some(folder) = at.parent() {
        sweep(folder);
    }
    let _ = fs::remove_dir_all(&fresh);
    fs::create_dir_all(&fresh).map_err(|error| format!("{}: {error}", fresh.display()))?;
    let traced = checks::each_at_once(
        &chunks,
        at_once,
        || !crate::pressure::memory_stalled(),
        |(number, chunk)| {
            let trace = at.with_extension(format!("trace{number}"));
            let selecting = checks::selecting(chunk)?;
            let trace_name = trace.to_string_lossy();
            let mut asked = vec!["-f", "-qq", "-y", "-e", TRACED, "-o", &trace_name, "nix"];
            asked.extend(evaluating::pure(&[
                "eval",
                "--json",
                &checks,
                "--apply",
                &selecting,
                evaluating::UNCACHED,
            ]));
            let began = Instant::now();
            let ran = uncached(&fresh, &strace, &asked);
            let took = began.elapsed();
            let text = fs::read_to_string(&trace);
            let _ = fs::remove_file(&trace);
            let said: Derivations = serde_json::from_str(&ran?)
                .map_err(|error| format!("nix gave the checks' derivations oddly: {error}"))?;
            let text = text.map_err(|error| format!("{}: {error}", trace.display()))?;
            let reads = Reads::traced(&text, &source)
                .ok_or_else(|| "a line of the trace names no path it read".to_owned())?;
            // Every evaluation of the flake reads its flake.nix, so a trace
            // without it didn't see the evaluation.
            if !reads.read.contains("flake.nix") {
                return Err("the trace saw nothing read".to_owned());
            }
            Ok::<_, String>((chunk.len(), took, said, reads))
        },
    );
    let _ = fs::remove_dir_all(&fresh);
    let mut said = Derivations::new();
    let mut reads = Reads::default();
    for (checks, took, derivations, read) in traced? {
        evaluation.chunk(checks, took);
        said.extend(derivations);
        reads.join(read);
    }
    Ok((said, reads, tree, evaluation))
}

/// How long a keep's traces and cache may stand before another keep takes
/// them for left behind: a keep took 11 minutes on 12 November.
const LEFT_AFTER: Duration = Duration::from_hours(2);

/// Clear from `folder` the traces, caches and half-written records a keep
/// stopped while tracing left there, as a switch stopped one on 12
/// November, leaving 30 MB of traces, and the copies from another builder
/// a killed ssh cut short; one still being written stays.
fn sweep(folder: &Path) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.map_while(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let left = [
            ".reads.trace",
            ".reads.cache",
            ".reads.writing",
            ".copying.",
        ]
        .iter()
        .any(|kind| name.contains(kind));
        let old = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > LEFT_AFTER);
        if left && old {
            let path = entry.path();
            let _ = fs::remove_file(&path).or_else(|_| fs::remove_dir_all(&path));
        }
    }
}

/// Keep `reads`, read from `tree`, at `at`, whole or not at all.
fn write(at: &Path, reads: &Reads, tree: &str) -> Result<(), String> {
    let writing = at.with_extension("writing");
    let failed = |error: std::io::Error| format!("{}: {error}", at.display());
    fs::write(&writing, reads.kept(tree).to_string()).map_err(failed)?;
    fs::rename(&writing, at).map_err(failed)
}

/// Run a program as [`crate::run`] does, with the cache folder at `fresh`.
fn uncached(fresh: &Path, program: &str, arguments: &[&str]) -> Result<String, String> {
    let ran = std::process::Command::new(program)
        .args(arguments)
        .env("XDG_CACHE_HOME", fresh)
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(|error| format!("running {program}: {error}"))?;
    if !ran.status.success() {
        return Err(format!(
            "{program} {} failed: {}",
            arguments.join(" "),
            ran.status
        ));
    }
    String::from_utf8(ran.stdout).map_err(|_| format!("{program} printed what isn't text"))
}

/// The first flake input `flake.lock` locks to a local path, if any; none
/// when there's no lock. One the lock can't say leaves it unread.
fn local_input(lock: &Path) -> Result<Option<String>, String> {
    let text = match fs::read_to_string(lock) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{}: {error}", lock.display())),
    };
    let lock: Value =
        serde_json::from_str(&text).map_err(|error| format!("flake.lock: {error}"))?;
    let nodes = lock
        .get("nodes")
        .and_then(Value::as_object)
        .ok_or("flake.lock holds no nodes")?;
    Ok(nodes
        .iter()
        .find(|(_, node)| {
            let locked = node.get("locked");
            let kind = locked
                .and_then(|locked| locked.get("type"))
                .and_then(Value::as_str);
            let url = locked
                .and_then(|locked| locked.get("url"))
                .and_then(Value::as_str);
            kind == Some("path") || url.is_some_and(|url| url.contains("file:"))
        })
        .map(|(name, _)| name.clone()))
}

/// Whether the change from `base` to the working tree at `root` reaches
/// nothing the record at `at` says the base's VM tests read: no when
/// there's no record, it was made from another tree, or git can't say.
pub fn untouched(at: &Path, root: &str, base: &str) -> bool {
    let Ok(text) = fs::read_to_string(at) else {
        return false;
    };
    let Some(reads) = tree(root, base)
        .ok()
        .and_then(|tree| Reads::from_kept(&text, &tree))
    else {
        return false;
    };
    // A folder came or went when it's at the base or in the tree, not both.
    let came_or_went = |folder: &str| {
        let at_base = format!("{base}:{folder}");
        let was = crate::run("git", &["-C", root, "cat-file", "-e", &at_base]).is_ok();
        was != Path::new(root).join(folder).is_dir()
    };
    let asked = ["-C", root, "diff", "--name-status", "--no-renames", base];
    let Ok(said) = crate::run("git", &asked) else {
        return false;
    };
    touched(&said).is_some_and(|touched| {
        touched
            .iter()
            .all(|(file, how)| !reads.reached_by(file, *how, &came_or_went))
    })
}

/// Each file `git diff --name-status` names, and how; nothing when a line
/// isn't one it gives.
fn touched(said: &str) -> Option<Vec<(String, Touched)>> {
    said.lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let (status, file) = line.split_once('\t')?;
            let how = match status {
                "A" => Touched::Added,
                // A change of kind takes the entry a listing gave away.
                "D" | "T" => Touched::Removed,
                "M" => Touched::Changed,
                _ => return None,
            };
            Some((file.to_owned(), how))
        })
        .collect()
}

#[cfg(test)]
mod tests;
