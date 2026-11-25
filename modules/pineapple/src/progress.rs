//! How far a reader of a site speaking GitHub's API has read it, kept in
//! judged-<site>.json beside the Slack reader's: from when, and the keys
//! of what it judged, so a restart neither judges anything twice nor
//! misses what was said while it was down; how far back Joel's own words
//! were searched, so a restart neither judges past where a first start
//! chose not to nor forgets a search held back; what it owes, the pull
//! requests and issues a round couldn't read, so a restart reads them too;
//! and when it last read the site whole, owing nothing, which
//! `pineapple-github` says (src/bin/pineapple-github/).
//! A search held back by days of failing is let go by taking `own` out of
//! the file with its reader stopped, as a running one writes it back each
//! round; that lets go of a first start's floor too. A reader from before
//! owing writes the file back without what's owed, so going back to one
//! lets that go unnamed, its `since` already past it.

use std::path::Path;

use serde_json::{Value, json};

use crate::github::Thread;

/// How far it has read: from `since`, in seconds, the keys of what it
/// judged, newest last, and how far Joel's own words were searched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Judged {
    /// From when the next round asks, in seconds since the epoch.
    pub since: u64,
    /// The keys of what it judged, newest last.
    pub keys: Vec<String>,
    /// How far Joel's own words were searched; none where nothing kept
    /// says, as before a reader searched for them.
    pub own: Option<Own>,
    /// What a round couldn't read, so the next reads it again.
    pub owed: Vec<Owed>,
}

/// A pull request or issue a round couldn't read, read again from where it
/// was to be until it is: what's judged on it from `from`, in seconds
/// since the epoch, read from `read_from`, owed since `at`, asked and
/// failed `tries` times, and asked again after `skip` more rounds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Owed {
    /// What's owed.
    pub thread: Thread,
    /// From when what's said there is judged.
    pub from: u64,
    /// From when it's read, before `from` where a late stop is told of.
    pub read_from: u64,
    /// When a round first couldn't read it.
    pub at: u64,
    /// How many asks its wait chose failed: not asks a spent limit kept
    /// back, nor those made early as it changed.
    pub tries: u32,
    /// How many rounds it waits before it's asked again.
    pub skip: u32,
}

/// How far a search for Joel's own words reached, in seconds since the
/// epoch: where it was last whole from, and the farthest back it may
/// judge, which a first start sets and zero leaves open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Own {
    /// Where the search was last whole from.
    pub from: u64,
    /// The farthest back it judges, whatever `from` says; what it finds
    /// is read an hour further back, for the journal to name.
    pub floor: u64,
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
    let own = kept.get("own").and_then(|own| {
        Some(Own {
            from: own.get("from")?.as_u64()?,
            floor: own.get("floor")?.as_u64()?,
        })
    });
    let owed = kept
        .get("owed")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|owed| {
            Some(Owed {
                thread: Thread {
                    site: site.to_owned(),
                    repo: owed.get("repo")?.as_str()?.to_owned(),
                    number: owed.get("number")?.as_u64()?,
                    pull: owed.get("pull")?.as_bool()?,
                },
                from: owed.get("from")?.as_u64()?,
                read_from: owed.get("read_from")?.as_u64()?,
                at: owed.get("at")?.as_u64()?,
                tries: count(owed, "tries"),
                skip: count(owed, "skip"),
            })
        })
        .collect();
    Some(Judged {
        since,
        keys,
        own,
        owed,
    })
}

/// The count `owed` keeps under `name`, none where an older file kept none.
fn count(owed: &Value, name: &str) -> u32 {
    owed.get(name)
        .and_then(Value::as_u64)
        .and_then(|count| u32::try_from(count).ok())
        .unwrap_or(0)
}

/// Each site kept in `state`, by name, and when it was last read whole, in
/// milliseconds, sorted by name; a file that can't be read is left out.
#[must_use]
pub fn sites(state: &Path) -> Vec<(String, u64)> {
    let mut sites: Vec<(String, u64)> = std::fs::read_dir(state)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let site = name.strip_prefix("judged-")?.strip_suffix(".json")?;
            let text = std::fs::read_to_string(entry.path()).ok()?;
            let kept: Value = serde_json::from_str(&text).ok()?;
            Some((site.to_owned(), kept.get("read_at")?.as_u64()?))
        })
        .collect();
    sites.sort();
    sites
}

/// Keeps how far a site has been read, and that it was read whole at
/// `read_at`, in milliseconds, through a file renamed into place, so a
/// reader stopped mid-write leaves the last whole one. While it owes
/// anything, when it was last read whole stays as it was, so a pull
/// request it can't read shows as the site not read whole since.
pub fn write(state: &Path, site: &str, judged: &Judged, read_at: u64) {
    let mut kept = json!({ "since": judged.since, "keys": judged.keys });
    let whole = if judged.owed.is_empty() {
        Some(read_at)
    } else {
        let text = std::fs::read_to_string(state.join(file(site))).ok();
        let before = text.and_then(|text| serde_json::from_str::<Value>(&text).ok());
        before.and_then(|before| before.get("read_at")?.as_u64())
    };
    if let Some(whole) = whole {
        kept["read_at"] = json!(whole);
    }
    if let Some(Own { from, floor }) = judged.own {
        kept["own"] = json!({ "from": from, "floor": floor });
    }
    if !judged.owed.is_empty() {
        let owed: Vec<Value> = judged
            .owed
            .iter()
            .map(|owed| {
                json!({ "repo": owed.thread.repo, "number": owed.thread.number,
                    "pull": owed.thread.pull, "from": owed.from,
                    "read_from": owed.read_from, "at": owed.at,
                    "tries": owed.tries, "skip": owed.skip })
            })
            .collect();
        kept["owed"] = Value::Array(owed);
    }
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

    /// A state folder of the case's own, named `what`, made by no one yet.
    fn fresh(what: &str) -> std::path::PathBuf {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the clock is past 1970")
            .as_nanos();
        let id = std::process::id();
        std::env::temp_dir().join(format!("pineapple-progress-{what}-{id}-{now}"))
    }

    #[test]
    fn each_site_kept_says_when_it_was_read_and_nothing_else_counts() {
        let state = fresh("sites");
        assert_eq!(sites(&state), Vec::new(), "nothing kept yet");
        let judged = Judged {
            since: 1_700_000_000,
            keys: Vec::new(),
            own: None,
            owed: Vec::new(),
        };
        write(&state, "github", &judged, 2_000);
        write(&state, "code", &judged, 1_000);
        write(&state, "delta", &judged, 3_000);
        std::fs::write(state.join(file("broken")), "not json").expect("a broken file");
        std::fs::write(state.join(file("unread")), r#"{"since": 1, "keys": []}"#)
            .expect("a file never read whole");
        std::fs::write(state.join("decisions.jsonl"), "{}").expect("another file");
        assert_eq!(
            sites(&state),
            vec![
                ("code".to_owned(), 1_000),
                ("delta".to_owned(), 3_000),
                ("github".to_owned(), 2_000)
            ],
            "each site by name, sorted, with when it was read; one never read whole, \
             a broken one and the rest left out"
        );
        std::fs::remove_dir_all(&state).expect("the case's state goes");
    }

    #[test]
    fn what_is_kept_is_read_back_and_nothing_kept_is_none() {
        let state = fresh("kept");
        assert_eq!(read(&state, "code"), None, "nothing kept yet");
        let judged = Judged {
            since: 1_700_000_000,
            keys: vec!["code:example/shop#12:comment:1".to_owned()],
            own: None,
            owed: Vec::new(),
        };
        write(&state, "code", &judged, 1_700_000_000_000);
        assert_eq!(read(&state, "code"), Some(judged), "what was kept");
        assert_eq!(read(&state, "github"), None, "another site's is its own");
        let searched = Judged {
            since: 1_700_000_000,
            keys: Vec::new(),
            own: Some(Own {
                from: 1_699_999_000,
                floor: 1_699_998_000,
            }),
            owed: Vec::new(),
        };
        write(&state, "github", &searched, 1_700_000_000_000);
        assert_eq!(
            read(&state, "github"),
            Some(searched),
            "how far Joel's own words were searched is kept with it"
        );
        let owing = Judged {
            since: 1_700_000_100,
            keys: Vec::new(),
            own: None,
            owed: vec![Owed {
                thread: Thread {
                    site: "github".to_owned(),
                    repo: "example/shop".to_owned(),
                    number: 14,
                    pull: true,
                },
                from: 1_699_999_940,
                read_from: 1_699_996_340,
                at: 1_700_000_000,
                tries: 2,
                skip: 1,
            }],
        };
        write(&state, "github", &owing, 1_700_000_100_000);
        assert_eq!(
            read(&state, "github"),
            Some(owing),
            "what it owes is kept with it"
        );
        assert_eq!(
            sites(&state),
            vec![
                ("code".to_owned(), 1_700_000_000_000),
                ("github".to_owned(), 1_700_000_000_000)
            ],
            "while it owes anything, it was last read whole when it last owed nothing"
        );
        std::fs::remove_dir_all(&state).expect("the case's state goes");
    }

    #[test]
    fn what_an_older_file_owes_has_failed_no_tries_and_waits_no_rounds() {
        let state = fresh("older");
        std::fs::create_dir_all(&state).expect("the case's state");
        let owed = json!({"repo": "example/shop", "number": 14, "pull": true,
            "from": 1_699_999_940, "read_from": 1_699_996_340, "at": 1_700_000_000});
        let kept = json!({"since": 1_700_000_100, "keys": [], "owed": [owed]});
        std::fs::write(state.join(file("github")), kept.to_string()).expect("the file");

        let owed = read(&state, "github").map(|judged| judged.owed);
        let counts = owed.as_deref().and_then(|owed| {
            let first = owed.first()?;
            Some((owed.len(), first.tries, first.skip))
        });
        assert_eq!(counts, Some((1, 0, 0)), "it's asked next round, as one new");
        std::fs::remove_dir_all(&state).expect("the case's state goes");
    }
}
