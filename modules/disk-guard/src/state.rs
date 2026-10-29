//! What the guard keeps between runs: files written whole by rename, so a
//! crash leaves the old file or the new one, each carrying a version and
//! any field a newer guard wrote, so going back a release loses nothing.

use crate::clock::Awake;
use crate::pass::Learned;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// The version this guard writes.
pub const VERSION: u32 = 1;

/// What the guard has learned and when it last did its upkeep.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Memory {
    #[serde(default = "version")]
    pub version: u32,
    #[serde(default)]
    pub awake: Awake,
    #[serde(default)]
    pub learned: Learned,
    /// When routine upkeep last ran, in wall seconds.
    #[serde(default)]
    pub upkept: Option<f64>,
    /// How long a pass reaching for the target takes, in seconds,
    /// averaged over recent passes.
    #[serde(default)]
    pub pass_seconds: Option<f64>,
    /// When each source's command last ran, in wall seconds, by name.
    #[serde(default)]
    pub ran: std::collections::BTreeMap<String, f64>,
    /// What each producer's watch said as its command last ran, by name,
    /// so only something new makes it due again.
    #[serde(default)]
    pub said: std::collections::BTreeMap<String, u64>,
    /// The last run linking the store's identical files that finished.
    #[serde(default)]
    pub deduped: Option<crate::dedupe::Deduped>,
    /// The writers frozen now, kept so the next guard thaws them.
    #[serde(default)]
    pub frozen: Vec<crate::writers::Frozen>,
    /// Writers somebody thawed by hand, by pid and start, never frozen
    /// again while they live.
    #[serde(default)]
    pub spared: Vec<(u32, u64)>,
    /// The owners the last look found ended, so one ending since makes
    /// upkeep due, even across a restart.
    #[serde(default)]
    pub owners: Option<Owners>,
    /// What a newer guard wrote that this one doesn't know.
    #[serde(flatten)]
    pub rest: serde_json::Map<String, serde_json::Value>,
}

impl Default for Memory {
    fn default() -> Self {
        Memory {
            version: VERSION,
            awake: Awake::default(),
            learned: Learned::default(),
            upkept: None,
            pass_seconds: None,
            ran: std::collections::BTreeMap::new(),
            said: std::collections::BTreeMap::new(),
            deduped: None,
            frozen: Vec::new(),
            spared: Vec::new(),
            owners: None,
            rest: serde_json::Map::new(),
        }
    }
}

/// What has ended, as the guard last saw it: the boot it ran in, the guard
/// it was, and the trees whose builds were spent.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Owners {
    pub boot: String,
    pub guard: Option<PathBuf>,
    pub spent: BTreeSet<PathBuf>,
}

fn version() -> u32 {
    VERSION
}

/// Writes `value` to `path` whole: to a file beside it, flushed to disk,
/// then renamed over it, and the directory flushed so the rename lasts.
pub fn save<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let (Some(dir), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no file to save to",
        ));
    };
    // Named for the process, since a command and the daemon both write
    // the status file, and one's rename must never take the other's
    // half-written file.
    let mut beside = std::ffi::OsString::from(".");
    beside.push(name);
    beside.push(format!(".{}.new", std::process::id()));
    let beside = dir.join(beside);
    let text = serde_json::to_vec(value).map_err(io::Error::other)?;
    let mut file = fs::File::create(&beside)?;
    file.write_all(&text)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&beside, path)?;
    fs::File::open(dir)?.sync_all()
}

/// Reads what `path` holds, or nothing when there's no file. A file that
/// won't read is an error, for the caller to set aside.
pub fn load<T: DeserializeOwned>(path: &Path) -> io::Result<Option<T>> {
    let text = match fs::read(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    serde_json::from_slice(&text)
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}
