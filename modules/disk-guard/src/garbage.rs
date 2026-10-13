//! Nix garbage, as a rebuild cache with a budget: which dead store paths
//! go, in what order, and in which batches.
//!
//! `nix store delete` deletes a path together with every dead path that
//! refers to it, so a path is dated by the newest registration among it
//! and all its dead referrers. Under that date, all garbage older than any
//! moment is closed under referrers, and deleting it takes nothing newer.

use serde_json::Value;
use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap, HashMap};

/// What a batch holds before it closes. A pass measures what it freed
/// only between batches, so one batch of a whole pile of disk images
/// would free 145 GiB to cover a shortfall of 20; this is about one image.
pub const BATCH_BYTES: u64 = 8 << 30;

/// A store path no root reaches, as `nix path-info` describes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Dead {
    pub path: String,
    /// When it was registered, in wall seconds.
    pub registered: f64,
    /// Its size unpacked, in bytes.
    pub size: u64,
    /// The store paths it refers to, dead or alive.
    pub references: Vec<String>,
}

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

/// The garbage in the order it goes, in batches of at most `most` paths
/// that close once they hold `BATCH_BYTES`:
/// first what is older than `keep`, oldest first; then the young, largest
/// first, each only once no garbage left refers to it. A batch is kept
/// while healthy once the young garbage left fits in `budget` bytes.
pub fn batches(dead: &[Dead], now: f64, keep: f64, budget: u64, most: usize) -> Vec<Batch> {
    let index: HashMap<&str, usize> = dead
        .iter()
        .enumerate()
        .map(|(at, path)| (path.path.as_str(), at))
        .collect();
    // Each path's dead references, without itself or repeats.
    let refers: Vec<BTreeSet<usize>> = dead
        .iter()
        .enumerate()
        .map(|(at, path)| {
            path.references
                .iter()
                .filter_map(|reference| index.get(reference.as_str()).copied())
                .filter(|&reference| reference != at)
                .collect()
        })
        .collect();
    let referrers = |refers: &[BTreeSet<usize>]| {
        let mut count = vec![0usize; dead.len()];
        for references in refers {
            for &reference in references {
                count[reference] += 1;
            }
        }
        count
    };

    // Dated by the newest referrer, visiting referrers before what they
    // refer to. The store's references never loop but through a path
    // itself.
    let mut newest: Vec<f64> = dead.iter().map(|path| path.registered).collect();
    let mut waiting = referrers(&refers);
    let mut ready: Vec<usize> = (0..dead.len()).filter(|&at| waiting[at] == 0).collect();
    while let Some(at) = ready.pop() {
        for &reference in &refers[at] {
            newest[reference] = newest[reference].max(newest[at]);
            waiting[reference] -= 1;
            if waiting[reference] == 0 {
                ready.push(reference);
            }
        }
    }

    let cutoff = now - keep;
    let mut old: Vec<usize> = (0..dead.len()).filter(|&at| newest[at] <= cutoff).collect();
    old.sort_by(|&a, &b| {
        newest[a]
            .total_cmp(&newest[b])
            .then(dead[a].path.cmp(&dead[b].path))
    });
    let mut order: Vec<(usize, bool)> = old.into_iter().map(|at| (at, false)).collect();

    // A referrer of young garbage is young too, so the young stand alone.
    // The largest goes first, with whatever garbage still refers to it,
    // referrers first, as deleting it would take them anyway: a disk
    // image goes with the small link a test run left to it.
    let young = |at: usize| newest[at] > cutoff;
    let mut referred_by = vec![Vec::new(); dead.len()];
    for (at, references) in refers.iter().enumerate() {
        for &reference in references {
            referred_by[reference].push(at);
        }
    }
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
            for &referrer in &referred_by[at] {
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
