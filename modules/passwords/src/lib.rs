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

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

pub mod audit;
pub mod cli;
pub mod rows;
pub mod typing;

mod entry;
mod held;

pub use entry::Entry;
pub use held::{Changes, Checked, Held, KEPT, finds, words};

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
/// one shows. Asked for no keys, drawers are none. The entries with the
/// secret asks' rows alone is the Passwords place's one watch where asks
/// are on, so its asks share its connection rather than a second one
/// watching the same folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Of {
    Entries,
    Rows,
    EntriesAndAsks,
    Drawers(Vec<String>),
}

impl Of {
    /// Whether a watch of this is told the entries as they change.
    #[must_use]
    pub fn has_entries(&self) -> bool {
        matches!(self, Self::Entries | Self::EntriesAndAsks)
    }
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
    /// One detail's value typed into the window the launcher opened over,
    /// niri's `window` by its id, once the launcher has let go of the keys
    /// and only while that window still has them (typing.rs says which
    /// windows are refused). Without a detail, the password.
    Type {
        id: Value,
        key: String,
        detail: Option<String>,
        window: u64,
    },
    /// The one-time code an entry gives now, which is refused, as a
    /// reveal is, since the code would be a secret on the pipe.
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
    /// The shell's yes to a secret ask, as its row's act says it, with
    /// how long the line was shown and which opening said it; answered
    /// by the engine (secret-asks), which checks all of it again.
    Release {
        id: Value,
        said: Value,
    },
    /// The shell's no to a secret ask.
    Refuse {
        id: Value,
        said: Value,
    },
    /// Asks' Forget It on a standing yes, which the daemon's wire calls
    /// a revoke.
    Revoke {
        id: Value,
        said: Value,
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

/// The kind a line asks for: a list of kinds is the entries with the
/// asks' rows, or none it keeps; a kind said alone is that kind.
fn kind_of(said: &Value, asks_place: bool) -> String {
    match said.get("kind") {
        Some(Value::Array(kinds)) => {
            let mut named: Vec<&str> = kinds.iter().filter_map(Value::as_str).collect();
            named.sort_unstable();
            if named == [KIND, ROW] && asks_place {
                "entries and asks".to_owned()
            } else {
                named.join(" and ")
            }
        }
        said => said.and_then(Value::as_str).unwrap_or(KIND).to_owned(),
    }
}

/// A type asked: an entry's key, the detail to type, its password when
/// none's named, and niri's id for the window it's to be typed into.
fn type_in(id: Value, said: &Value) -> Ask {
    let text = |name: &str| said.get(name).and_then(Value::as_str).map(str::to_owned);
    match (
        text("key").filter(|key| named(key).is_some()),
        said.get("window").and_then(Value::as_u64),
    ) {
        (Some(key), Some(window)) => Ask::Type {
            id,
            key,
            detail: text("detail"),
            window,
        },
        _ => Ask::Unknown {
            id,
            ask: "type without an entry's key and a window".to_owned(),
        },
    }
}

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
    let asks_place = said.get("place").and_then(Value::as_str) == Some(secret_asks::row::PLACE);
    if asked == "search" && asks_place {
        // The asks are few and drawn whole, so the launcher matches them
        // itself, as it does the secret asks' own daemon's, rather than
        // being answered with entries.
        return Ask::Unknown {
            id,
            ask: "search of the asks".to_owned(),
        };
    }
    let kind = kind_of(&said, asks_place);
    let of = match kind.as_str() {
        "entries and asks" if matches!(asked, "list" | "watch") => Of::EntriesAndAsks,
        KIND => Of::Entries,
        ROW if matches!(asked, "list" | "watch") => Of::Rows,
        DRAWER if matches!(asked, "list" | "watch") => Of::Drawers(keys("drawer:passwords:")),
        _ if matches!(asked, "list" | "watch" | "search") => {
            return Ask::UnknownKind { id, kind };
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
        "reveal" | "copy" | "code" => entry_act(id, asked, text("key"), text("detail")),
        "type" => type_in(id, &said),
        "let-in" => Ask::LetIn {
            id,
            manager: text("manager").unwrap_or_default(),
        },
        "health" => Ask::Health { id },
        "audit" => Ask::Audit {
            id,
            asked: flag("asked"),
        },
        "release" => Ask::Release { id, said },
        "refuse" => Ask::Refuse { id, said },
        "revoke" => Ask::Revoke { id, said },
        other => Ask::Unknown {
            id,
            ask: other.to_owned(),
        },
    }
}

/// A reveal, copy or code, which names one entry by its key, or an
/// unknown ask when it names none.
fn entry_act(id: Value, asked: &str, key: Option<String>, detail: Option<String>) -> Ask {
    let Some(key) = key.filter(|key| named(key).is_some()) else {
        return Ask::Unknown {
            id,
            ask: format!("{asked} without an entry's key"),
        };
    };
    match asked {
        "reveal" => Ask::Reveal { id, key, detail },
        "copy" => Ask::Copy { id, key, detail },
        _ => Ask::Code { id, key },
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

/// Milliseconds since the epoch.
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
