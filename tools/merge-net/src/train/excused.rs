//! What the train keeps of a candidate whose red it excused as main's or
//! a flake's: a record read as nix-offload's record of a pass is, so when
//! main moves before the candidate lands, its recut is judged by the rule
//! a passed candidate's is (`landing::still_passes`) rather than always
//! riding again. nix-offload writes its record only on a pass, so before
//! these an excused red never recut, and its riders were built again.
//!
//! The records sit in the train's own folder, apart from nix-offload's,
//! which `ready` and `needs` read, so nothing outside the train takes an
//! excused red for a pass.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde_json::json;

use super::flaky::{Blame, Red};
use crate::state::short;

/// The train's folder for them, beside its checkout.
const FOLDER: &str = "excused";

/// How long a record is read and kept: a day, as long as `ready` reads
/// nix-offload's.
const KEPT_FOR: Duration = Duration::from_hours(24);

/// How long a full commit's name is, as a record is named.
const COMMIT_LENGTH: usize = 40;

/// Keep, under the net's state folder `root`, the record of the `red`
/// candidate whose failed checks were each `judged` main's or a flake's,
/// and let go of those past their day.
pub(super) fn keep(root: &Path, red: &Red<'_>, judged: &[(String, Blame)]) {
    let folder = super::folder(root).join(FOLDER);
    let excused: Vec<_> = judged
        .iter()
        .map(|(check, blamed)| json!([check, format!("{blamed:?}")]))
        .collect();
    // A run that built, on the candidate's base, is what `built_there`
    // takes as a pass: no `--name-only` or `--keep-base`.
    let line = json!({ "dir": null, "run": ["--base", red.base], "excused": excused });
    let kept = fs::create_dir_all(&folder)
        .and_then(|()| fs::write(folder.join(red.candidate), format!("{line}\n")));
    if let Err(error) = kept {
        println!(
            "merge-net: the excuse of {} couldn't be kept, so a recut of it runs again: {error}",
            short(red.candidate)
        );
    }
    for (at, old) in records_in(&folder) {
        if !within_its_day(at) {
            // One left behind is only read past its day, never as a pass.
            let _ = fs::remove_file(folder.join(old));
        }
    }
}

/// The candidates the train excused in the last day, kept beside its
/// `checkout`, the newest first.
pub(super) fn recent(checkout: &Path) -> Vec<String> {
    let mut kept: Vec<(SystemTime, String)> = records_in(&beside(checkout))
        .into_iter()
        .filter(|(at, _)| within_its_day(*at))
        .collect();
    kept.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
    kept.into_iter().map(|(_, name)| name).collect()
}

/// The record the train kept of `candidate` beside its `checkout`, or
/// nothing.
pub(super) fn record(checkout: &Path, candidate: &str) -> String {
    if candidate.len() != COMMIT_LENGTH {
        return String::new();
    }
    fs::read_to_string(beside(checkout).join(candidate)).unwrap_or_default()
}

/// The folder of records beside the train's `checkout`, which `ride_from`
/// keeps in the train's folder, as `keep` writes them.
fn beside(checkout: &Path) -> PathBuf {
    checkout.with_file_name(FOLDER)
}

/// Each record in `folder`, with when it was written.
fn records_in(folder: &Path) -> Vec<(SystemTime, String)> {
    fs::read_dir(folder)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let at = entry.metadata().ok()?.modified().ok()?;
            let name = entry.file_name().into_string().ok()?;
            (name.len() == COMMIT_LENGTH).then_some((at, name))
        })
        .collect()
}

/// Whether a record written `at` is still within its day; one written
/// after now, as a clock set back leaves it, is.
fn within_its_day(at: SystemTime) -> bool {
    match SystemTime::now().duration_since(at) {
        Ok(age) => age < KEPT_FOR,
        Err(_) => true,
    }
}

#[cfg(test)]
#[path = "excused_tests.rs"]
mod tests;
