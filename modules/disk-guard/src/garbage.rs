//! Nix garbage, as a rebuild cache with a budget: which dead store paths
//! go, in what order, and in which batches.
//!
//! `nix store delete` refuses a path while anything still refers to it
//! that it wasn't given too, calling it alive, so a path goes only with
//! or after all its dead referrers, and is dated by the newest
//! registration among it and them. Under that date, all garbage older
//! than any moment is closed under referrers, and deleting it needs
//! nothing newer.

use crate::commands::{self, Job, Ran};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::ffi::OsStr;
use std::path::Path;
use std::time::Duration;

/// What a batch holds before it closes. A pass measures what it freed
/// only between batches, so one batch of a whole pile of disk images
/// would free 145 GiB to cover a shortfall of 20; this is about one image.
pub const BATCH_BYTES: u64 = 8 << 30;

/// A store path no root reaches, as `nix path-info` describes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dead {
    pub path: String,
    /// When it was registered, in wall seconds.
    pub registered: f64,
    /// Its size unpacked, in bytes.
    pub size: u64,
    /// The store paths it refers to, dead or alive.
    pub references: Vec<String>,
}

/// The dead paths as last listed, kept across restarts since listing them
/// takes about a minute of the disk's time. A store's listing can hold
/// 80,000 paths with a score of references each, so each path holds only
/// the references that can hold it back, the dead ones, by their place in
/// the listing: as names, every reference live or dead, one held 200 MiB.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Listing {
    /// When the listing began, in wall seconds: garbage made since isn't
    /// in it.
    pub listed_at: f64,
    pub dead: Vec<Listed>,
}

/// A dead path as a listing holds it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Listed {
    pub path: String,
    /// When it was registered, in wall seconds.
    pub registered: f64,
    /// Its size unpacked, in bytes.
    pub size: u64,
    /// The other dead paths it refers to, by their place in the listing,
    /// each once.
    pub refers: Vec<u32>,
}

impl Listing {
    /// The listing of `dead`, begun at `listed_at`.
    pub fn new(listed_at: f64, dead: Vec<Dead>) -> Listing {
        // Places count in u32, as references hold them; no listing comes
        // near four billion dead paths.
        let index: HashMap<&str, u32> = (0u32..)
            .zip(&dead)
            .map(|(at, path)| (path.path.as_str(), at))
            .collect();
        let refers: Vec<Vec<u32>> = (0u32..)
            .zip(&dead)
            .map(|(at, path)| among(&index, at, &path.references))
            .collect();
        let dead = dead
            .into_iter()
            .zip(refers)
            .map(|(path, refers)| Listed {
                path: path.path,
                registered: path.registered,
                size: path.size,
                refers,
            })
            .collect();
        Listing { listed_at, dead }
    }
}

/// Of `references`, those `index` places among the dead, but for `at`
/// itself, each once.
fn among(index: &HashMap<&str, u32>, at: u32, references: &[String]) -> Vec<u32> {
    let mut places: Vec<u32> = references
        .iter()
        .filter_map(|reference| index.get(reference.as_str()).copied())
        .filter(|&place| place != at)
        .collect();
    places.sort_unstable();
    places.dedup();
    places.shrink_to_fit();
    places
}

/// How long one step of `nix store gc --max` may take, when there is no
/// listing to delete from.
pub const GC_TIMEOUT: Duration = Duration::from_mins(10);

/// What the `nix` command needs to be told before any of the commands the
/// guard runs, which stock Nix still calls experimental and refuses
/// unless the machine's own settings allow them. Where they're stable the
/// words are taken without a complaint.
pub const NIX_COMMAND: [&str; 2] = ["--extra-experimental-features", "nix-command"];

/// Dead paths to delete together, in the order given.
#[derive(Clone, Debug, PartialEq)]
pub struct Batch {
    pub paths: Vec<String>,
    pub size: u64,
    /// The newest registration among them and all that refers to them.
    pub newest: f64,
    /// Whether a healthy disk keeps it: young, and within the budget.
    pub kept: bool,
}

/// Reads `nix path-info --json`: an object of store paths, each with its
/// size, registration time and references, or null for one no longer
/// valid, which is left out since it's already gone. Anything else is
/// refused rather than guessed at, since a misread reference could delete
/// what a young path still needs.
pub fn parse(json: &str) -> Result<Vec<Dead>, String> {
    let value: Value = serde_json::from_str(json).map_err(|error| error.to_string())?;
    let Value::Object(paths) = value else {
        return Err("nix path-info gave no object of paths".to_owned());
    };
    paths
        .into_iter()
        .filter(|(_, info)| !info.is_null())
        .map(|(path, info)| {
            let refused = |what: &str| format!("nix path-info gave {path} no {what}");
            let size = info["narSize"].as_u64().ok_or_else(|| refused("narSize"))?;
            let registered = info["registrationTime"]
                .as_f64()
                .ok_or_else(|| refused("registrationTime"))?;
            let references = info["references"]
                .as_array()
                .ok_or_else(|| refused("references"))?
                .iter()
                .map(|reference| reference.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| refused("readable references"))?;
            Ok(Dead {
                path,
                registered,
                size,
                references,
            })
        })
        .collect()
}

/// How many paths one `nix path-info` is asked about, so its arguments
/// stay well inside what the kernel lets a program be given.
const CHUNK: usize = 1000;
/// How long listing the dead may take: about a minute on a store of
/// 240 GiB of garbage, and far longer at idle priority on a busy disk.
const LIST_TIMEOUT: Duration = Duration::from_mins(30);
/// How long describing one chunk may take.
const DESCRIBE_TIMEOUT: Duration = Duration::from_mins(5);
/// The most of a command's output read: a listing of 12,000 paths is
/// 4 MiB, and a chunk's description about 3.
const OUTPUT_LIMIT: usize = 256 << 20;

/// Every dead path, through `nix-store --gc --print-dead` beside `nix`,
/// and what each weighs and refers to, through `nix path-info`. With
/// `idle`, at idle priority. A path that died since is found at the next
/// listing; one that came alive is refused by Nix when it's deleted.
pub fn list(nix: &Path, idle: bool) -> Result<Vec<Listed>, String> {
    let store = nix.with_file_name("nix-store");
    let listed = ran(
        &[store.as_os_str(), "--gc".as_ref(), "--print-dead".as_ref()],
        LIST_TIMEOUT,
        idle,
    )?;
    // Nix's own temporary files in the store's directory, such as its
    // .tmp-link-s, are listed too and gone a moment later, and asking
    // about one fails the whole listing; no store path's name has a dot
    // first.
    let paths: Vec<&str> = listed
        .lines()
        .filter(|line| line.starts_with('/'))
        .filter(|line| {
            Path::new(line)
                .file_name()
                .is_some_and(|name| !name.as_encoded_bytes().starts_with(b"."))
        })
        .collect();
    // Each chunk's references are placed as they're read, among all the
    // dead the listing named, so no chunk's names are held past it.
    let index: HashMap<&str, u32> = (0u32..).zip(&paths).map(|(at, path)| (*path, at)).collect();
    let mut described: Vec<Option<Listed>> = vec![None; paths.len()];
    for chunk in paths.chunks(CHUNK) {
        let mut argv: Vec<&OsStr> = vec![nix.as_os_str()];
        argv.extend(NIX_COMMAND.map(OsStr::new));
        // A dead path is only ever in the local store, so nothing a
        // substituter says is wanted, and the system unit has no network
        // to ask one: unasked, Nix looks up a path it can't find there.
        argv.extend(["path-info", "--json", "--offline"].map(OsStr::new));
        argv.extend(chunk.iter().map(OsStr::new));
        for path in parse(&ran(&argv, DESCRIBE_TIMEOUT, idle)?)? {
            let Some(&at) = index.get(path.path.as_str()) else {
                continue;
            };
            described[at as usize] = Some(Listed {
                refers: among(&index, at, &path.references),
                path: path.path,
                registered: path.registered,
                size: path.size,
            });
        }
    }
    // What Nix no longer knows is gone already, so the rest close up
    // around it, and a reference to it holds nothing back.
    let mut placed: Vec<Option<u32>> = Vec::with_capacity(described.len());
    let mut next = 0;
    for path in &described {
        placed.push(path.as_ref().map(|_| {
            next += 1;
            next - 1
        }));
    }
    Ok(described
        .into_iter()
        .flatten()
        .map(|mut path| {
            path.refers = path
                .refers
                .iter()
                .filter_map(|&at| placed[at as usize])
                .collect();
            path.refers.shrink_to_fit();
            path
        })
        .collect())
}

/// How long deleting one batch may take.
const DELETE_TIMEOUT: Duration = Duration::from_mins(10);
/// How many paths may come alive since the listing before it's taken as
/// too stale to go on with.
const REVIVALS: usize = 16;

/// Deletes the batch's `paths` that still exist through `nix store
/// delete`, whose dead referrers are among them or gone. Nix refuses a path a root
/// reaches again since the listing, having deleted those before it, so
/// it's dropped and the rest tried again.
pub fn delete(nix: &Path, paths: &[String], idle: bool) -> Result<(), String> {
    let mut alive: Vec<String> = Vec::new();
    loop {
        let left: Vec<&String> = paths
            .iter()
            .filter(|path| !alive.contains(path))
            .filter(|path| Path::new(path).symlink_metadata().is_ok())
            .collect();
        if left.is_empty() {
            return Ok(());
        }
        let mut argv: Vec<&OsStr> = vec![nix.as_os_str()];
        argv.extend(NIX_COMMAND.map(OsStr::new));
        argv.extend(["store", "delete"].map(OsStr::new));
        argv.extend(left.iter().map(|path| OsStr::new(path.as_str())));
        let (ok, said) = output(&argv, DELETE_TIMEOUT, idle)?;
        if ok {
            return Ok(());
        }
        let revived = said
            .split("Cannot delete path '")
            .nth(1)
            .and_then(|rest| rest.split_once("' since it is still alive"))
            .map(|(path, _)| path.to_owned())
            .filter(|path| left.contains(&path));
        match revived {
            Some(path) if alive.len() < REVIVALS => alive.push(path),
            Some(_) => return Err(format!("{REVIVALS} paths came alive since the listing")),
            None => return Err(failed(&argv, &said)),
        }
    }
}

/// What a command printed, or why it failed, in its own last words.
pub(crate) fn ran(argv: &[&OsStr], timeout: Duration, idle: bool) -> Result<String, String> {
    match output(argv, timeout, idle)? {
        (true, stdout) => Ok(stdout),
        (false, stderr) => Err(failed(argv, &stderr)),
    }
}

/// Whether a command succeeded, with what it printed if so and what it
/// said on its error stream if not.
fn output(argv: &[&OsStr], timeout: Duration, idle: bool) -> Result<(bool, String), String> {
    let name = name(argv);
    let job = Job {
        argv: argv.iter().map(|&arg| arg.to_owned()).collect(),
        cwd: None,
        env: Vec::new(),
        timeout,
        limit: OUTPUT_LIMIT,
        idle,
    };
    match commands::run(&job).map_err(|error| format!("starting {name}: {error}"))? {
        Ran::TimedOut | Ran::Stopped => Err(format!("{name} ran past {timeout:?}")),
        Ran::Finished(output) if output.truncated => Err(format!("{name} said more than it may")),
        Ran::Finished(output) if output.status == Some(0) => {
            let stdout = String::from_utf8(output.stdout);
            Ok((
                true,
                stdout.map_err(|_| format!("{name} printed what isn't text"))?,
            ))
        }
        Ran::Finished(output) => Ok((false, String::from_utf8_lossy(&output.stderr).into_owned())),
    }
}

fn failed(argv: &[&OsStr], said: &str) -> String {
    let last = said
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    format!("{} failed: {last}", name(argv))
}

fn name(argv: &[&OsStr]) -> String {
    let program = Path::new(argv[0]).file_name().unwrap_or_default();
    program.to_string_lossy().into_owned()
}

impl Listing {
    /// The garbage `there` says is still there, in the order it goes, in
    /// batches of at most `most` paths that close once they hold
    /// `BATCH_BYTES`: first what is older than `keep`, oldest first; then
    /// the young, largest first, each after the garbage still referring to
    /// it, since Nix refuses it before those. A batch is kept while healthy
    /// once the young garbage left fits in `budget` bytes. What's gone holds
    /// nothing back.
    pub fn batches(
        &self,
        there: impl Fn(&Listed) -> bool,
        now: f64,
        keep: f64,
        budget: u64,
        most: usize,
    ) -> Vec<Batch> {
        let mut local: Vec<Option<usize>> = vec![None; self.dead.len()];
        let mut dead: Vec<&Listed> = Vec::new();
        for (at, path) in self.dead.iter().enumerate() {
            if there(path) {
                local[at] = Some(dead.len());
                dead.push(path);
            }
        }
        let mut refers = Edges {
            starts: Vec::with_capacity(dead.len() + 1),
            to: Vec::new(),
        };
        refers.starts.push(0);
        for path in &dead {
            refers.to.extend(
                path.refers
                    .iter()
                    .filter_map(|&at| local.get(at as usize).copied().flatten()),
            );
            refers.starts.push(refers.to.len());
        }
        order(&dead, &refers, now, keep, budget, most)
    }
}

/// Each path's edges to others, by place, kept flat: a store's worth is a
/// few allocations rather than one for each of its 80,000 paths, which the
/// allocator would keep long after.
struct Edges {
    /// Where each path's edges begin in `to`, and where the last's end.
    starts: Vec<usize>,
    to: Vec<usize>,
}

impl Edges {
    fn of(&self, at: usize) -> &[usize] {
        &self.to[self.starts[at]..self.starts[at + 1]]
    }

    /// The same edges the other way, each path's in the order the paths
    /// they come from are placed.
    fn reversed(&self) -> Edges {
        let paths = self.starts.len() - 1;
        let mut starts = vec![0usize; paths + 1];
        for &to in &self.to {
            starts[to + 1] += 1;
        }
        for at in 0..paths {
            starts[at + 1] += starts[at];
        }
        let mut filled = starts.clone();
        let mut to = vec![0usize; self.to.len()];
        for from in 0..paths {
            for &edge in self.of(from) {
                to[filled[edge]] = from;
                filled[edge] += 1;
            }
        }
        Edges { starts, to }
    }
}

/// The batches of `dead`, whose dead references `refers` holds by place.
fn order(
    dead: &[&Listed],
    refers: &Edges,
    now: f64,
    keep: f64,
    budget: u64,
    most: usize,
) -> Vec<Batch> {
    let referrers = |refers: &Edges| {
        let mut count = vec![0usize; dead.len()];
        for &reference in &refers.to {
            count[reference] += 1;
        }
        count
    };

    // Dated by the newest referrer, visiting referrers before what they
    // refer to, in the order `walked` keeps. The store's references never
    // loop but through a path itself.
    let mut newest: Vec<f64> = dead.iter().map(|path| path.registered).collect();
    let mut walked = vec![0usize; dead.len()];
    let mut waiting = referrers(refers);
    let mut ready: Vec<usize> = (0..dead.len()).filter(|&at| waiting[at] == 0).collect();
    let mut seen = 0;
    while let Some(at) = ready.pop() {
        walked[at] = seen;
        seen += 1;
        for &reference in refers.of(at) {
            newest[reference] = newest[reference].max(newest[at]);
            waiting[reference] -= 1;
            if waiting[reference] == 0 {
                ready.push(reference);
            }
        }
    }

    // A path shares its date with the referrer it took it from, and Nix
    // refuses a path while a referrer it wasn't given is still there, so
    // a tie goes in the order walked, referrers first.
    let cutoff = now - keep;
    let mut old: Vec<usize> = (0..dead.len()).filter(|&at| newest[at] <= cutoff).collect();
    old.sort_by(|&a, &b| {
        newest[a]
            .total_cmp(&newest[b])
            .then(walked[a].cmp(&walked[b]))
    });
    let mut order: Vec<(usize, bool)> = old.into_iter().map(|at| (at, false)).collect();

    // A referrer of young garbage is young too, so the young stand alone.
    // The largest goes first, with whatever garbage still refers to it,
    // referrers first, as it can't go before them: a disk
    // image goes with the small link a test run left to it.
    let young = |at: usize| newest[at] > cutoff;
    let referred_by = refers.reversed();
    let mut left: u64 = (0..dead.len())
        .filter(|&at| young(at))
        .map(|at| dead[at].size)
        .sum();
    let mut gone = vec![false; dead.len()];
    let mut largest: BinaryHeap<(u64, Reverse<&str>, usize)> = (0..dead.len())
        .filter(|&at| young(at))
        .map(|at| (dead[at].size, Reverse(dead[at].path.as_str()), at))
        .collect();
    while let Some((_, _, start)) = largest.pop() {
        // Each path once all that refers to it is out, walked without
        // recursion since a chain of references can be long.
        let mut todo = vec![(start, false)];
        while let Some((at, visited)) = todo.pop() {
            if gone[at] {
                continue;
            }
            if visited {
                gone[at] = true;
                order.push((at, left <= budget));
                left -= dead[at].size;
                continue;
            }
            todo.push((at, true));
            for &referrer in referred_by.of(at) {
                if !gone[referrer] && young(referrer) {
                    todo.push((referrer, false));
                }
            }
        }
    }

    let mut batches: Vec<Batch> = Vec::new();
    for (at, kept) in order {
        let fits = batches.last().is_some_and(|batch| {
            batch.kept == kept && batch.paths.len() < most && batch.size < BATCH_BYTES
        });
        if !fits {
            batches.push(Batch {
                paths: Vec::new(),
                size: 0,
                newest: f64::NEG_INFINITY,
                kept,
            });
        }
        let batch = batches.last_mut().expect("one was just pushed");
        batch.paths.push(dead[at].path.clone());
        batch.size += dead[at].size;
        batch.newest = batch.newest.max(newest[at]);
    }
    batches
}
