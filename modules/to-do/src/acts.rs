//! Joel's word on a to-do, the one ask the to-do daemon answers itself:
//! kept in settled.json (settle.rs), which the daemon's watch on it then
//! tells every watch as it tells any write. Everything else asked is
//! answered by daemon-serve from the records the daemon's loop hands it.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use daemon_serve::asks::error;
use daemon_serve::serve::Own;
use serde_json::{Value, json};

use crate::interface::{DOMAIN, act_of};
use crate::settle::settle;
use crate::stream::said;

/// The to-dos as last told, by key, so an act finds what its to-do says;
/// daemon-serve holds every record too, but answers a daemon's own ask
/// while it holds them, so they're kept again here.
pub struct Acts {
    settled: Option<PathBuf>,
    to_dos: Mutex<BTreeMap<String, Value>>,
}

impl Acts {
    /// What answers acts, keeping Joel's word in `settled`, or refusing
    /// each where there's nowhere to keep it.
    pub fn new(settled: Option<PathBuf>) -> Arc<Self> {
        Arc::new(Acts {
            settled,
            to_dos: Mutex::new(BTreeMap::new()),
        })
    }

    /// Takes the records told, keeping the to-dos of them.
    pub fn told(&self, records: &[Value]) {
        let to_dos = records
            .iter()
            .filter_map(|record| {
                let key = record["key"].as_str()?;
                (DOMAIN.kind_of(key) == Some("to-do")).then(|| (key.to_owned(), record.clone()))
            })
            .collect();
        *self.to_dos.lock().unwrap_or_else(PoisonError::into_inner) = to_dos;
    }

    /// What answers the daemon's own asks for daemon-serve: an act, and
    /// no other.
    pub fn own(self: &Arc<Self>) -> Own {
        let acts = Arc::clone(self);
        Box::new(move |said| act_of(said).map(|act| acts.acted(&act.act, &act.item)))
    }

    /// What an act on a to-do is answered with, once Joel's word is kept:
    /// done, or why not. Its id is daemon-serve's to give.
    fn acted(&self, act: &str, item: &str) -> Value {
        let to_dos = self.to_dos.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(record) = to_dos.get(&format!("to-do:{item}")) else {
            return error(
                &Value::Null,
                "unknown-item",
                &format!("no to-do {item} here"),
            );
        };
        let Some(path) = &self.settled else {
            return error(&Value::Null, "refused", "nowhere to keep Joel's word");
        };
        match settle(path, act, item, &said(&record["item"]), now_ms()) {
            Ok(()) => json!({"done": true}),
            Err(refused) => error(&Value::Null, refused.code, &refused.message),
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}
