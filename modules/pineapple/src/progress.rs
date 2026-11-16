//! How far a reader of a site speaking GitHub's API has read it, kept in
//! judged-<site>.json beside the Slack reader's: from when, and the keys
//! of what it judged, so a restart neither judges anything twice nor
//! misses what was said while it was down; and when it last read the
//! site whole, which `pineapple-github` says (src/bin/pineapple-github.rs).

use std::path::Path;

use serde_json::{Value, json};

/// How far it has read: from `since`, in seconds, the keys of what it
/// judged, newest last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Judged {
    /// From when the next round asks, in seconds since the epoch.
    pub since: u64,
    /// The keys of what it judged, newest last.
    pub keys: Vec<String>,
}

/// The name of the file keeping how far a site has been read.
#[must_use]
pub fn file(site: &str) -> String {
    format!("judged-{site}.json")
}

/// How far a site has been read, as kept in `state`; none when nothing is
/// kept or what is can't be read.
#[must_use]
pub fn read(state: &Path, site: &str) -> Option<Judged> {
    let text = std::fs::read_to_string(state.join(file(site))).ok()?;
    let kept: Value = serde_json::from_str(&text).ok()?;
    let since = kept.get("since").and_then(Value::as_u64)?;
    let keys = kept
        .get("keys")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    Some(Judged { since, keys })
}

/// Keeps how far a site has been read, and that it was read whole at
/// `read_at`, in milliseconds, through a file renamed into place, so a
/// reader stopped mid-write leaves the last whole one.
pub fn write(state: &Path, site: &str, judged: &Judged, read_at: u64) {
    let kept = json!({ "since": judged.since, "keys": judged.keys, "read_at": read_at });
    let path = state.join(file(site));
    let written = path.with_extension("new");
    let wrote = std::fs::create_dir_all(state)
        .and_then(|()| std::fs::write(&written, kept.to_string()))
        .and_then(|()| std::fs::rename(&written, &path));
    if let Err(error) = wrote {
        eprintln!("<3>couldn't keep how far it has read: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_kept_is_read_back_and_nothing_kept_is_none() {
        let state = std::env::temp_dir().join(format!("pineapple-progress-{}", std::process::id()));
        assert_eq!(read(&state, "code"), None, "nothing kept yet");
        let judged = Judged {
            since: 1_700_000_000,
            keys: vec!["code:example/shop#12:comment:1".to_owned()],
        };
        write(&state, "code", &judged, 1_700_000_000_000);
        assert_eq!(read(&state, "code"), Some(judged), "what was kept");
        assert_eq!(read(&state, "github"), None, "another site's is its own");
        std::fs::remove_dir_all(&state).expect("the case's state goes");
    }
}
