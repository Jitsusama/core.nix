//! The Home Network's rows as the socket serves them, ready to draw
//! (ready-rows' wire): made once in each vocabulary a listener asks for,
//! the launcher's words.json and its number settings, both read only from
//! the Nix store, and made again with each read of the network, so a
//! watch is told only the rows that changed.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use ready_rows::kept::Kept;
use ready_rows::numbers::Numbers;
use serde_json::{Value, json};

use crate::model::HomeNetwork;
use crate::rows::{self, Words};

/// The kind every row's record is.
pub const ROW: &str = "row";

/// Where the vocabularies read live.
const STORE: &str = "/nix/store/";

/// A vocabulary's rows' records, by key, each `{ key, kind: "row", row }`.
pub type Made = BTreeMap<String, Value>;

/// Where a vocabulary's words and number settings are read from.
pub type Said = (PathBuf, Option<PathBuf>);

/// The rows made in one vocabulary.
struct Vocabulary {
    words: Words,
    numbers: Numbers,
    kept: Kept,
    made: Made,
}

impl Vocabulary {
    /// Makes its rows again from the network, each keeping its spaced
    /// order where it can; none while there's no network yet.
    fn make(&mut self, network: Option<&HomeNetwork>) {
        let rows = network.map_or_else(Vec::new, |network| {
            rows::rows(network, &self.words, &self.numbers)
        });
        self.kept.take(rows, Vec::new());
        self.made = self
            .kept
            .all()
            .filter_map(|record| Some((record["key"].as_str()?.to_owned(), record.clone())))
            .collect();
    }
}

/// Every vocabulary asked for, and where they may be read from.
pub struct Vocabularies {
    store: PathBuf,
    held: HashMap<Said, Vocabulary>,
}

impl Default for Vocabularies {
    fn default() -> Self {
        Vocabularies::within(PathBuf::from(STORE))
    }
}

impl Vocabularies {
    /// Vocabularies read only from under `store`, the Nix store but in
    /// tests.
    pub fn within(store: PathBuf) -> Vocabularies {
        Vocabularies {
            store,
            held: HashMap::new(),
        }
    }

    /// The vocabulary at `words`, said in the number settings at
    /// `numbers` if any, read the first time it's asked for and its rows
    /// made from `network`; or why it can't be read.
    pub fn rows_in(
        &mut self,
        words: &str,
        numbers: Option<&str>,
        network: Option<&HomeNetwork>,
    ) -> Result<Said, String> {
        let said = (
            self.in_store(words)?,
            numbers.map(|numbers| self.in_store(numbers)).transpose()?,
        );
        if self.held.contains_key(&said) {
            return Ok(said);
        }
        let numbers = match &said.1 {
            Some(path) => Numbers::read(&read(path)?),
            None => Numbers::default(),
        };
        let mut vocabulary = Vocabulary {
            words: Words::read(&read(&said.0)?)?,
            numbers,
            kept: Kept::default(),
            made: Made::new(),
        };
        vocabulary.make(network);
        self.held.insert(said.clone(), vocabulary);
        Ok(said)
    }

    /// The rows made in a vocabulary read before.
    pub fn made(&self, said: &Said) -> Option<&Made> {
        self.held.get(said).map(|vocabulary| &vocabulary.made)
    }

    /// Makes every vocabulary's rows again from the network, answering
    /// with what each held before.
    pub fn make(&mut self, network: &HomeNetwork) -> HashMap<Said, Made> {
        self.held
            .iter_mut()
            .map(|(said, vocabulary)| {
                let before = std::mem::take(&mut vocabulary.made);
                vocabulary.make(Some(network));
                (said.clone(), before)
            })
            .collect()
    }

    /// Lets go of every vocabulary nobody watches, so a shell's words
    /// from before a switch aren't made again for nobody.
    pub fn keep(&mut self, watched: impl Fn(&Said) -> bool) {
        self.held.retain(|said, _| watched(said));
    }

    /// A file's path, once it's found to be in the store.
    fn in_store(&self, path: &str) -> Result<PathBuf, String> {
        let found = Path::new(path)
            .canonicalize()
            .map_err(|error| format!("{path} can't be found: {error}"))?;
        if !found.starts_with(&self.store) || !found.is_file() {
            return Err(format!("{path} isn't a file in {}", self.store.display()));
        }
        Ok(found)
    }
}

/// A file in the store, read whole as JSON.
fn read(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("{} can't be read: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("{} isn't JSON: {error}", path.display()))
}

/// Whether an ask is for rows: one naming the row kind or a place.
pub fn asks_rows(asked: &Value) -> bool {
    let kind = &asked["kind"];
    kind == ROW
        || kind
            .as_array()
            .is_some_and(|kinds| kinds.iter().any(|kind| kind == ROW))
        || asked.get("place").is_some()
}

/// Why an ask for rows names a kind or place this socket doesn't keep,
/// if it does.
pub fn unkept(asked: &Value) -> Option<String> {
    let kinds: Vec<&Value> = match &asked["kind"] {
        Value::Null => vec![],
        Value::Array(kinds) => kinds.iter().collect(),
        kind => vec![kind],
    };
    if let Some(kind) = kinds.iter().find(|kind| **kind != ROW) {
        return Some(format!("no kind {kind} here"));
    }
    match asked.get("place").and_then(Value::as_str) {
        Some(place) if place != rows::PLACE => Some(format!("no place {place} here")),
        _ => None,
    }
}

/// The records a watch is told between two makings: those that read
/// differently, and the keys gone, among the keys it wants.
pub fn between(
    before: &Made,
    now: &Made,
    wanted: impl Fn(&str) -> bool,
) -> (Vec<Value>, Vec<String>) {
    let changed = now
        .iter()
        .filter(|(key, row)| wanted(key) && before.get(*key) != Some(*row))
        .map(|(_, row)| row.clone())
        .collect();
    let gone = before
        .keys()
        .filter(|key| wanted(key) && !now.contains_key(*key))
        .cloned()
        .collect();
    (changed, gone)
}

/// An error answer, by the interface's codes.
pub fn error(id: &Value, code: &str, message: &str) -> Value {
    json!({"id": id, "error": {"code": code, "message": message}})
}
