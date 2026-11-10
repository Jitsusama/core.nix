//! The passwords daemon's own logic, apart from its sockets and processes
//! so its tests reach it: what a client asks ([`Ask`]), what a password
//! manager says back ([`Said`]), and the entries held of every manager
//! ([`Held`]), none of them holding a secret.
//!
//! The interface is version 1 of the shell's daemons
//! (RSCH-20261029-0E8050, "Daemons: One Interface"): JSON lines, each ask
//! with an `id` echoed in its answers. Its one kind is `entry`, keyed
//! `entry:<manager>:<the manager's id>`.
//!
//! An entry is what a password manager keeps of one thing: a login, a
//! card, a note, an identity or a key, each by its `category`. Its
//! details are each a row named by its label, and a hidden one (a
//! password, a card's number, a one-time code's seed) never carries its
//! value here: a manager that gives one has it taken off before anything
//! holds the entry ([`Entry::without_secrets`]), so nothing a client
//! lists, gets, searches or watches holds a secret. A secret is asked of
//! its manager only to copy or show it, and handed on as it comes.
//!
//! An audit, asked only by Joel (`asked`), reads every password of every
//! manager at once to find those shared and those breached (audit.rs);
//! what it keeps of each entry is its `health`, two counts laid on its
//! record, until a list says the entry changed. What the checks found
//! outlives the daemon ([`Held::kept`]): when the last ended, since when
//! they've found trouble, and when one found none after there had been
//! some, with each troubled entry's two counts, so For You's one to-do
//! for them (Passwords.qml) stands through a reboot. Never a hash and
//! never a password.

use std::collections::{BTreeMap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

pub mod audit;
pub mod cli;
pub mod rows;

use audit::Health;

/// The interface's version this speaks.
pub const VERSION: u64 = 1;

/// The kind of record this daemon keeps.
pub const KIND: &str = "entry";

/// The kind each entry's ready row is served as, to list and watch
/// (rows.rs).
pub const ROW: &str = "row";

/// The kind each row's drawer is served as, by the key its row names,
/// to list and watch only while it shows (rows.rs).
pub const DRAWER: &str = "drawer";

/// What a list or a watch is of: the entries, their ready rows, or the
/// drawers those rows name, by key, which the launcher watches only while
/// one shows. Asked for no keys, drawers are none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Of {
    Entries,
    Rows,
    Drawers(Vec<String>),
}

/// The domain it answers for, as hello says it.
pub const DOMAIN: &str = "passwords";

/// How many a search answers when it doesn't say.
pub const SEARCHED: usize = 50;

/// An entry's key, by its manager and the manager's own id for it.
pub fn key(manager: &str, id: &str) -> String {
    format!("entry:{manager}:{id}")
}

/// The manager and its id that an entry's key names, or none for a key of
/// another kind.
pub fn named(key: &str) -> Option<(&str, &str)> {
    let rest = key.strip_prefix("entry:")?;
    let (manager, id) = rest.split_once(':')?;
    (!manager.is_empty() && !id.is_empty()).then_some((manager, id))
}

/// What a client asked, one line of it.
#[derive(Clone, Debug, PartialEq)]
pub enum Ask {
    Hello {
        id: Value,
    },
    /// Every entry held, at most `limit`; read from the managers first
    /// when none is held yet, or `fresh` asks it. `asked` says Joel asked,
    /// by stepping into Passwords, so a manager may ask him to let it in.
    /// `held` answers with what's held alone and asks no manager anything,
    /// for what must never wake one, as zsh's completion mustn't.
    List {
        id: Value,
        limit: Option<usize>,
        fresh: bool,
        asked: bool,
        held: bool,
        patience: u64,
        /// Whether it asks for the entries, their rows or their drawers.
        of: Of,
    },
    /// Entries by key with every detail, read from their managers when
    /// held only in summary, unless `held` asks for what's held alone.
    Get {
        id: Value,
        keys: Vec<String>,
        asked: bool,
        held: bool,
        patience: u64,
    },
    /// Entries as they change, or their rows or drawers where `of` asks,
    /// and how each manager stands.
    Watch {
        id: Value,
        of: Of,
    },
    Unwatch {
        id: Value,
    },
    /// Those whose name, username, website or tags the words find, best
    /// first, at most `limit`.
    Search {
        id: Value,
        words: String,
        limit: usize,
    },
    /// One detail's value, its secret included, asked of its manager and
    /// answered to this client alone. Without a detail, the password.
    Reveal {
        id: Value,
        key: String,
        detail: Option<String>,
    },
    /// One detail's value put on the clipboard, marked as sensitive and
    /// cleared again after `clears` milliseconds unless something else was
    /// copied meanwhile. Without a detail, the password.
    Copy {
        id: Value,
        key: String,
        detail: Option<String>,
    },
    /// The one-time code an entry gives now, and how long it lasts.
    Code {
        id: Value,
        key: String,
    },
    /// Ask a manager to let the daemon in, as 1Password asks Joel to
    /// approve its command the first time.
    LetIn {
        id: Value,
        manager: String,
    },
    Health {
        id: Value,
    },
    /// Every password audited for reuse and breaches, only when `asked`
    /// says Joel asked: it reads every secret, so nothing else may.
    Audit {
        id: Value,
        asked: bool,
    },
    /// A kind this daemon doesn't keep.
    UnknownKind {
        id: Value,
        kind: String,
    },
    /// An ask this daemon doesn't know, or a line that isn't one.
    Unknown {
        id: Value,
        ask: String,
    },
}

/// How long an ask waits on a manager when it doesn't say: a manager's
/// command can take a second or two, and one waiting on Joel to let it in
/// longer, which a client says by its own patience.
pub const PATIENCE: u64 = 10_000;

/// What a line from a client asks.
pub fn ask(line: &str) -> Ask {
    let Ok(said) = serde_json::from_str::<Value>(line) else {
        return Ask::Unknown {
            id: Value::Null,
            ask: String::new(),
        };
    };
    let id = said.get("id").cloned().unwrap_or(Value::Null);
    let asked = said.get("ask").and_then(Value::as_str).unwrap_or("");
    let text = |name: &str| said.get(name).and_then(Value::as_str).map(str::to_owned);
    let flag = |name: &str| said.get(name).and_then(Value::as_bool) == Some(true);
    let count = |name: &str| {
        said.get(name)
            .and_then(Value::as_u64)
            .and_then(|count| usize::try_from(count).ok())
    };
    let patience = said
        .get("patience")
        .and_then(Value::as_u64)
        .unwrap_or(PATIENCE);
    let keys = |prefix: &str| -> Vec<String> {
        said.get("keys")
            .and_then(Value::as_array)
            .map(|keys| {
                keys.iter()
                    .filter_map(Value::as_str)
                    .filter(|key| key.starts_with(prefix))
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    let kind = said.get("kind").and_then(Value::as_str).unwrap_or(KIND);
    let of = match kind {
        KIND => Of::Entries,
        ROW if matches!(asked, "list" | "watch") => Of::Rows,
        DRAWER if matches!(asked, "list" | "watch") => Of::Drawers(keys("drawer:entry:")),
        _ if matches!(asked, "list" | "watch" | "search") => {
            return Ask::UnknownKind {
                id,
                kind: kind.to_owned(),
            };
        }
        _ => Of::Entries,
    };
    match asked {
        "hello" => Ask::Hello { id },
        "list" => Ask::List {
            id,
            limit: count("limit"),
            fresh: flag("fresh"),
            asked: flag("asked"),
            held: flag("held"),
            patience,
            of,
        },
        "get" => Ask::Get {
            id,
            keys: keys("entry:")
                .into_iter()
                .filter(|key| named(key).is_some())
                .collect(),
            asked: flag("asked"),
            held: flag("held"),
            patience,
        },
        "watch" => Ask::Watch { id, of },
        "unwatch" => Ask::Unwatch { id },
        "search" => Ask::Search {
            id,
            words: text("words").unwrap_or_default(),
            limit: count("limit").unwrap_or(SEARCHED),
        },
        "reveal" | "copy" | "code" => {
            let Some(key) = text("key").filter(|key| named(key).is_some()) else {
                return Ask::Unknown {
                    id,
                    ask: format!("{asked} without an entry's key"),
                };
            };
            let detail = text("detail");
            match asked {
                "reveal" => Ask::Reveal { id, key, detail },
                "copy" => Ask::Copy { id, key, detail },
                _ => Ask::Code { id, key },
            }
        }
        "let-in" => Ask::LetIn {
            id,
            manager: text("manager").unwrap_or_default(),
        },
        "health" => Ask::Health { id },
        "audit" => Ask::Audit {
            id,
            asked: flag("asked"),
        },
        other => Ask::Unknown {
            id,
            ask: other.to_owned(),
        },
    }
}

/// An error answer, by the interface's codes.
pub fn error(id: &Value, code: &str, message: &str) -> Value {
    json!({"id": id, "error": {"code": code, "message": message}})
}

/// How a manager stands, as the interface names a source's state.
#[derive(Clone, Debug, PartialEq)]
pub enum Standing {
    Starting,
    Answering,
    /// It can't read until Joel lets it in, the reason saying how.
    SignedOut(String),
    /// It can't read at all, the reason saying why, plainly.
    Refused(String),
}

impl Standing {
    /// As the interface says it: `{"state", "reason"}`.
    pub fn said(&self) -> Value {
        match self {
            Standing::Starting => json!({"state": "starting"}),
            Standing::Answering => json!({"state": "answering"}),
            Standing::SignedOut(reason) => json!({"state": "signed-out", "reason": reason}),
            Standing::Refused(reason) => json!({"state": "refused", "reason": reason}),
        }
    }
}

/// What a manager said, one line of its protocol.
#[derive(Clone, Debug, PartialEq)]
pub enum Said {
    /// How it stands now, said unasked or with an answer.
    Stands(Standing),
    /// The answer to one of its asks, by the id it was asked with.
    Answer { id: u64, answer: Map<String, Value> },
    /// A line it says that this doesn't read.
    Other,
}

/// What a manager's line says.
pub fn said(line: &str) -> Said {
    let Ok(Value::Object(mut said)) = serde_json::from_str::<Value>(line) else {
        return Said::Other;
    };
    if let Some(id) = said.get("id").and_then(Value::as_u64) {
        said.remove("id");
        return Said::Answer { id, answer: said };
    }
    standing(&said).map_or(Said::Other, Said::Stands)
}

/// How a manager says it stands, in a line of its own or beside an
/// answer: its `state` and `reason`.
pub fn standing(said: &Map<String, Value>) -> Option<Standing> {
    let reason = || {
        said.get("reason")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    match said.get("state").and_then(Value::as_str)? {
        "starting" => Some(Standing::Starting),
        "answering" => Some(Standing::Answering),
        "signed-out" => Some(Standing::SignedOut(reason())),
        "refused" => Some(Standing::Refused(reason())),
        _ => None,
    }
}

/// One entry as a manager gives it, its secrets already taken off.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry(Map<String, Value>);

impl Entry {
    /// An entry a manager gave, with every hidden detail's value taken
    /// off, or none for one with no id.
    pub fn without_secrets(given: &Value) -> Option<Entry> {
        let Value::Object(given) = given else {
            return None;
        };
        let id = given.get("id").and_then(Value::as_str)?;
        if id.is_empty() {
            return None;
        }
        let mut entry = given.clone();
        if let Some(Value::Array(details)) = entry.get_mut("details") {
            for detail in details.iter_mut() {
                if let Value::Object(detail) = detail {
                    let hidden = detail.get("hidden").and_then(Value::as_bool) == Some(true)
                        || matches!(
                            detail.get("type").and_then(Value::as_str),
                            Some("hidden" | "code")
                        );
                    if hidden {
                        detail.remove("value");
                        detail.insert("hidden".to_owned(), Value::Bool(true));
                    }
                }
            }
        }
        Some(Entry(entry))
    }

    pub fn id(&self) -> &str {
        self.0.get("id").and_then(Value::as_str).unwrap_or("")
    }

    /// Whether it holds every detail, rather than the summary a list gives.
    pub fn detailed(&self) -> bool {
        self.0.get("details").is_some_and(Value::is_array)
    }

    /// A detail it holds by its id, or, given none, its password: the one
    /// whose purpose is a password, else the first hidden one.
    pub fn detail(&self, wanted: Option<&str>) -> Option<&Map<String, Value>> {
        let details = self.0.get("details").and_then(Value::as_array)?;
        let each = || details.iter().filter_map(Value::as_object);
        match wanted {
            Some(wanted) => each().find(|detail| text(detail, "id") == Some(wanted)),
            None => each()
                .find(|detail| text(detail, "purpose") == Some("password"))
                .or_else(|| {
                    each().find(|detail| {
                        detail.get("hidden").and_then(Value::as_bool) == Some(true)
                            && text(detail, "type") != Some("code")
                    })
                }),
        }
    }

    /// Its one-time code's detail, if it has one.
    pub fn code(&self) -> Option<&Map<String, Value>> {
        self.0
            .get("details")
            .and_then(Value::as_array)?
            .iter()
            .filter_map(Value::as_object)
            .find(|detail| text(detail, "type") == Some("code"))
    }

    /// The words a search reads it by: its title, username, websites'
    /// addresses and tags, and its category.
    fn words(&self) -> Vec<String> {
        let mut words = Vec::new();
        for name in ["title", "username", "category"] {
            if let Some(text) = self.0.get(name).and_then(Value::as_str) {
                words.push(text.to_owned());
            }
        }
        for website in self
            .0
            .get("websites")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(address) = website.get("address").and_then(Value::as_str) {
                words.push(address.to_owned());
            }
        }
        for tag in self
            .0
            .get("tags")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(tag) = tag.as_str() {
                words.push(tag.to_owned());
            }
        }
        words
    }

    /// As a client reads it, keyed and stamped.
    pub fn record(&self, manager: &str, heard: u64) -> Value {
        let mut record = self.0.clone();
        record.insert("key".to_owned(), Value::String(key(manager, self.id())));
        record.insert("kind".to_owned(), Value::String(KIND.to_owned()));
        record.insert("manager".to_owned(), Value::String(manager.to_owned()));
        record.insert("heard".to_owned(), json!(heard));
        Value::Object(record)
    }
}

fn text<'a>(map: &'a Map<String, Value>, name: &str) -> Option<&'a str> {
    map.get(name).and_then(Value::as_str)
}

/// The version of what's kept of the checks this writes and reads.
pub const KEPT: u64 = 1;

/// What the checks of every password found over time, each a time in
/// milliseconds since the epoch: when the last ended, when one first found
/// a password breached or reused while that has gone on, and when one
/// found none after there had been some. Never a secret.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Checked {
    pub at: Option<u64>,
    pub since: Option<u64>,
    pub cleared: Option<u64>,
}

/// What's held of one manager: its entries by id, in the order it gave
/// them, when each was heard, and how it stands.
#[derive(Clone, Debug)]
struct Kept {
    entries: BTreeMap<String, (Entry, u64)>,
    order: Vec<String>,
    listed: bool,
    standing: Standing,
}

/// Every manager's entries, none with a secret, how each stands, each
/// entry's health as the last audit found it, by its key, what the checks
/// found over time, and whether that's changed since it was last kept.
#[derive(Clone, Debug)]
pub struct Held {
    managers: Vec<String>,
    kept: HashMap<String, Kept>,
    health: HashMap<String, Health>,
    checked: Checked,
    unsaved: bool,
}

impl Held {
    /// Nothing held yet of these managers, each starting.
    pub fn of(managers: &[String]) -> Held {
        Held {
            managers: managers.to_vec(),
            kept: managers
                .iter()
                .map(|manager| {
                    (
                        manager.clone(),
                        Kept {
                            entries: BTreeMap::new(),
                            order: Vec::new(),
                            listed: false,
                            standing: Standing::Starting,
                        },
                    )
                })
                .collect(),
            health: HashMap::new(),
            checked: Checked::default(),
            unsaved: false,
        }
    }

    pub fn managers(&self) -> &[String] {
        &self.managers
    }

    /// Whether a manager has given its list since it started.
    pub fn listed(&self, manager: &str) -> bool {
        self.kept.get(manager).is_some_and(|kept| kept.listed)
    }

    pub fn stand(&mut self, manager: &str, standing: Standing) {
        if let Some(kept) = self.kept.get_mut(manager) {
            kept.standing = standing;
        }
    }

    pub fn standing(&self, manager: &str) -> Option<&Standing> {
        self.kept.get(manager).map(|kept| &kept.standing)
    }

    /// Takes a manager's whole list, keeping what was read in full of an
    /// entry its list gives unchanged, and says each key that changed and
    /// each that's gone.
    pub fn list(&mut self, manager: &str, given: &[Value], heard: u64) -> Changes {
        let Some(kept) = self.kept.get_mut(manager) else {
            return Changes::default();
        };
        let mut changes = Changes::default();
        let mut entries = BTreeMap::new();
        let mut order = Vec::new();
        for entry in given.iter().filter_map(Entry::without_secrets) {
            let id = entry.id().to_owned();
            if entries.contains_key(&id) {
                continue;
            }
            // An entry whose version moved may have a new password, so
            // what an audit found of the old one goes.
            if kept
                .entries
                .get(&id)
                .is_some_and(|(was, _)| was.0.get("version") != entry.0.get("version"))
                && self.health.remove(&key(manager, &id)).is_some()
            {
                self.unsaved = true;
            }
            let kept_whole = kept.entries.get(&id).filter(|(was, _)| {
                was.detailed()
                    && was.0.get("version") == entry.0.get("version")
                    && entry.0.get("version").is_some()
            });
            let (entry, heard) = if let Some((was, at)) = kept_whole {
                (was.clone(), *at)
            } else {
                if kept.entries.get(&id).map(|(was, _)| was) != Some(&entry) {
                    changes.changed.push(key(manager, &id));
                }
                (entry, heard)
            };
            order.push(id.clone());
            entries.insert(id, (entry, heard));
        }
        for id in kept.entries.keys() {
            if !entries.contains_key(id) {
                changes.gone.push(key(manager, id));
            }
        }
        // What a check found of an entry its manager no longer lists goes,
        // whether it was held before or only kept from before the daemon
        // started.
        let before = self.health.len();
        self.health.retain(|held, _| {
            named(held).is_none_or(|(of, id)| of != manager || entries.contains_key(id))
        });
        self.unsaved |= self.health.len() != before;
        kept.entries = entries;
        kept.order = order;
        kept.listed = true;
        changes
    }

    /// Takes one entry read in full, saying whether it changed.
    pub fn entry(&mut self, manager: &str, given: &Value, heard: u64) -> Option<String> {
        let kept = self.kept.get_mut(manager)?;
        let entry = Entry::without_secrets(given)?;
        let id = entry.id().to_owned();
        if kept.entries.get(&id).map(|(was, _)| was) == Some(&entry) {
            return None;
        }
        if kept
            .entries
            .get(&id)
            .is_some_and(|(was, _)| was.0.get("version") != entry.0.get("version"))
            && self.health.remove(&key(manager, &id)).is_some()
        {
            self.unsaved = true;
        }
        if !kept.entries.contains_key(&id) {
            kept.order.push(id.clone());
        }
        kept.entries.insert(id.clone(), (entry, heard));
        Some(key(manager, &id))
    }

    /// An entry held, by its key.
    pub fn get(&self, key: &str) -> Option<&Entry> {
        let (manager, id) = named(key)?;
        self.kept
            .get(manager)?
            .entries
            .get(id)
            .map(|(entry, _)| entry)
    }

    /// An entry held as a client reads it, by its key.
    pub fn record(&self, key: &str) -> Option<Value> {
        let (manager, id) = named(key)?;
        let (entry, heard) = self.kept.get(manager)?.entries.get(id)?;
        Some(self.dressed(entry, manager, *heard))
    }

    /// An entry as a client reads it, with its health where an audit
    /// found it.
    fn dressed(&self, entry: &Entry, manager: &str, heard: u64) -> Value {
        let mut record = entry.record(manager, heard);
        if let Some(health) = self.health.get(&key(manager, entry.id())) {
            record["health"] = health.said();
        }
        record
    }

    /// Takes what an audit of these managers found, ending `at`, in place
    /// of what the last one did, and says each key whose health changed.
    /// One finding any entry breached or reused starts what's been found
    /// since, or keeps it; one finding none after there had been some
    /// clears it, unless it couldn't ask about breaches (`whole` false),
    /// since then it can't say none is breached.
    pub fn audited(
        &mut self,
        managers: &[String],
        found: HashMap<String, Health>,
        at: u64,
        whole: bool,
    ) -> Vec<String> {
        let of_these = |key: &str| {
            named(key).is_some_and(|(manager, _)| managers.iter().any(|one| one == manager))
        };
        let mut changed: Vec<String> = self
            .health
            .keys()
            .filter(|key| of_these(key.as_str()) && !found.contains_key(*key))
            .cloned()
            .collect();
        self.health.retain(|key, _| !of_these(key.as_str()));
        for (key, health) in found {
            changed.push(key.clone());
            self.health.insert(key, health);
        }
        changed.sort();
        self.checked.at = Some(at);
        if self.health.values().any(Health::troubled) {
            self.checked.since.get_or_insert(at);
            self.checked.cleared = None;
        } else if whole && self.checked.since.is_some() {
            self.checked.since = None;
            self.checked.cleared = Some(at);
        }
        self.unsaved = true;
        changed
    }

    /// What the checks found, as the `health` ask answers it: when the
    /// last ended, since when trouble's been found and when it cleared,
    /// each null where it hasn't, and how many entries are breached, how
    /// many reused and how many either, by what's held of each now, which
    /// falls as a list finds entries changed.
    pub fn checked(&self) -> Value {
        let counted =
            |which: fn(&Health) -> bool| self.health.values().filter(|h| which(h)).count();
        json!({
            "at": self.checked.at,
            "since": self.checked.since,
            "cleared": self.checked.cleared,
            "breached": counted(Health::breached),
            "reused": counted(|health| health.reused > 0),
            "troubled": counted(Health::troubled),
        })
    }

    /// What's kept of the checks between one run of the daemon and the
    /// next: their times, and each troubled entry's health by its key.
    /// Nothing else of an entry, and never a hash or a password.
    pub fn kept(&self) -> Value {
        let health: Map<String, Value> = self
            .health
            .iter()
            .filter(|(_, health)| health.troubled())
            .map(|(key, health)| (key.clone(), health.said()))
            .collect();
        json!({
            "version": KEPT,
            "at": self.checked.at,
            "since": self.checked.since,
            "cleared": self.checked.cleared,
            "health": health,
        })
    }

    /// What's kept, once it has changed since it was last taken, so it's
    /// written only when it differs.
    pub fn unsaved(&mut self) -> Option<Value> {
        std::mem::take(&mut self.unsaved).then(|| self.kept())
    }

    /// Takes what was kept of the checks as the daemon starts, saying
    /// whether it could be read: of an entry of a manager no longer named,
    /// nothing, and of one its manager no longer lists, nothing past its
    /// manager's first list, so it never dresses a record.
    pub fn load(&mut self, kept: &Value) -> bool {
        if kept.get("version").and_then(Value::as_u64) != Some(KEPT) {
            return false;
        }
        let time = |name: &str| kept.get(name).and_then(Value::as_u64);
        self.checked = Checked {
            at: time("at"),
            since: time("since"),
            cleared: time("cleared"),
        };
        let managers = &self.managers;
        self.health = kept
            .get("health")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .filter(|(key, _)| {
                named(key).is_some_and(|(manager, _)| managers.iter().any(|one| one == manager))
            })
            .filter_map(|(key, said)| Some((key.clone(), Health::heard(said)?)))
            .filter(|(_, health)| health.troubled())
            .collect();
        true
    }

    /// An entry's health, as the last audit found it.
    pub fn health(&self, key: &str) -> Option<&Health> {
        self.health.get(key)
    }

    /// Every entry held, manager by manager in their order, each in the
    /// order its manager gave them.
    pub fn records(&self) -> Vec<Value> {
        let mut records = Vec::new();
        for manager in &self.managers {
            let Some(kept) = self.kept.get(manager) else {
                continue;
            };
            for id in &kept.order {
                if let Some((entry, heard)) = kept.entries.get(id) {
                    records.push(self.dressed(entry, manager, *heard));
                }
            }
        }
        records
    }

    /// Those the words find, by their title first, then by anything else
    /// they're read by, each in the order held.
    pub fn search(&self, typed: &str, limit: usize) -> Vec<Value> {
        let words = words(typed);
        if words.is_empty() {
            return Vec::new();
        }
        let mut titled = Vec::new();
        let mut rest = Vec::new();
        for manager in &self.managers {
            let Some(kept) = self.kept.get(manager) else {
                continue;
            };
            for id in &kept.order {
                let Some((entry, heard)) = kept.entries.get(id) else {
                    continue;
                };
                let title = entry.0.get("title").and_then(Value::as_str).unwrap_or("");
                if finds(title, &words) {
                    titled.push(self.dressed(entry, manager, *heard));
                } else if finds(&entry.words().join(" "), &words) {
                    rest.push(self.dressed(entry, manager, *heard));
                }
            }
        }
        titled.extend(rest);
        titled.truncate(limit);
        titled
    }

    /// Every manager's standing, as hello and health say them.
    pub fn standings(&self) -> Value {
        Value::Object(
            self.managers
                .iter()
                .map(|manager| {
                    let standing = self
                        .kept
                        .get(manager)
                        .map_or(Standing::Starting, |kept| kept.standing.clone());
                    (manager.clone(), standing.said())
                })
                .collect(),
        )
    }

    /// The daemon's own state: answering once any manager answers,
    /// starting while every one is, else as the first stands.
    pub fn state(&self) -> &'static str {
        let standings: Vec<&Standing> = self
            .managers
            .iter()
            .filter_map(|manager| self.standing(manager))
            .collect();
        if standings
            .iter()
            .any(|standing| **standing == Standing::Answering)
        {
            "answering"
        } else if standings
            .iter()
            .all(|standing| **standing == Standing::Starting)
        {
            "starting"
        } else if standings
            .iter()
            .any(|standing| matches!(standing, Standing::SignedOut(_)))
        {
            "signed-out"
        } else {
            "refused"
        }
    }
}

/// The keys a manager's answer changed, and those it took away.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Changes {
    pub changed: Vec<String>,
    pub gone: Vec<String>,
}

/// The words typed, each lowercased, for [`finds`].
pub fn words(typed: &str) -> Vec<String> {
    typed
        .split_whitespace()
        .map(str::to_lowercase)
        .filter(|word| !word.is_empty())
        .collect()
}

/// Whether every word starts a word of the text, or, for a website, any
/// part of its host: "hub" finds "GitHub" no more than a name's middle
/// would, but "git" does.
pub fn finds(text: &str, words: &[String]) -> bool {
    let text = text.to_lowercase();
    let starts: Vec<usize> = text
        .char_indices()
        .filter(|(at, c)| {
            c.is_alphanumeric()
                && text[..*at]
                    .chars()
                    .next_back()
                    .is_none_or(|before| !before.is_alphanumeric())
        })
        .map(|(at, _)| at)
        .collect();
    words.iter().all(|word| {
        starts
            .iter()
            .any(|at| text[*at..].starts_with(word.as_str()))
    })
}

/// Milliseconds since the epoch.
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn login(id: &str, title: &str, version: u64) -> Value {
        json!({
            "id": id, "title": title, "category": "login", "version": version,
            "username": "ada", "websites": [{"address": "https://github.com", "primary": true}],
        })
    }

    fn detailed(id: &str, version: u64) -> Value {
        json!({
            "id": id, "title": "GitHub", "category": "login", "version": version,
            "details": [
                {"id": "username", "label": "username", "type": "text",
                 "purpose": "username", "value": "ada"},
                {"id": "password", "label": "password", "type": "hidden",
                 "purpose": "password", "value": "hunter2"},
                {"id": "otp", "label": "one-time password", "type": "code",
                 "value": "otpauth://totp/x?secret=ABC"},
                {"id": "pin", "label": "PIN", "type": "text", "hidden": true, "value": "1234"},
            ],
        })
    }

    #[test]
    fn a_hidden_detail_never_keeps_its_value() {
        let entry = Entry::without_secrets(&detailed("a", 1)).unwrap();
        let said = serde_json::to_string(&entry.record("1password", 0)).unwrap();
        for secret in ["hunter2", "otpauth", "1234"] {
            assert!(!said.contains(secret), "{secret} was kept: {said}");
        }
        assert!(
            said.contains("\"value\":\"ada\""),
            "the username went too: {said}"
        );
        let password = entry.detail(None).unwrap();
        assert_eq!(password.get("hidden"), Some(&Value::Bool(true)));
        assert_eq!(text(password, "id"), Some("password"));
        assert_eq!(entry.code().and_then(|code| text(code, "id")), Some("otp"));
    }

    #[test]
    fn the_password_is_the_hidden_detail_when_none_says_its_purpose() {
        let entry = Entry::without_secrets(&json!({
            "id": "c", "details": [
                {"id": "number", "type": "text", "value": "4111"},
                {"id": "otp", "type": "code"},
                {"id": "cvv", "type": "hidden"},
            ],
        }))
        .unwrap();
        assert_eq!(
            entry.detail(None).and_then(|detail| text(detail, "id")),
            Some("cvv")
        );
        assert_eq!(
            entry.detail(Some("number")).and_then(|d| text(d, "id")),
            Some("number")
        );
    }

    #[test]
    fn an_entry_without_an_id_is_never_held() {
        assert!(Entry::without_secrets(&json!({"title": "No one"})).is_none());
        assert!(Entry::without_secrets(&json!({"id": ""})).is_none());
        assert!(Entry::without_secrets(&json!("text")).is_none());
    }

    #[test]
    fn a_list_says_what_changed_and_what_went() {
        let mut held = Held::of(&["1password".to_owned()]);
        let first = held.list(
            "1password",
            &[login("a", "GitHub", 1), login("b", "Bank", 1)],
            5,
        );
        assert_eq!(first.changed, ["entry:1password:a", "entry:1password:b"]);
        assert_eq!(first.gone, Vec::<String>::new());
        let again = held.list("1password", &[login("a", "GitHub", 1)], 9);
        assert!(again.changed.is_empty(), "{again:?}");
        assert_eq!(again.gone, ["entry:1password:b"]);
        assert!(held.listed("1password"));
    }

    #[test]
    fn a_list_keeps_an_entry_read_in_full_until_its_version_moves() {
        let mut held = Held::of(&["1password".to_owned()]);
        held.list("1password", &[login("a", "GitHub", 1)], 1);
        assert_eq!(
            held.entry("1password", &detailed("a", 1), 2).as_deref(),
            Some("entry:1password:a")
        );
        held.list("1password", &[login("a", "GitHub", 1)], 3);
        assert!(held.get("entry:1password:a").unwrap().detailed());
        let moved = held.list("1password", &[login("a", "GitHub", 2)], 4);
        assert_eq!(moved.changed, ["entry:1password:a"]);
        assert!(!held.get("entry:1password:a").unwrap().detailed());
    }

    #[test]
    fn a_search_finds_titles_first_then_anything_else() {
        let mut held = Held::of(&["1password".to_owned()]);
        held.list(
            "1password",
            &[
                login("a", "Work Mail", 1),
                login("b", "GitHub", 1),
                login("c", "Bank", 1),
            ],
            1,
        );
        let found: Vec<String> = held
            .search("git", 10)
            .iter()
            .map(|record| record["key"].as_str().unwrap().to_owned())
            .collect();
        // Every login here has GitHub's address, but only one is titled so.
        assert_eq!(found[0], "entry:1password:b");
        assert_eq!(found.len(), 3);
        assert_eq!(held.search("hub", 10), Vec::<Value>::new());
        assert_eq!(held.search("", 10), Vec::<Value>::new());
        assert_eq!(held.search("git", 1).len(), 1);
    }

    #[test]
    fn asks_are_read_with_their_defaults() {
        assert_eq!(
            ask(r#"{"id":1,"ask":"list","asked":true}"#),
            Ask::List {
                id: json!(1),
                limit: None,
                fresh: false,
                asked: true,
                held: false,
                patience: PATIENCE,
                of: Of::Entries
            }
        );
        assert_eq!(
            ask(r#"{"id":7,"ask":"watch","kind":"row"}"#),
            Ask::Watch {
                id: json!(7),
                of: Of::Rows
            }
        );
        assert_eq!(
            ask(r#"{"id":9,"ask":"watch","kind":"drawer",
                    "keys":["drawer:entry:onepassword:a","row:entry:onepassword:b"]}"#),
            Ask::Watch {
                id: json!(9),
                of: Of::Drawers(vec!["drawer:entry:onepassword:a".to_owned()])
            },
            "a drawer watch kept a key that names no drawer"
        );
        assert_eq!(
            ask(r#"{"id":10,"ask":"search","kind":"drawer"}"#),
            Ask::UnknownKind {
                id: json!(10),
                kind: "drawer".to_owned()
            }
        );
        assert_eq!(
            ask(r#"{"id":8,"ask":"search","kind":"row"}"#),
            Ask::UnknownKind {
                id: json!(8),
                kind: "row".to_owned()
            }
        );
        assert!(matches!(
            ask(r#"{"id":6,"ask":"list","held":true}"#),
            Ask::List {
                held: true,
                asked: false,
                ..
            }
        ));
        assert_eq!(
            ask(r#"{"id":2,"ask":"copy","key":"entry:1password:a"}"#),
            Ask::Copy {
                id: json!(2),
                key: "entry:1password:a".to_owned(),
                detail: None
            }
        );
        assert!(matches!(
            ask(r#"{"id":3,"ask":"reveal","key":"person:ada"}"#),
            Ask::Unknown { .. }
        ));
        assert!(matches!(
            ask(r#"{"id":4,"ask":"list","kind":"person"}"#),
            Ask::UnknownKind { .. }
        ));
        assert!(matches!(ask("not json"), Ask::Unknown { .. }));
        assert_eq!(
            ask(r#"{"id":5,"ask":"get","keys":["entry:1password:a","nope"]}"#),
            Ask::Get {
                id: json!(5),
                keys: vec!["entry:1password:a".to_owned()],
                asked: false,
                held: false,
                patience: PATIENCE
            }
        );
    }

    #[test]
    fn a_manager_says_answers_and_standings() {
        assert_eq!(
            said(r#"{"state":"signed-out","reason":"Let the launcher in"}"#),
            Said::Stands(Standing::SignedOut("Let the launcher in".to_owned()))
        );
        let Said::Answer { id, answer } = said(r#"{"id":7,"entries":[]}"#) else {
            panic!("an answer wasn't read as one");
        };
        assert_eq!(id, 7);
        assert!(answer.contains_key("entries") && !answer.contains_key("id"));
        assert_eq!(said("{}"), Said::Other);
    }

    #[test]
    fn the_daemon_answers_once_any_manager_does() {
        let mut held = Held::of(&["a".to_owned(), "b".to_owned()]);
        assert_eq!(held.state(), "starting");
        held.stand("a", Standing::SignedOut("Let it in".to_owned()));
        assert_eq!(held.state(), "signed-out");
        held.stand("b", Standing::Answering);
        assert_eq!(held.state(), "answering");
        assert_eq!(held.standings()["a"]["state"], "signed-out");
    }

    #[test]
    fn an_audit_is_asked_only_as_joels_own() {
        assert_eq!(
            ask(r#"{"id":1,"ask":"audit","asked":true}"#),
            Ask::Audit {
                id: json!(1),
                asked: true
            }
        );
        assert!(matches!(
            ask(r#"{"id":2,"ask":"audit"}"#),
            Ask::Audit { asked: false, .. }
        ));
    }

    #[test]
    fn an_entrys_health_is_kept_until_its_version_moves() {
        let mut held = Held::of(&["1password".to_owned()]);
        held.list(
            "1password",
            &[login("a", "GitHub", 1), login("b", "Bank", 1)],
            1,
        );
        let found = HashMap::from([
            (
                "entry:1password:a".to_owned(),
                Health {
                    reused: 1,
                    breached: Some(3),
                },
            ),
            (
                "entry:1password:b".to_owned(),
                Health {
                    reused: 1,
                    breached: None,
                },
            ),
        ]);
        let changed = held.audited(&["1password".to_owned()], found, 10, true);
        assert_eq!(changed, ["entry:1password:a", "entry:1password:b"]);
        let a = held.record("entry:1password:a").unwrap();
        assert_eq!(a["health"], json!({"reused": 1, "breached": 3}));
        assert_eq!(held.records()[1]["health"], json!({"reused": 1}));
        // Listed again unchanged, it keeps its health; its version moved,
        // it may hold another password, so it holds none.
        held.list(
            "1password",
            &[login("a", "GitHub", 1), login("b", "Bank", 2)],
            2,
        );
        assert!(held.health("entry:1password:a").is_some());
        assert!(
            held.record("entry:1password:b")
                .unwrap()
                .get("health")
                .is_none()
        );
        // An audit finding nothing of an entry takes what the last found.
        let changed = held.audited(&["1password".to_owned()], HashMap::new(), 20, true);
        assert_eq!(changed, ["entry:1password:a"]);
        assert!(held.health("entry:1password:a").is_none());
    }

    fn troubled(reused: u64, breached: Option<u64>) -> Health {
        Health { reused, breached }
    }

    #[test]
    fn what_checks_found_starts_holds_and_clears() {
        let managers = ["1password".to_owned()];
        let mut held = Held::of(&managers);
        held.list("1password", &[login("a", "GitHub", 1)], 1);
        assert_eq!(held.checked()["at"], Value::Null);
        let a = "entry:1password:a".to_owned();
        held.audited(
            &managers,
            HashMap::from([(a.clone(), troubled(0, Some(0)))]),
            5,
            true,
        );
        // A check finding nothing before any found trouble clears nothing.
        assert_eq!(
            held.checked(),
            json!({"at": 5, "since": null, "cleared": null,
                   "breached": 0, "reused": 0, "troubled": 0})
        );
        held.audited(
            &managers,
            HashMap::from([(a.clone(), troubled(1, Some(3)))]),
            10,
            true,
        );
        held.audited(
            &managers,
            HashMap::from([(a.clone(), troubled(1, Some(0)))]),
            20,
            true,
        );
        // Trouble found at 10 is still what's been found since, at 20.
        assert_eq!(
            held.checked(),
            json!({"at": 20, "since": 10, "cleared": null,
                   "breached": 0, "reused": 1, "troubled": 1})
        );
        // One that couldn't ask about breaches can't say none is breached.
        held.audited(
            &managers,
            HashMap::from([(a.clone(), troubled(0, None))]),
            30,
            false,
        );
        assert_eq!(held.checked()["since"], 10);
        held.audited(
            &managers,
            HashMap::from([(a, troubled(0, Some(0)))]),
            40,
            true,
        );
        assert_eq!(held.checked()["since"], Value::Null);
        assert_eq!(held.checked()["cleared"], 40);
    }

    #[test]
    fn what_checks_found_is_kept_without_a_secret_and_read_back() {
        let managers = ["1password".to_owned()];
        let mut held = Held::of(&managers);
        held.list(
            "1password",
            &[login("a", "GitHub", 1), login("b", "Bank", 1)],
            1,
        );
        assert!(held.unsaved().is_none(), "nothing was checked yet");
        let found = HashMap::from([
            ("entry:1password:a".to_owned(), troubled(2, Some(1204))),
            ("entry:1password:b".to_owned(), troubled(0, Some(0))),
        ]);
        held.audited(&managers, found, 10, true);
        let kept = held.unsaved().expect("a check is kept");
        // Only the troubled entry, by its two counts.
        assert_eq!(
            kept,
            json!({"version": 1, "at": 10, "since": 10, "cleared": null,
                   "health": {"entry:1password:a": {"reused": 2, "breached": 1204}}})
        );
        assert!(held.unsaved().is_none(), "it was kept already");
        // A daemon starting again reads it, and dresses the entry once its
        // manager lists it; one of a manager no longer named it leaves out.
        let mut again = Held::of(&managers);
        let mut with_another = kept.clone();
        with_another["health"]["entry:gone:z"] = json!({"reused": 1});
        assert!(again.load(&with_another));
        assert_eq!(again.checked()["troubled"], 1);
        again.list("1password", &[login("a", "GitHub", 1)], 2);
        assert_eq!(
            again.record("entry:1password:a").unwrap()["health"],
            json!({"reused": 2, "breached": 1204})
        );
        // A list no longer giving it lets it go, and says it's to be kept.
        again.list("1password", &[login("b", "Bank", 1)], 3);
        assert_eq!(again.checked()["troubled"], 0);
        assert_eq!(again.unsaved().unwrap()["health"], json!({}));
        assert!(!again.load(&json!({"version": 2})));
    }

    #[test]
    fn keys_name_their_manager_and_id() {
        assert_eq!(named(&key("1password", "abc")), Some(("1password", "abc")));
        assert_eq!(named("entry:1password:"), None);
        assert_eq!(named("person:ada"), None);
    }
}
