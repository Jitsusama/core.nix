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

use crate::row::Words;
use crate::said::Said;
use crate::served::{PLACE, SPACING};

/// The key of Sessions to Reopen's own record, saying how many there are,
/// where they came from and how many folders are gone.
pub const TO_REOPEN: &str = "to-reopen:agents";

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

/// What a row to reopen is said against beyond its record.
pub struct Wording<'a> {
    /// The launcher's words.
    pub words: &'a Words,
    /// The account's home folder, said as `~`.
    pub home: &'a str,
    /// Whether a folder is there.
    pub there: &'a dyn Fn(&str) -> bool,
}

/// Gives each record `offered` made its row as the launcher shows it,
/// under `row`, and answers Sessions to Reopen's own record, or none
/// while none is offered. A row is named by its title, address or id;
/// its side says the folder it reopens in from home, or that it's gone,
/// looked for as it's made, as a closed orchestrator's is, rather than
/// by the shell on every opening; one held elsewhere says its machine;
/// its note says the way it came. One pi's host gave back names that
/// host apart, since the host's name is no place Joel chose (VKCF3Z,
/// [92a1d30c]).
pub fn worded(offered: &mut [Value], wording: &Wording) -> Option<Value> {
    if offered.is_empty() {
        return None;
    }
    let place = &wording.words.to_reopen;
    let mut ways = Ways::default();
    let mut gone = 0;
    for record in offered.iter_mut() {
        let row = row_of(record, wording, &mut ways);
        gone += usize::from(row["gone"] == true);
        record["row"] = row;
    }
    let count = offered.len();
    let side = if count == 1 {
        place.word("one").to_owned()
    } else {
        place.fill("many", &[("count", &count.to_string())])
    };
    let there = place.fill(
        "there",
        &[
            ("there", &(count - gone).to_string()),
            ("count", &count.to_string()),
        ],
    );
    let gone_said = if gone == 1 {
        place.word("goneOne").to_owned()
    } else {
        place.fill("goneMany", &[("count", &gone.to_string())])
    };
    // Reopen Them All counts those it can reopen only when any can't, the
    // count that failed in the alert's colour (VKCF3Z, 2026-11-07).
    let (all_side, all_items) = if gone == 0 {
        (String::new(), json!([]))
    } else {
        (
            format!("{there} \u{b7} {gone_said}"),
            json!([{"said": there, "keep": 2}, {"said": gone_said, "keep": 3, "hue": "alert"}]),
        )
    };
    Some(json!({"key": TO_REOPEN, "kind": "to-reopen", "toReopen": {
        "side": side,
        "note": ways.note(place),
        "there": count - gone,
        "gone": gone,
        "lastBoot": ways.booted,
        "allSide": all_side,
        "allItems": all_items,
    }}))
}

/// How many came each way, and the machines those carried came from, in
/// the order first met.
#[derive(Default)]
struct Ways {
    booted: usize,
    carried: usize,
    machines: Vec<String>,
    released: usize,
}

impl Ways {
    /// Where they came from, as Sessions to Reopen's note says it: one
    /// way alone by itself, but those given back counted, since that way
    /// names no place; several each counted, and several machines counted
    /// rather than listed.
    fn note(&self, place: &Said) -> String {
        let carried = if self.machines.len() == 1 {
            place.fill("carried", &[("machine", &self.machines[0])])
        } else {
            let machines = place.fill("machines", &[("count", &self.machines.len().to_string())]);
            place.fill("carried", &[("machine", &machines)])
        };
        let ways: Vec<(usize, String, bool)> = [
            (self.booted, place.word("lastBoot").to_owned(), false),
            (self.carried, carried, false),
            (self.released, place.word("givenBack").to_owned(), true),
        ]
        .into_iter()
        .filter(|(count, _, _)| *count > 0)
        .collect();
        let counted = |(count, from, _): &(usize, String, bool)| {
            place.fill("counted", &[("count", &count.to_string()), ("from", from)])
        };
        match &ways[..] {
            [] => place.word("lastBoot").to_owned(),
            [one] if one.2 => counted(one),
            [one] => one.1.clone(),
            several => several.iter().map(counted).collect::<Vec<_>>().join(", "),
        }
    }
}

/// One record's row, counting the way it came into `ways`.
fn row_of(record: &Value, wording: &Wording, ways: &mut Ways) -> Value {
    let words = wording.words;
    let session = &record["session"];
    let text = |field: &str| session[field].as_str().unwrap_or("");
    let name = [text("title"), text("address"), text("id")]
        .into_iter()
        .find(|one| !one.is_empty())
        .unwrap_or("");
    let cwd = text("cwd");
    let machine = session["machine"].as_str();
    let gone = machine.is_none() && !(wording.there)(cwd);
    let homely = from_home(cwd, wording.home);
    let side = match machine {
        Some(machine) => words.machine.fill("on", &[("machine", machine)]),
        None if gone => words.reopen_one.fill("gone", &[("cwd", &homely)]),
        None => homely,
    };
    let from = record["from"].as_str().unwrap_or("");
    let (note, host) = match record["way"].as_str() {
        Some("carried") => {
            ways.carried += 1;
            if !ways.machines.iter().any(|one| one == from) {
                ways.machines.push(from.to_owned());
            }
            (words.to_reopen.fill("carried", &[("machine", from)]), "")
        }
        Some("released") => {
            ways.released += 1;
            (words.to_reopen.word("givenBack").to_owned(), from)
        }
        _ => {
            ways.booted += 1;
            (words.to_reopen.word("lastBoot").to_owned(), "")
        }
    };
    json!({"name": name, "side": side, "note": note, "gone": gone, "host": host})
}

/// A folder said from `home`, as `~` and what's under it.
fn from_home(folder: &str, home: &str) -> String {
    if home.is_empty() {
        return folder.to_owned();
    }
    match folder.strip_prefix(home) {
        Some("") => "~".to_owned(),
        Some(under) if under.starts_with('/') => format!("~{under}"),
        _ => folder.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Words {
        Words::from_vocabulary(crate::said::LAUNCHERS).expect("the launcher's words read")
    }

    fn worded_now(offered: &mut [Value], there: &dyn Fn(&str) -> bool) -> Option<Value> {
        let words = words();
        let wording = Wording {
            words: &words,
            home: "/home/skiff",
            there,
        };
        worded(offered, &wording)
    }

    fn rows(offered: &[Value]) -> Vec<Value> {
        offered.iter().map(|one| one["row"].clone()).collect()
    }

    fn reef() -> Value {
        json!({"boot": "the-last", "sessions": [
            {"id": "01-kept", "cwd": "/home/skiff/harbour", "title": "Harbour"},
            {"id": "01-lost", "cwd": "/home/skiff/gone", "address": "GONEAW"},
            {"id": "01-far", "cwd": "", "machine": "lighthouse", "address": "FARAWY",
                "title": "Far", "attach": ["mycelium", "attach", "FARAWY"]},
            {"id": "01-ferried", "cwd": "/home/skiff", "carried": {"from": "far", "at": "1"}},
            {"id": "01-freed", "cwd": "/tmp", "title": "Freed",
                "released": {"from": "attended", "at": "2"}}
        ]})
    }

    #[test]
    fn each_row_says_where_it_reopens_and_how_it_came() {
        let mut all = offered(Some(&reef()), "this-one", &Kept::default(), &[]);
        let place = worded_now(&mut all, &|folder| folder != "/home/skiff/gone");
        assert_eq!(
            rows(&all),
            [
                json!({"name": "Harbour", "side": "~/harbour", "note": "from the last boot",
                    "gone": false, "host": ""}),
                json!({"name": "GONEAW", "side": "~/gone is gone",
                    "note": "from the last boot", "gone": true, "host": ""}),
                json!({"name": "Far", "side": "on lighthouse", "note": "from the last boot",
                    "gone": false, "host": ""}),
                json!({"name": "Freed", "side": "/tmp", "note": "given back by its host",
                    "gone": false, "host": "attended"}),
                json!({"name": "01-ferried", "side": "~", "note": "carried from far",
                    "gone": false, "host": ""}),
            ],
            "named by title, address or id; its folder from home, or gone; its machine; its way"
        );
        assert_eq!(
            place,
            Some(json!({"key": TO_REOPEN, "kind": "to-reopen", "toReopen": {
                "side": "5 sessions",
                "note": "3 from the last boot, 1 carried from far, 1 given back by its host",
                "there": 4, "gone": 1, "lastBoot": 3,
                "allSide": "4 of 5 \u{b7} 1 folder gone",
                "allItems": [{"said": "4 of 5", "keep": 2},
                    {"said": "1 folder gone", "keep": 3, "hue": "alert"}]}})),
            "the place counts them, says where they came from and how many folders are gone"
        );
    }

    #[test]
    fn one_way_alone_is_said_by_itself_and_none_offered_is_no_place() {
        let one = json!({"boot": "the-last", "sessions": [
            {"id": "01-kept", "cwd": "/home/skiff/harbour"}]});
        let mut all = offered(Some(&one), "this-one", &Kept::default(), &[]);
        let place = worded_now(&mut all, &|_| true).expect("one is offered");
        assert_eq!(
            (&place["toReopen"]["side"], &place["toReopen"]["note"]),
            (&json!("1 session"), &json!("from the last boot")),
            "one from the last boot alone says so uncounted"
        );
        assert_eq!(
            (
                &place["toReopen"]["allSide"],
                &place["toReopen"]["allItems"]
            ),
            (&json!(""), &json!([])),
            "with every folder there, Reopen Them All says nothing at its side"
        );
        let given = json!({"boot": "this-one", "sessions": [
            {"id": "01-a", "cwd": "/tmp", "released": {"from": "attended", "at": "1"}},
            {"id": "01-b", "cwd": "/tmp", "released": {"from": "attended", "at": "2"}}]});
        let mut all = offered(Some(&given), "this-one", &Kept::default(), &[]);
        let place = worded_now(&mut all, &|_| true).expect("two are offered");
        assert_eq!(
            place["toReopen"]["note"], "2 given back by its host",
            "those given back are counted even alone, since the way names no place"
        );
        let carried = json!({"boot": "this-one", "sessions": [
            {"id": "01-a", "cwd": "/tmp", "carried": {"from": "far", "at": "1"}},
            {"id": "01-b", "cwd": "/tmp", "carried": {"from": "reef", "at": "2"}}]});
        let mut all = offered(Some(&carried), "this-one", &Kept::default(), &[]);
        let place = worded_now(&mut all, &|_| true).expect("two are offered");
        assert_eq!(
            place["toReopen"]["note"], "carried from 2 machines",
            "several machines are counted rather than listed"
        );
        assert_eq!(
            worded_now(&mut [], &|_| true),
            None,
            "none offered is no place"
        );
    }

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
