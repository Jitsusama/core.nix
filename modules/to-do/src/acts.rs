//! Joel's word, the two asks the to-do daemon answers itself: on a to-do,
//! kept in settled.json (settle.rs), and on a look, kept in its own file
//! (looked.rs). The daemon's watches on them then tell every watch as they
//! tell any write, after the act is answered done. Everything else asked
//! is answered by daemon-serve from the records the daemon's loop hands it.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use daemon_serve::asks::error;
use daemon_serve::serve::Own;
use serde_json::{Value, json};

use crate::interface::{DOMAIN, LookSettled, act_of, look_settled_of};
use crate::looked::settle_look;
use crate::settle::settle;
use crate::stream::said;

/// The to-dos as last told, by key, so an act finds what its to-do says;
/// daemon-serve holds every record too, but answers a daemon's own ask
/// while it holds them, so they're kept again here.
pub struct Acts {
    settled: Option<PathBuf>,
    looks: Option<PathBuf>,
    to_dos: Mutex<BTreeMap<String, Value>>,
}

impl Acts {
    /// What answers acts, keeping Joel's word on a to-do in `settled` and
    /// on a look in its file in `looks`, or refusing each where there's
    /// nowhere to keep it.
    pub fn new(settled: Option<PathBuf>, looks: Option<PathBuf>) -> Arc<Self> {
        Arc::new(Acts {
            settled,
            looks,
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

    /// What answers the daemon's own asks for daemon-serve: an act on a
    /// to-do or a look settled, and no other.
    pub fn own(self: &Arc<Self>) -> Own {
        let acts = Arc::clone(self);
        Box::new(move |said| {
            act_of(said)
                .map(|act| acts.acted(&act.act, &act.item))
                .or_else(|| look_settled_of(said).map(|settled| acts.looked(&settled)))
        })
    }

    /// What a look settled is answered with, once its file is renamed into
    /// place, so no watch hears the look as it was after Joel is told done.
    fn looked(&self, settled: &LookSettled) -> Value {
        let Some(folder) = &self.looks else {
            return error(&Value::Null, "refused", "no looks folder here");
        };
        let at = i64::try_from(now_ms()).unwrap_or(i64::MAX);
        match settle_look(folder, &settled.look, &settled.state, &settled.how, at) {
            Ok(()) => json!({"done": true}),
            Err(refused) => error(&Value::Null, refused.code, &refused.message),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn a_look_settled_is_kept_in_its_file_and_answered_done() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("a1.json");
        fs::write(&path, r#"{"v":1,"id":"a1","state":"waiting"}"#).unwrap();
        let acts = Acts::new(None, Some(folder.path().to_owned()));
        let own = acts.own();
        let asked = |look: &str| {
            own(
                &json!({"id": 3, "ask": "settle-look", "look": look, "state": "cleared",
                "how": "cleared from For You"}),
            )
        };
        assert_eq!(asked("a1"), Some(json!({"done": true})), "a look there");
        let kept: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            (&kept["state"], &kept["how"], &kept["id"]),
            (
                &json!("cleared"),
                &json!("cleared from For You"),
                &json!("a1")
            ),
            "already in its file when answered: {kept}"
        );
        assert_eq!(
            asked("gone").map(|answer| answer["error"]["code"].clone()),
            Some(json!("unknown-item")),
            "a look not there is refused"
        );
        let nowhere = Acts::new(None, None).own()(&json!({"id": 4, "ask": "settle-look",
            "look": "a1", "state": "opened", "how": ""}));
        assert_eq!(
            nowhere.map(|answer| answer["error"]["code"].clone()),
            Some(json!("refused")),
            "with no folder of looks there's nowhere to keep it"
        );
    }
}
