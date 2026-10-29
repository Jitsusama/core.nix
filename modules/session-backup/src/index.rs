//! The index the logs are mined from: one JSON line per log sent, saying
//! what the session was without opening its sealed copy. Mining reads the
//! index first and fetches only the logs it names.
//!
//! A row is read from the log as it is sent: its session's id, folder and
//! start from the log's first line, its end from its last entry, its name
//! from the last one pi gave it, the models it answered with, how many
//! turns it took and what they cost, and its quest from the harness's
//! record of the session where one is kept. Each run that sends logs, or
//! finds logs sent before there was an index, sends their rows sealed as
//! one more object, `<host>/index/<run>.jsonl.zst.age`, and keeps them on
//! the machine in the index file, which says which logs have a row.

use std::collections::{BTreeSet, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::names::Settled;

/// The index on the machine, at one path.
pub struct Index {
    path: PathBuf,
    named: HashSet<String>,
}

impl Index {
    /// The index at `path`, empty when there is none yet. A line that isn't
    /// a row, as a run killed mid-write leaves, is passed over, so its log
    /// is read again next time.
    pub fn open(path: &Path) -> Result<Index, String> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(why) => return Err(format!("can't read the index {}: {why}", path.display())),
        };
        let named = text
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter_map(|row| row.get("name").and_then(Value::as_str).map(str::to_owned))
            .collect();
        Ok(Index {
            path: path.to_owned(),
            named,
        })
    }

    /// Whether the log sent under this name has a row.
    pub fn has(&self, name: &str) -> bool {
        self.named.contains(name)
    }

    /// Keeps `rows`, each a line of its own, once their sealed copy is sent.
    pub fn keep(&mut self, rows: &[Value]) -> Result<(), String> {
        if let Some(state) = self.path.parent() {
            fs::create_dir_all(state)
                .map_err(|why| format!("can't make {}: {why}", state.display()))?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|why| format!("can't open the index {}: {why}", self.path.display()))?;
        for row in rows {
            writeln!(file, "{row}")
                .map_err(|why| format!("can't write the index {}: {why}", self.path.display()))?;
            if let Some(name) = row.get("name").and_then(Value::as_str) {
                self.named.insert(name.to_owned());
            }
        }
        file.sync_data()
            .map_err(|why| format!("can't write the index {}: {why}", self.path.display()))
    }
}

/// The row for a log sent as `log.name`, read from the log, with its quest
/// from the session's record in `records` when one is kept there.
pub fn row(log: &Settled, records: Option<&Path>) -> Result<Value, String> {
    let file =
        File::open(&log.path).map_err(|why| format!("can't read {}: {why}", log.path.display()))?;
    let mut id = None;
    let mut folder = None;
    let mut started = None;
    let mut ended = None;
    let mut title = None;
    let mut models = BTreeSet::new();
    let mut turns = 0u64;
    let mut cost = 0f64;
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|why| format!("can't read {}: {why}", log.path.display()))?;
        // A line cut short, as a session killed mid-write leaves, says
        // nothing, and the rest of the log still does.
        let Ok(entry) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let text = |key: &str| entry.get(key).and_then(Value::as_str).map(str::to_owned);
        if let Some(at) = text("timestamp") {
            ended = Some(at);
        }
        match entry.get("type").and_then(Value::as_str) {
            Some("session") if id.is_none() => {
                id = text("id");
                folder = text("cwd");
                started = text("timestamp");
            }
            Some("session_info") => {
                if let Some(name) = text("name") {
                    title = Some(name);
                }
            }
            Some("message") => {
                let message = &entry["message"];
                if message.get("role").and_then(Value::as_str) == Some("assistant") {
                    turns += 1;
                    if let Some(model) = message.get("model").and_then(Value::as_str) {
                        models.insert(model.to_owned());
                    }
                    cost += message["usage"]["cost"]["total"].as_f64().unwrap_or(0.0);
                }
            }
            _ => {}
        }
    }
    let quest = records.zip(id.as_deref()).and_then(|(dir, id)| quest(dir, id));
    Ok(json!({
        "name": log.name,
        "id": id,
        "folder": folder,
        "started": started,
        "ended": ended,
        "title": title,
        "quest": quest,
        "models": models,
        "turns": turns,
        // To the cent, as a bill says it.
        "cost": (cost * 100.0).round() / 100.0,
        "bytes": log.bytes,
    }))
}

/// The quest the harness's record of session `id` names, if it keeps one.
fn quest(records: &Path, id: &str) -> Option<String> {
    let text = fs::read_to_string(records.join(format!("{id}.json"))).ok()?;
    let record: Value = serde_json::from_str(&text).ok()?;
    record.get("quest")?.as_str().map(str::to_owned)
}
