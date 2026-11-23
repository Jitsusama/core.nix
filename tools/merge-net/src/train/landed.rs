//! The record of what the train landed, and of each rider it refused, a
//! line each in `train/landed.jsonl`, which cadence reads for how long
//! riders waited.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use serde_json::json;

use super::{folder, riders};

/// When each of `heads` boarded, in their order, or none for a rider whose
/// file is gone or unreadable, so each slot still answers its head. Read
/// before the riders alight, which takes their files.
pub(super) fn boarded_at(root: &Path, heads: &[String]) -> Vec<Option<u64>> {
    let aboard = riders(root);
    heads
        .iter()
        .map(|head| {
            aboard
                .iter()
                .find(|rider| &rider.head == head)
                .map(|rider| rider.at)
        })
        .collect()
}

/// Keep what landed at `landed`, the heads it carried, when each boarded
/// and how long since its run began. The run's length isn't a rider's
/// wait: one boarded behind a split waited through every run before, which
/// only its boarding time shows.
pub(super) fn keep_landed(
    root: &Path,
    landed: &str,
    heads: &[String],
    boarded: &[Option<u64>],
    seconds: u64,
) {
    let line = json!({
        "at": crate::state::now(),
        "landed": landed,
        "heads": heads,
        "boarded": boarded,
        "seconds": seconds,
    });
    keep(root, &line, &format!("landing at {landed}"));
}

/// Keep that the rider at `head` fell off, and when it boarded, read
/// before it alights, so its wait counts though it never landed: a train
/// that keeps refusing riders would otherwise read better than it is. One
/// not aboard waited on no train, so nothing is kept of it.
pub(super) fn keep_fell(root: &Path, head: &str) {
    let heads = [head.to_owned()];
    let boarded = boarded_at(root, &heads);
    if boarded.iter().all(Option::is_none) {
        return;
    }
    let line = json!({
        "at": crate::state::now(),
        "fell": heads,
        "boarded": boarded,
    });
    keep(root, &line, &format!("rider {head} falling off"));
}

/// Add `line` to the record, saying `what` wasn't kept where it can't be.
fn keep(root: &Path, line: &serde_json::Value, what: &str) {
    let kept = OpenOptions::new()
        .create(true)
        .append(true)
        .open(folder(root).join("landed.jsonl"))
        .and_then(|mut file| file.write_all(format!("{line}\n").as_bytes()));
    if let Err(error) = kept {
        eprintln!("merge-net: the train's {what} wasn't kept: {error}");
    }
}
