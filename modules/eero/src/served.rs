//! The Home Network's rows as the socket serves them, ready to draw
//! (ready-rows' wire): made once in each vocabulary a listener asks for,
//! the launcher's words.json and its number settings, both read only from
//! the Nix store, and made again with each read of the network, and when
//! net's record says a different worst, so a watch is told only the rows
//! that changed.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use ready_rows::kept::Kept;
use ready_rows::numbers::Numbers;
use serde_json::{Value, json};

use crate::drawers;
use crate::model::HomeNetwork;
use crate::place;
use crate::rows::{self, Words};

/// The kind every row's record is.
pub const ROW: &str = "row";

/// Where the vocabularies read live.
const STORE: &str = "/nix/store/";

/// A vocabulary's rows' records, by key, each `{ key, kind: "row", row }`.
pub type Made = BTreeMap<String, Value>;

/// Where a vocabulary's words and number settings are read from.
pub type Said = (PathBuf, Option<PathBuf>);

/// The rows made in one vocabulary, and the worst of net's they were
/// made with, so each is known to be behind net on its own.
struct Vocabulary {
    words: Words,
    numbers: Numbers,
    kept: Kept,
    made: Made,
    made_with: Option<String>,
}

impl Vocabulary {
    /// Makes its rows again from the network, each keeping its spaced
    /// order where it can, and the place's own drawer beside them; none
    /// while there's no network yet. The map is offered where there's a
    /// `map` page to open.
    fn make(&mut self, network: Option<&HomeNetwork>, map: Option<&rows::Map>) {
        let (rows, beside) = network.map_or_else(Default::default, |network| {
            (
                rows::rows(network, &self.words, &self.numbers, map),
                std::iter::once(place::record(network, &self.words, &self.numbers))
                    .chain(drawers::records(network, &self.words, &self.numbers))
                    .collect(),
            )
        });
        self.kept.take(rows, beside);
        self.made = self
            .kept
            .all()
            .filter_map(|record| Some((record["key"].as_str()?.to_owned(), record.clone())))
            .collect();
        self.made_with = map.and_then(|map| map.worst).map(str::to_owned);
    }
}

/// The map of the network the rows offer: its page, and net's record it's
/// drawn from (modules/net), read for what's worth a look whenever net
/// has written it anew.
pub struct Mapped {
    pub page: String,
    pub record: PathBuf,
}

/// What net's record said worst, as of when it was written; none written
/// while there's no record.
struct Heard {
    written: Option<SystemTime>,
    worst: Option<String>,
}

/// Every vocabulary asked for, where they may be read from, the map of
/// the network if there's one to open, and what net last said of it.
pub struct Vocabularies {
    store: PathBuf,
    map: Option<Mapped>,
    heard: Option<Heard>,
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
            map: None,
            heard: None,
            held: HashMap::new(),
        }
    }

    /// The same, its rows offering the map of the network at `map`.
    pub fn mapped(self, map: Option<Mapped>) -> Vocabularies {
        Vocabularies { map, ..self }
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
            made_with: None,
        };
        self.hear();
        vocabulary.make(network, self.map_now().as_ref());
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
        self.hear();
        // Heard once for every vocabulary, so each says the same.
        let map = self.map.as_ref().map(|map| rows::Map {
            page: &map.page,
            worst: self.heard.as_ref().and_then(|heard| heard.worst.as_deref()),
        });
        self.held
            .iter_mut()
            .map(|(said, vocabulary)| {
                let before = std::mem::take(&mut vocabulary.made);
                vocabulary.make(Some(network), map.as_ref());
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

impl Vocabularies {
    /// Whether net's record says a different worst than any vocabulary's
    /// rows were last made with, as `make` would read it now. net writes its record
    /// each minute, and it's read again only once it's written anew, a
    /// look at its time costing far less than reading it whole. It's
    /// held against what each vocabulary was made with rather than what
    /// was last heard, since a vocabulary asked for between looks hears
    /// too, and may hear a worst net has since dropped. With no map
    /// nothing is heard and every vocabulary was made with none.
    pub fn net_moved(&mut self) -> bool {
        self.hear();
        let now = self.heard.as_ref().and_then(|heard| heard.worst.as_deref());
        self.held
            .values()
            .any(|vocabulary| vocabulary.made_with.as_deref() != now)
    }

    /// Hears net's record again, if it's been written since it was last
    /// heard. Its time is taken before it's read, so a write between
    /// the two is heard on the next look rather than missed.
    fn hear(&mut self) {
        let Some(map) = &self.map else { return };
        let written = std::fs::metadata(&map.record)
            .and_then(|record| record.modified())
            .ok();
        if self
            .heard
            .as_ref()
            .is_some_and(|heard| heard.written == written)
        {
            return;
        }
        self.heard = Some(Heard {
            written,
            worst: worst(&map.record),
        });
    }

    /// The map's page, with the worst net was last heard to find.
    fn map_now(&self) -> Option<rows::Map<'_>> {
        self.map.as_ref().map(|map| rows::Map {
            page: &map.page,
            worst: self.heard.as_ref().and_then(|heard| heard.worst.as_deref()),
        })
    }
}

/// The worst of what net finds worth a look in its record at `record`,
/// in net's words: the first of its concerns, which net lists worst
/// first, but the internet's ceilings, which are the line's speed rather
/// than a fault (net's own view says them apart). None while there's
/// none, or no record to read, since net may not run here.
fn worst(record: &Path) -> Option<String> {
    let text = std::fs::read_to_string(record).ok()?;
    let record: Value = serde_json::from_str(&text).ok()?;
    record["concerns"]
        .as_array()?
        .iter()
        .find(|concern| concern["kind"] != "ceiling")?["says"]
        .as_str()
        .map(str::to_owned)
}

/// A file in the store, read whole as JSON.
fn read(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("{} can't be read: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("{} isn't JSON: {error}", path.display()))
}

/// The kinds of record served beside each other: the rows, and the
/// place's drawer.
const KINDS: [&str; 2] = [ROW, place::KIND];

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
    if let Some(kind) = kinds
        .iter()
        .find(|kind| !KINDS.iter().any(|kept| *kind == kept))
    {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// net's record as it would be written, at a path of the test's own.
    fn record(name: &str, concerns: &Value) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("eero-served-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("the test's folder is made");
        let path = folder.join(format!("{name}.json"));
        std::fs::write(&path, json!({"concerns": concerns}).to_string())
            .expect("the record is written");
        path
    }

    #[test]
    fn the_worst_is_the_first_net_finds_but_the_internets_ceilings() {
        let path = record(
            "worst",
            &json!([
                {"kind": "ceiling", "says": "The internet is the narrowest point up"},
                {"kind": "lost", "says": "The modem's downstream channel 16 has lost 878"},
                {"kind": "weak", "says": "laptop computer 1 has a weak signal"},
            ]),
        );
        assert_eq!(
            worst(&path).as_deref(),
            Some("The modem's downstream channel 16 has lost 878"),
            "a ceiling is the line's speed, not a fault, as net's own view says it apart"
        );
    }

    #[test]
    fn nothing_is_worst_with_only_ceilings_or_no_record() {
        let path = record(
            "ceilings",
            &json!([{"kind": "ceiling", "says": "The internet is the narrowest point up"}]),
        );
        assert_eq!(
            worst(&path),
            None,
            "the line's speed alone is nothing to look at"
        );
        assert_eq!(
            worst(Path::new("/nonexistent/net/network.json")),
            None,
            "with net not running, the map says what it is"
        );
    }
}
