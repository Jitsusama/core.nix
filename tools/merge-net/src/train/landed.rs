//! The record of what the train landed, and of each rider that left it
//! without landing, a line each in `train/landed.jsonl`, which cadence
//! reads for how long riders waited.

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

/// Keep what landed at `landed`, the heads it carried, when each boarded,
/// how long since `run` began and the checks that excused its red. The
/// run's length isn't a rider's
/// wait: one boarded behind a split waited through every run before, which
/// only its boarding time shows.
pub(super) fn keep_landed(
    root: &Path,
    landed: &str,
    heads: &[String],
    boarded: &[Option<u64>],
    run: &super::Run,
) {
    let line = json!({
        "at": crate::state::now(),
        "landed": landed,
        "excused": run.excused,
        "heads": heads,
        "boarded": boarded,
        "seconds": crate::state::now().saturating_sub(run.began),
    });
    keep(root, &line, &format!("landing at {landed}"));
}

/// How a rider left the train without landing.
#[derive(Clone, Copy)]
pub(super) enum Left {
    /// Refused, to run again alone, as `fall` takes one.
    Fell,
    /// Stepped off: its pull request a draft, moved on, closed or unread.
    Off,
}

/// Keep that the rider at `head`, which `boarded` when it was aboard, left
/// `how`, so its wait counts though it never landed: a train that sheds
/// riders would otherwise read better than it is.
pub(super) fn keep_left(root: &Path, head: &str, boarded: &[Option<u64>], how: Left) {
    let field = match how {
        Left::Fell => "fell",
        Left::Off => "off",
    };
    let mut line = json!({
        "at": crate::state::now(),
        "boarded": boarded,
    });
    line[field] = json!([head]);
    keep(root, &line, &format!("rider {head} leaving"));
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
