//! What programs tell of their own lists, as kitty's tells.py does: each
//! `changed` ask counted, and from how many programs, which health says
//! so how often they tell can be read; its sender heard as one that tells
//! from now on; and each told list it names read again.

use std::sync::Arc;

use serde_json::{Value, json};

use super::{Run, Shared, lock, sources_of, start, tabs};

/// A `changed` ask heard from `from`, naming these keys' lists, and its
/// answer.
pub fn heard(shared: &Arc<Shared>, id: &Value, keys: &[String], from: Option<u64>) -> Value {
    {
        let mut daemon = lock(&shared.daemon);
        daemon.told += 1;
        if let Some(from) = from {
            daemon.tellers.insert(from);
        }
    }
    changed(shared, keys);
    json!({"id": id, "heard": keys})
}

/// Each told list these keys name is read again: at once, on a thread of
/// its own, where a watch names its key, as a page open on it does, or
/// it's kitty's while the tabs are watched, and otherwise as it's next
/// asked. A list that isn't told reads as it always does, so a program
/// saying so changes nothing of it.
fn changed(shared: &Arc<Shared>, keys: &[String]) {
    for at in sources_of(shared, Run::List, &[], Some(keys)) {
        let source = &shared.sources[at];
        if !source.told {
            continue;
        }
        let key = source.list_key();
        let named = {
            let mut daemon = lock(&shared.daemon);
            daemon.stale[at] = true;
            daemon
                .watches
                .iter()
                .any(|watch| watch.keys.as_ref().is_some_and(|keys| keys.contains(&key)))
                || (tabs::kitty(shared) == Some(at) && tabs::watched(&daemon))
        };
        if named {
            start(shared, at, Run::List);
        }
    }
}
