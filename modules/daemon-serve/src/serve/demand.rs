//! What a daemon that reads on demand is handed, as the programs daemon
//! makes a preview only once it's got, what it reads as a get asks and
//! never holds, as the clipboard's history reads the text highlighted,
//! and what else it says on health: opt-in, so a daemon that hands every
//! record over as it changes is answered from what's held alone.

use std::collections::BTreeMap;

use serde_json::Value;

use super::Shared;
use crate::asks::Ask;

/// What a get, a list or a watch beginning names, for a daemon that
/// reads on demand to read before it's answered.
pub struct Wanted<'a> {
    /// The kinds named, or every kind but those named only where none
    /// were; none for a get.
    pub kinds: &'a [String],
    pub keys: Option<&'a [String]>,
    pub places: Option<&'a [String]>,
    /// Whether it's a watch beginning rather than a get or a list.
    pub watching: bool,
}

/// What reads what an ask wants before it's answered. It's handed the
/// ask outside the lock, on the asking connection's thread, so it may
/// call `change`; two connections asking at once may both read.
pub type Fresh = Box<dyn Fn(&Wanted<'_>) + Send + Sync>;

/// What else a daemon counts, as an object whose fields join health's
/// answer beside daemon-serve's own, which keep their values: the
/// daemons-agree check reads those alike for every daemon. It's asked
/// outside the lock.
pub type HealthToo = Box<dyn Fn() -> Value + Send + Sync>;

/// What reads a record a get names as it's asked, rather than holding it:
/// its key, and the record, or none when there's nothing to say of it.
pub type Read = Box<dyn Fn(&str) -> Option<Value> + Send + Sync>;

impl Shared {
    /// Hands `fresh` what each get, list and watch asks for before it's
    /// answered, from then on, a watch asked again under its id
    /// included; only the first a daemon gives is kept.
    pub fn fresh(&self, fresh: Fresh) {
        let _ = self.fresh.set(fresh);
    }

    /// Says the fields `too` gives on health too, from then on, where
    /// they don't name one of daemon-serve's own; only the first a
    /// daemon gives is kept.
    pub fn health_too(&self, too: HealthToo) {
        let _ = self.health_too.set(too);
    }

    /// Reads the records of these kinds with `read` as a get names their
    /// keys, from then on, rather than answering from what's held; only
    /// the first a daemon gives is kept. Each is read on its connection's
    /// thread, before the records held are locked, and never held.
    pub fn read_on_get(&self, kinds: &'static [&'static str], read: Read) {
        let _ = self.read.set((kinds, read));
    }

    /// The records of the kinds read on a get, by key, for these keys.
    pub(super) fn read_now(&self, keys: &[String]) -> BTreeMap<String, Value> {
        let Some((kinds, read)) = self.read.get() else {
            return BTreeMap::new();
        };
        keys.iter()
            .filter(|key| {
                self.domain
                    .kind_of(key)
                    .is_some_and(|kind| kinds.contains(&kind))
            })
            .filter_map(|key| read(key).map(|record| (key.clone(), record)))
            .collect()
    }

    /// Has the daemon read what `asked` wants, when it reads on demand.
    pub(super) fn read_for(&self, asked: &Ask) {
        let Some(fresh) = self.fresh.get() else {
            return;
        };
        let wanted = match asked {
            Ask::Get { keys, .. } => Wanted {
                kinds: &[],
                keys: Some(keys),
                places: None,
                watching: false,
            },
            Ask::List { kinds, places, .. } => Wanted {
                kinds,
                keys: None,
                places: places.as_deref(),
                watching: false,
            },
            Ask::Watch {
                kinds,
                keys,
                places,
                ..
            } => Wanted {
                kinds,
                keys: keys.as_deref(),
                places: places.as_deref(),
                watching: true,
            },
            _ => return,
        };
        fresh(&wanted);
    }

    /// What else health says for `asked`, when it's health.
    pub(super) fn said_too(&self, asked: &Ask) -> Option<Value> {
        let too = self.health_too.get()?;
        matches!(asked, Ask::Health { .. }).then(too)
    }
}

/// `too`'s fields added to `answer`, where it doesn't already say them.
pub(super) fn joined(answer: &mut Value, too: Value) {
    let (Some(answer), Value::Object(too)) = (answer.as_object_mut(), too) else {
        return;
    };
    for (field, said) in too {
        answer.entry(field).or_insert(said);
    }
}
