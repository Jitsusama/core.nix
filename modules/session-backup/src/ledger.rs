//! What was sent: one JSON line per object Drive holds, appended as each
//! lands, so a run stopped at any point loses nothing it had sent.

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// The ledger at one path.
pub struct Ledger {
    path: PathBuf,
    /// Each object sent, by name: its Drive id.
    sent: HashMap<String, String>,
}

impl Ledger {
    /// The ledger at `path`, empty when there is none yet. A line that isn't
    /// an entry, as a run killed mid-write leaves, is passed over: its
    /// object is found on Drive by name next time.
    pub fn open(path: &Path) -> Result<Ledger, String> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(why) => return Err(format!("can't read the ledger {}: {why}", path.display())),
        };
        let sent = text
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter_map(|entry| {
                let field = |key: &str| entry.get(key).and_then(Value::as_str).map(str::to_owned);
                Some((field("name")?, field("id").unwrap_or_default()))
            })
            .collect();
        Ok(Ledger {
            path: path.to_owned(),
            sent,
        })
    }

    /// Whether an object of this name was sent.
    pub fn has(&self, name: &str) -> bool {
        self.sent.contains_key(name)
    }

    /// The Drive id of the object sent under this name, if the ledger has it.
    pub fn id(&self, name: &str) -> Option<&str> {
        self.sent
            .get(name)
            .map(String::as_str)
            .filter(|id| !id.is_empty())
    }

    /// Each object sent, by its name and Drive id, in no order.
    pub fn sent(&self) -> impl Iterator<Item = (&str, &str)> {
        self.sent
            .iter()
            .map(|(name, id)| (name.as_str(), id.as_str()))
    }

    /// How many objects were sent.
    pub fn len(&self) -> usize {
        self.sent.len()
    }

    /// Whether nothing was sent yet.
    pub fn is_empty(&self) -> bool {
        self.sent.is_empty()
    }

    /// Say that Drive holds `name` as `id`, `bytes` long once sealed.
    pub fn record(&mut self, name: &str, id: &str, bytes: u64, at: u64) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|why| format!("can't make {}: {why}", parent.display()))?;
        }
        let line = json!({ "name": name, "id": id, "bytes": bytes, "at": at });
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|why| format!("can't open the ledger {}: {why}", self.path.display()))?;
        writeln!(file, "{line}")
            .and_then(|()| file.sync_data())
            .map_err(|why| format!("can't write the ledger {}: {why}", self.path.display()))?;
        self.sent.insert(name.to_owned(), id.to_owned());
        Ok(())
    }
}
