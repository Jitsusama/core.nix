//! The record of what the train landed, a line each in
//! `train/landed.jsonl`, which cadence reads for how long riders waited.

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
    let kept = OpenOptions::new()
        .create(true)
        .append(true)
        .open(folder(root).join("landed.jsonl"))
        .and_then(|mut file| file.write_all(format!("{line}\n").as_bytes()));
    if let Err(error) = kept {
        eprintln!("merge-net: the train's landing at {landed} wasn't kept: {error}");
    }
}
