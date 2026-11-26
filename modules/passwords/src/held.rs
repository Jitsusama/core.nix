//! Every manager's entries as they're held: how each manager stands,
//! what a list or an entry it gives changed, each entry's health as
//! the last audit found it, what the checks found over time and what's
//! kept of it, and the search over titles and words.

use std::collections::{BTreeMap, HashMap};

use serde_json::{Map, Value, json};

use crate::audit::Health;
use crate::{Entry, Standing, key, named};

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
