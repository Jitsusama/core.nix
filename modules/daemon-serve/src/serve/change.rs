//! What a daemon's loop hands to what's held, and what each watch is told
//! of it: every record whole (`change`), or only those that moved
//! (`mend`), both told through `tell`.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use serde_json::{Value, json};

use super::{Held, Peer, Shared, Unqueued, Watch, lock, moved, queue};
use crate::asks::{Domain, tells, trimmed};

/// The connections a change found stopped reading or gone, and why.
type Stuck = Vec<(Arc<Peer>, Unqueued)>;

impl Shared {
    /// Holds `records` as every record there is, and tells each watch
    /// those that changed and the keys that went, and, on a line of its
    /// own as the shell's Daemon.qml reads it, how the sources stand once
    /// that changes. A paced watch is told how the sources stand at once
    /// too, and holds back the keys that moved until its pace is up, until
    /// a record new to it comes, which is told at once with them, or until
    /// the sources stand anew, when they're told on the line before. A
    /// connection whose lines aren't being read is hung up on, and its
    /// watches dropped, rather than told less than it asked for.
    ///
    /// Each source is given by its word, or as `{"state", "reason"}`
    /// when it can say why it stands so, as one refused can: health says
    /// both, and every other line the word alone (`words`).
    pub fn change(&self, records: Vec<Value>, sources: Value) {
        let mut guard = lock(self);
        let held: &mut Held = &mut guard;
        let standing = held.stand(sources);
        let mut now = BTreeMap::new();
        let mut given = Vec::with_capacity(records.len());
        for record in records {
            if let Some(key) = record["key"].as_str() {
                given.push(key.to_owned());
                now.insert(key.to_owned(), record);
            }
        }
        let gone: Vec<String> = held
            .records
            .keys()
            .filter(|key| !now.contains_key(*key))
            .cloned()
            .collect();
        let changed: Vec<&Value> = now
            .iter()
            .filter(|(key, record)| held.records.get(*key) != Some(record))
            .map(|(_, record)| record)
            .collect();
        let told = Self::tell(
            &self.domain,
            &mut held.watches,
            &changed,
            &gone,
            &|key| held.records.get(key),
            &now,
            standing.as_ref(),
        );
        // Held whatever was told, so a list keeps the order given last
        // though no record changed and no watch heard of it.
        held.records = now;
        held.given = given;
        if let Some(told) = told {
            self.answered(held, &told);
        }
    }

    /// Tells each watch these records, which changed, and these keys,
    /// which went, as `change` does, holding every other record as it
    /// was: for a daemon that knows what moved and holds too many to
    /// give them all again, as the agents daemon's 6,000 were, 2.9 MB of
    /// them landings cloned and read against themselves at every move
    /// (perf, 2026-11-16). Its records keep their keys' order.
    pub fn mend(&self, changed: Vec<Value>, gone: &[String], sources: Value) {
        let mut guard = lock(self);
        let held: &mut Held = &mut guard;
        let standing = held.stand(sources);
        let mut before: BTreeMap<String, Option<Value>> = BTreeMap::new();
        for record in changed {
            if let Some(key) = record["key"].as_str().map(str::to_owned) {
                let was = held.records.insert(key.clone(), record);
                before.entry(key).or_insert(was);
            }
        }
        let gone: Vec<String> = gone
            .iter()
            .filter(|key| held.records.remove(*key).is_some())
            .cloned()
            .collect();
        if !gone.is_empty() || before.values().any(Option::is_none) {
            held.given = held.records.keys().cloned().collect();
        }
        let changed: Vec<&Value> = before
            .iter()
            .filter_map(|(key, was)| {
                held.records
                    .get(key)
                    .filter(|now| was.as_ref() != Some(*now))
            })
            .collect();
        let Some(told) = Self::tell(
            &self.domain,
            &mut held.watches,
            &changed,
            &gone,
            &|key| {
                before
                    .get(key)
                    .map_or_else(|| held.records.get(key), Option::as_ref)
            },
            &held.records,
            standing.as_ref(),
        ) else {
            return;
        };
        self.answered(held, &told);
    }

    /// Hangs up on the connections `told` found stopped reading, and wakes
    /// the pace where a paced watch has something due.
    fn answered(&self, held: &mut Held, (stuck, pacing): &(Stuck, bool)) {
        self.hang_up(held, stuck, "a watch was told what moved");
        if *pacing {
            self.paced.notify_all();
        }
    }

    /// Tells each watch what it wants of the records that changed and the
    /// keys that went, each record against how it read `before`, and how
    /// the sources stand where that changed, all of them held `now`.
    /// Answers the connections that stopped reading, and whether a paced
    /// watch has something due; with nothing changed, gone or newly
    /// standing, tells no watch and answers nothing.
    fn tell<'a>(
        domain: &Domain,
        watches: &mut [Watch],
        changed: &[&'a Value],
        gone: &[String],
        before: &dyn Fn(&str) -> Option<&'a Value>,
        now: &BTreeMap<String, Value>,
        standing: Option<&Value>,
    ) -> Option<(Stuck, bool)> {
        if changed.is_empty() && gone.is_empty() && standing.is_none() {
            return None;
        }
        let mut stuck = Vec::new();
        let mut pacing = false;
        for watch in watches {
            let fields = watch.fields.as_deref();
            let records: Vec<&Value> = changed
                .iter()
                .copied()
                .filter(|record| {
                    record["key"].as_str().is_some_and(|key| {
                        watch.wants(domain, key) && tells(before(key), record, fields)
                    })
                })
                .collect();
            let went: Vec<&str> = gone
                .iter()
                .map(String::as_str)
                .filter(|key| watch.wants(domain, key))
                .collect();
            // The records first, so a watch told how the sources stand
            // already holds what the same reading changed.
            let mut lines = Vec::new();
            if let Some(every) = watch.every {
                let keys = records.iter().filter_map(|record| record["key"].as_str());
                let arrived = keys.clone().any(|key| before(key).is_none());
                watch
                    .moved
                    .extend(keys.chain(went.iter().copied()).map(str::to_owned));
                // A record new to it, as a session started while the
                // launcher is shut, is told at once with all that was held
                // back, so an opening a moment later holds it in its place:
                // heard after the opening's first frame, the launcher keeps
                // what it shows still and puts it last (finds-sessions,
                // 2026-11-13). Only what moves waits for the pace. So is
                // all that was held back when the sources stand anew, as
                // the line saying so follows it: a watcher told niri
                // answers again takes what it holds as current, so a
                // record moved meanwhile would stay stale for a pace
                // (ISXEG8, 2026-11-18).
                if (arrived || standing.is_some())
                    && let Some(line) = watch.held_back(now)
                {
                    lines.push(line);
                } else if !watch.moved.is_empty() && watch.due.is_none() {
                    watch.due = Some(Instant::now() + every);
                    pacing = true;
                }
            } else if !records.is_empty() || !went.is_empty() {
                let told: Vec<Value> = records
                    .iter()
                    .map(|record| trimmed(record, fields))
                    .collect();
                lines.push(moved(&watch.id, &told, &went));
            }
            if let Some(sources) = standing {
                lines.push(json!({"id": watch.id, "sources": sources}));
            }
            if let Err(why) = lines.iter().try_for_each(|line| queue(&watch.told, line)) {
                stuck.push((Arc::clone(&watch.peer), why));
            }
        }
        Some((stuck, pacing))
    }
}
