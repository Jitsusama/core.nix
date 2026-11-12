//! One watch a client holds: the kinds or keys it's told of as they
//! change, and, of the `row` kind, the one place whose rows it hears
//! (ready-rows.md), with what that place's rows are made of.

use std::collections::BTreeMap;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::sync::mpsc::SyncSender;

use programs::interface::kind_of;
use serde_json::Value;

pub struct Watch {
    pub connection: u64,
    pub id: Value,
    pub kinds: Vec<String>,
    pub keys: Option<Vec<String>>,
    pub told: SyncSender<String>,
    pub stream: Arc<UnixStream>,
    /// The hidden programs it lists all the same (`program_rows.rs`).
    pub shown: Vec<String>,
    /// The one place whose rows it hears, and the folders Capture's are
    /// taken from (`taken_rows.rs`).
    pub place: Option<String>,
    pub folders: Vec<String>,
    /// What each program's key binds do, by its id, which its search
    /// finds it by too (search.rs).
    pub also: BTreeMap<String, String>,
}

impl Watch {
    /// Whether it's told of this key: one it names, or of a kind it
    /// watches whole, a row only of its own place where it names one.
    pub fn wants(&self, key: &str) -> bool {
        let elsewhere = self.place.as_ref().is_some_and(|place| {
            key.starts_with("row:") && !key.starts_with(&format!("row:{place}:"))
        });
        !elsewhere && self.named(key)
    }

    fn named(&self, key: &str) -> bool {
        match &self.keys {
            Some(keys) => keys.iter().any(|k| k == key),
            None => kind_of(key).is_some_and(|kind| self.kinds.iter().any(|k| k == kind)),
        }
    }
}
