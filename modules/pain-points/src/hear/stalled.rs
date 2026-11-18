//! Turns that stalled Joel, heard as pains though he said nothing: the
//! listener's foresight. `pain-points read` already times each of his
//! turns in pi and keeps them in its state; a turn that lost him a minute
//! or more goes to its session's cell the way his words do, so the cell
//! can fix what slowed him before he has to say so.

use super::heard::Heard;
use crate::pi::Log;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// A turn that lost him at least this long is worth its cell's time: read
/// names every lost second, and a cell sent each would hear noise.
pub const STALL_S: f64 = 60.0;

/// The kind a stall is heard as, which finding and note read to tell it
/// from what he said.
pub const KIND: &str = "stalled";

/// What read keeps of pi's logs, by each log's file name.
#[derive(Deserialize)]
struct Kept {
    logs: BTreeMap<String, Log>,
}

/// read's turns for each log, or none when read hasn't run.
pub fn logs(state: &Path) -> BTreeMap<String, Log> {
    fs::read(state.join("pi-logs.json"))
        .ok()
        .and_then(|kept| serde_json::from_slice::<Kept>(&kept).ok())
        .map(|kept| kept.logs)
        .unwrap_or_default()
}

/// Whether a pain is a stall rather than his words.
pub fn is(pain: &Heard) -> bool {
    pain.kinds.iter().any(|k| k == KIND)
}

/// Each turn in `log` begun in [since, until) that lost him a minute or
/// more, as a pain heard in `session`.
pub fn stalls(log: &Log, session: &str, void: bool, since: i64, until: i64) -> Vec<Heard> {
    crate::pi::painful(&[log.turns.as_slice()], since, until)
        .into_iter()
        .filter(|p| p.lost_seconds >= STALL_S)
        .map(|p| {
            let entry = format!("turn-{}", p.at);
            let why = match p.slowed {
                "waiting" => "waiting on pi's queue or a tool that held it",
                "an error" => "the provider failing",
                "stopped it" => "he stopped it",
                "tried again" => "he had to say it again",
                other => other,
            };
            Heard {
                key: format!("{session}/{entry}"),
                session: session.to_string(),
                entry,
                at: p.at * 1000,
                void,
                words: format!(
                    "a turn of {:.0} s lost him {:.0} s, {why}",
                    p.seconds, p.lost_seconds
                ),
                kinds: vec![KIND.to_string()],
                looking_at: None,
                tools: Vec::new(),
                failed: Vec::new(),
                hits: 1,
                carried: false,
                to: None,
                routed_at: None,
                tries: 0,
                taken_by: None,
                closed_by: None,
            }
        })
        .collect()
}
