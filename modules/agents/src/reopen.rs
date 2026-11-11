//! The sessions there are to reopen, worked out here rather than in the
//! launcher, which only shows what it's given (Joel, 2026-11-10: "the
//! daemons need to be the data layer"): the last boot's, as the harness's
//! restore.json keeps them, and those offered apart from any boot, each
//! until it runs here, but those Joel forgot. What Sessions.qml and
//! Places.js worked out of the file is worked out once here, as the file
//! or the sessions change.
//!
//! - The last boot's are offered only after another boot, and only until
//!   Joel puts that boot off. One whose quest has a session here that
//!   restore.json doesn't list, begun since, is left out, since that one
//!   took its place (OSPHT4, 2026-11-11: a quest ran twice after a reboot).
//! - Those offered apart came another way, whatever the boot: carried here
//!   by another machine as it went away, or given back by pi's host. They
//!   are never put off with a boot, and come the newest first.
//! - An entry the file doesn't name whole is left out: an id safe for pi's
//!   command line, a folder to open in, or for one held on another
//!   machine, the words that join it again. One that ran hosted is
//!   offered for its viewer alone, since its host brings the conversation
//!   back, and counts as back once its viewer is open where the boot
//!   being restored kept a place of its window, as the shell says it
//!   (inputs.rs).
//!
//! What Joel put off and forgot is kept by the daemon, in a file of its
//! own (`Kept`), since restore.json is the harness's to write.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;

use serde_json::{Value, json};

use crate::served::{PLACE, SPACING};

/// The ways a session comes to be offered apart from a boot.
const APART: [&str; 2] = ["carried", "released"];

/// What Joel said of the sessions to reopen: the boot he put off, and
/// the sessions he forgot, by id.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Kept {
    pub put_off: String,
    pub forgotten: Vec<String>,
}

impl Kept {
    /// What `path` keeps, or nothing kept when it isn't there or can't be
    /// read whole.
    pub fn read(path: &Path) -> Self {
        let Some(said) = fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        else {
            return Self::default();
        };
        Self {
            put_off: said["putOff"].as_str().unwrap_or("").to_owned(),
            forgotten: said["forgotten"]
                .as_array()
                .map(|ids| {
                    ids.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    /// Writes what's kept to `path` whole, renamed into place, so a
    /// daemon started again reads it as it was.
    pub fn write(&self, path: &Path) -> io::Result<()> {
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)?;
        }
        let writing = path.with_extension("json.new");
        let said = json!({"putOff": self.put_off, "forgotten": self.forgotten});
        fs::write(&writing, said.to_string())?;
        fs::rename(&writing, path)
    }

    /// Joel forgets `id`, keeping only those `restore` still names, so
    /// what's kept never grows past the file.
    pub fn forget(&mut self, id: &str, restore: Option<&Value>) {
        let named: HashSet<&str> = entries(restore)
            .filter_map(|one| one["id"].as_str())
            .collect();
        self.forgotten.retain(|one| named.contains(one.as_str()));
        if !self.forgotten.iter().any(|one| one == id) {
            self.forgotten.push(id.to_owned());
        }
    }
}

/// A session here, running or parked, by pi's id for it and its quest.
pub struct Here<'a> {
    pub id: &'a str,
    pub quest: &'a str,
}

/// How one came to be offered: from the last boot, or apart from it, the
/// way it came, from where and when.
#[derive(Debug, PartialEq, Eq)]
enum Way<'a> {
    Boot,
    Apart {
        kind: &'a str,
        from: &'a str,
        at: &'a str,
    },
}

fn entries(restore: Option<&Value>) -> impl Iterator<Item = &Value> {
    restore
        .and_then(|said| said["sessions"].as_array())
        .into_iter()
        .flatten()
}

/// The way `one` came, or none where it says so badly, or two ways.
fn way_of(one: &Value) -> Option<Way<'_>> {
    let ways: Vec<&str> = APART
        .into_iter()
        .filter(|way| one.get(*way).is_some())
        .collect();
    match ways[..] {
        [] => Some(Way::Boot),
        [kind] => {
            let said = &one[kind];
            let from = said["from"].as_str().filter(|from| !from.is_empty())?;
            let at = said["at"].as_str()?;
            Some(Way::Apart { kind, from, at })
        }
        _ => None,
    }
}

/// Whether `one` is named whole: an id pi's command line takes, and a
/// folder to open in here, or for one held on another machine, the words
/// that join it again.
fn whole(one: &Value) -> bool {
    let id = one["id"].as_str().unwrap_or("");
    let safe = !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte));
    let cwd = one["cwd"].as_str();
    let held_here = || cwd.is_some_and(|cwd| cwd.starts_with('/'));
    let held_elsewhere = || {
        let words = one["attach"].as_array();
        one["address"].is_string()
            && words.is_some_and(|words| !words.is_empty() && words.iter().all(Value::is_string))
            && cwd.is_some_and(|cwd| cwd.is_empty() || cwd.starts_with('/'))
    };
    let placed = match one.get("machine") {
        None => held_here(),
        Some(machine) => machine.as_str().is_some_and(|name| !name.is_empty()) && held_elsewhere(),
    };
    safe && placed
}

/// The boot restore.json names, served as `restoring:agents`, so the shell
/// keeps each place by the boot its sessions ran in without reading the
/// harness's file itself; empty when the file names none.
pub fn restoring(restore: Option<&Value>) -> Value {
    let boot = restore.and_then(|said| said["boot"].as_str()).unwrap_or("");
    json!({"key": format!("restoring:{PLACE}"), "kind": "restoring",
           "restoring": {"boot": boot}})
}

/// Every session there is to reopen, each a record keyed
/// `reopen:agents:<id>`, in Agents' place, so a watch of its place hears it,
/// in the order offered, the last boot's first: `restore` as the harness
/// keeps it, `boot` this one, `kept` what Joel said, and `here` the
/// sessions running or parked here now.
pub fn offered(restore: Option<&Value>, boot: &str, kept: &Kept, here: &[Here]) -> Vec<Value> {
    let from = restore.and_then(|said| said["boot"].as_str()).unwrap_or("");
    let last_boot = !boot.is_empty() && !from.is_empty() && from != boot && from != kept.put_off;
    let named: Vec<&Value> = entries(restore).filter(|one| whole(one)).collect();
    let listed: HashSet<&str> = named.iter().filter_map(|one| one["id"].as_str()).collect();
    let back: HashSet<&str> = here.iter().map(|one| one.id).collect();
    let replaced: HashSet<&str> = here
        .iter()
        .filter(|one| !listed.contains(one.id) && !one.quest.is_empty())
        .map(|one| one.quest)
        .collect();
    let mut of_the_boot = Vec::new();
    let mut apart = Vec::new();
    for one in named {
        let id = one["id"].as_str().unwrap_or("");
        if kept.forgotten.iter().any(|forgot| forgot == id) {
            continue;
        }
        match way_of(one) {
            Some(Way::Boot) => {
                let quest = one["quest"].as_str().unwrap_or("");
                // No empty quest is among those replaced.
                if last_boot && !back.contains(id) && !replaced.contains(quest) {
                    of_the_boot.push((one, json!({"way": "boot", "boot": from})));
                }
            }
            Some(Way::Apart { kind, from, at }) => {
                apart.push((at, id, one, json!({"way": kind, "from": from, "at": at})));
            }
            None => {}
        }
    }
    apart.sort_by(|one, other| other.0.cmp(one.0).then(one.1.cmp(other.1)));
    of_the_boot
        .into_iter()
        .chain(apart.into_iter().map(|(_, _, one, way)| (one, way)))
        .zip(1..)
        .map(|((one, mut record), at)| {
            let id = one["id"].as_str().unwrap_or("");
            record["key"] = json!(format!("reopen:{PLACE}:{id}"));
            record["kind"] = json!("reopen");
            record["order"] = json!(at * SPACING);
            record["session"] = one.clone();
            record
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn restore() -> Value {
        json!({"v": 1, "boot": "the-last", "sessions": [
            {"id": "01-kept", "cwd": "/tmp", "quest": "QEST-KEEL"},
            {"id": "01-asleep", "cwd": "/tmp", "parked": true, "quest": "QEST-REEF"},
            {"id": "01-ferried", "cwd": "/tmp", "carried": {"from": "far", "at": "1"}},
            {"id": "01-freed", "cwd": "/tmp", "released": {"from": "attended", "at": "2"}}
        ]})
    }

    fn ids(records: &[Value]) -> Vec<&str> {
        records
            .iter()
            .map(|one| one["session"]["id"].as_str().unwrap_or(""))
            .collect()
    }

    fn offered_now(kept: &Kept, here: &[Here]) -> Vec<Value> {
        offered(Some(&restore()), "this-one", kept, here)
    }

    #[test]
    fn the_last_boots_come_first_then_those_apart_the_newest_first() {
        let all = offered_now(&Kept::default(), &[]);
        assert_eq!(
            ids(&all),
            ["01-kept", "01-asleep", "01-freed", "01-ferried"]
        );
        assert_eq!(
            all[0],
            json!({"key": "reopen:agents:01-kept", "kind": "reopen", "order": 1024, "way": "boot",
                "boot": "the-last", "session": restore()["sessions"][0]}),
            "each says how it came, keyed by its id and placed by a spaced order"
        );
        assert_eq!(
            (&all[2]["way"], &all[2]["from"], &all[2]["at"]),
            (&json!("released"), &json!("attended"), &json!("2")),
            "one apart says the way it came, from where and when"
        );
    }

    #[test]
    fn each_is_offered_until_its_back_here_or_forgotten() {
        let here = [Here {
            id: "01-kept",
            quest: "QEST-KEEL",
        }];
        assert_eq!(
            ids(&offered_now(&Kept::default(), &here)),
            ["01-asleep", "01-freed", "01-ferried"]
        );
        let kept = Kept {
            put_off: String::new(),
            forgotten: vec!["01-freed".into()],
        };
        assert_eq!(
            ids(&offered_now(&kept, &[])),
            ["01-kept", "01-asleep", "01-ferried"]
        );
    }

    #[test]
    fn a_boot_put_off_or_this_boots_own_offers_only_those_apart() {
        let kept = Kept {
            put_off: "the-last".into(),
            forgotten: vec![],
        };
        assert_eq!(ids(&offered_now(&kept, &[])), ["01-freed", "01-ferried"]);
        assert_eq!(
            ids(&offered(
                Some(&restore()),
                "the-last",
                &Kept::default(),
                &[]
            )),
            ["01-freed", "01-ferried"],
            "nothing of this boot's own"
        );
        assert!(
            offered(None, "this-one", &Kept::default(), &[]).is_empty(),
            "no file, nothing to reopen"
        );
    }

    #[test]
    fn none_whose_quest_a_session_begun_since_took_over() {
        let fresh = [Here {
            id: "02-fresh",
            quest: "QEST-REEF",
        }];
        assert_eq!(
            ids(&offered_now(&Kept::default(), &fresh)),
            ["01-kept", "01-freed", "01-ferried"]
        );
        let loose = [Here {
            id: "02-loose",
            quest: "",
        }];
        assert_eq!(
            ids(&offered_now(&Kept::default(), &loose)).len(),
            4,
            "one begun since with no quest takes none's place"
        );
    }

    #[test]
    fn one_named_badly_or_saying_two_ways_is_left_out() {
        let said = json!({"boot": "the-last", "sessions": [
            {"id": "has space", "cwd": "/tmp"},
            {"id": "01-nowhere", "cwd": "tmp"},
            {"id": "01-both", "cwd": "/tmp", "carried": {"from": "far", "at": "1"},
                "released": {"from": "attended", "at": "2"}},
            {"id": "01-unsaid", "cwd": "/tmp", "carried": {"from": "", "at": "1"}},
            {"id": "01-far", "cwd": "", "machine": "lighthouse", "address": "FARAWY",
                "attach": ["mycelium", "attach", "FARAWY"]},
            {"id": "01-wordless", "cwd": "", "machine": "lighthouse", "address": "OUTRCH",
                "attach": []}
        ]});
        assert_eq!(
            ids(&offered(Some(&said), "this-one", &Kept::default(), &[])),
            ["01-far"],
            "only one held elsewhere with the words to join it again is named whole"
        );
    }

    #[test]
    fn forgetting_keeps_only_what_the_file_still_names() {
        let mut kept = Kept {
            put_off: String::new(),
            forgotten: vec!["01-long-gone".into()],
        };
        kept.forget("01-kept", Some(&restore()));
        kept.forget("01-kept", Some(&restore()));
        assert_eq!(kept.forgotten, ["01-kept"]);
    }

    #[test]
    fn what_joel_said_is_kept_across_a_start() {
        let folder = std::env::temp_dir().join(format!("agents-reopen-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        let path = folder.join("agents").join("reopen.json");
        assert_eq!(Kept::read(&path), Kept::default(), "nothing kept before");
        let kept = Kept {
            put_off: "the-last".into(),
            forgotten: vec!["01-kept".into()],
        };
        kept.write(&path).expect("it's written");
        assert_eq!(Kept::read(&path), kept);
    }
}
