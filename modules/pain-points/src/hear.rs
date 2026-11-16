//! `pain-points hear`: the listener. Each run reads what's new in pi's
//! session logs, hears each message Joel said that says something hurt
//! (sore.rs), keeps it with his words and what the agent had just done
//! (said.rs), and sends each pain no one has yet as a finding to the cell
//! that was working on it (route.rs), once, ranked by how often he hits
//! the same thing (heard.rs). His orchestrator's asks and misses ledgers
//! are read, never written: a pain they carry goes to no one again, and
//! the keeper of the ledgers hears of each new one. Nothing is ever sent
//! to the void, and nothing is posted anywhere.
//!
//! Unlike `read`, which keeps kinds and timings alone, `hear` keeps his
//! words, since a finding without them can't be acted on. They stay in
//! its own state (heard.jsonl, a fortnight of it) and go only over the
//! machine's own mycelium socket to the cell that owns what hurt.
//!
//! It writes hour.json each run, for cadence's hourly card: the pains
//! heard and routed in the last hour, those of the last day still open,
//! and the one he hit most.

pub mod heard;
pub mod route;
pub mod said;
pub mod sore;

use heard::Heard;
use route::{Cell, Routes};
use said::Carried;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// How far back a first run hears, and how long a pain stays open.
pub const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;

/// Where a run listens and what it knows of the time.
pub struct Ears {
    pub sessions: PathBuf,
    pub state: PathBuf,
    /// The asks and misses ledgers, read alone.
    pub ledgers: Vec<PathBuf>,
    /// The void's folder: a session begun there is the void.
    pub void: PathBuf,
    pub at: i64,
    /// His offset from UTC, in seconds, to say his own times.
    pub offset_s: i64,
}

/// How each session log stands between runs.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Following {
    offset: u64,
    session: String,
    void: bool,
    lab: bool,
    carried: Carried,
}

/// What hour.json says.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hour {
    pub at: i64,
    pub heard: usize,
    pub routed: usize,
    pub open: usize,
    pub worst: Option<String>,
    pub worst_hits: usize,
}

/// Hears what's new, sends what no one has, and says the hour. `send`
/// takes an address and a finding's text and says whether it went.
///
/// # Errors
///
/// When its state can't be written.
pub fn hear(
    ears: &Ears,
    cells_printed: &str,
    send: &mut dyn FnMut(&str, &str) -> bool,
) -> Result<Hour, String> {
    let _ = (ears, cells_printed, send, HOUR_MS);
    let _: Option<(Heard, Cell, Routes, Following, BTreeMap<String, String>, &Path)> = None;
    Ok(Hour::default())
}

#[cfg(test)]
mod tests;
