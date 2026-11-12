//! What pi's sessions landed, as Agents' Landed lists it, worked out here
//! rather than in the launcher, which only shows what it's given (Joel,
//! 2026-11-10: "the daemons need to be the data layer"): each landing
//! worth Joel's reading, newest first, by the session that landed it, and
//! whether he has read it. What Landed.js worked out of the node's
//! records at each change is worked out once here.
//!
//! - A landing outlives its session: the harness keeps each 30 days,
//!   which the realm's node serves as its `landing` kind. A session still
//!   running may carry landings from before the harness kept them, each
//!   taken once.
//! - One is worth reading when it's a change merged, or says what Joel
//!   should try; a plan item ticked is its own progress, 27 of the 77
//!   landings on 2026-11-07, so it's left out unless it says what to try.
//! - Each is kept by a key of its session's, its ref and when it landed,
//!   so the same change landed again reads anew, and which is how Joel's
//!   reading is kept, by the daemon, in a file of its own (`Reading`).
//! - Its order is its time, newest first, so a landing coming moves no
//!   other's record: two thousand of them were the realm's on 2026-11-11.
//! - How many landed each day, by Joel's own clock, is served beside them,
//!   so Landed's row says today's without the shell counting them all.
//!
//! Times go out as the harness wrote them, and the shell words them
//! (clock.rs).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io;
use std::path::Path;

use serde_json::{Value, json};

use crate::clock;
use crate::served::PLACE;
use crate::session::Session;

/// The key of the record saying how many landed each day.
pub fn days_key() -> String {
    format!("landed-days:{PLACE}")
}

/// The landings Joel has read, by key, and when, in milliseconds; none
/// where he read it before the shell kept when.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Reading {
    pub at: BTreeMap<String, Option<i64>>,
}

impl Reading {
    /// What `path` keeps, or nothing read when it isn't there or can't be
    /// read whole.
    pub fn read(path: &Path) -> Option<Self> {
        let said = fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())?;
        let at = said["read"]
            .as_object()?
            .iter()
            .map(|(key, when)| (key.clone(), when.as_i64()))
            .collect();
        Some(Self { at })
    }

    /// What the launcher kept of Joel's reading before the daemon did, in
    /// any of the shell's own folders under `shells`: its landings-read.json,
    /// `{keys, at}`. Read once, when the daemon has kept nothing yet, so
    /// no landing he had read comes back to For You.
    pub fn from_shell(shells: &Path) -> Self {
        let mut at = BTreeMap::new();
        for folder in fs::read_dir(shells).into_iter().flatten().flatten() {
            let Some(said) = fs::read(folder.path().join("landings-read.json"))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            else {
                continue;
            };
            for key in said["keys"].as_array().into_iter().flatten() {
                if let Some(key) = key.as_str() {
                    let when = said["at"][key].as_i64();
                    let kept: &mut Option<i64> = at.entry(key.to_owned()).or_default();
                    *kept = (*kept).max(when);
                }
            }
        }
        Self { at }
    }

    /// Writes what's read to `path` whole, renamed into place, so a daemon
    /// started again reads it as it was.
    pub fn write(&self, path: &Path) -> io::Result<()> {
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)?;
        }
        let writing = path.with_extension("json.new");
        fs::write(&writing, json!({"read": self.at}).to_string())?;
        fs::rename(&writing, path)
    }

    /// Joel reads `keys` at `now`, keeping of what he read before only
    /// what's still `held`, so what's kept never grows past what's there.
    pub fn mark(&mut self, keys: &[String], now: i64, held: &HashSet<String>) {
        self.at.retain(|key, _| held.contains(key));
        for key in keys {
            self.at.entry(key.clone()).or_insert(Some(now));
        }
    }
}

/// A change as a landing names it, `repo#number`, never a quest
/// document's item, `PLAN-20261106-2ZPAI5#14`.
fn is_change(reference: &str) -> bool {
    let Some((repo, number)) = reference.rsplit_once('#') else {
        return false;
    };
    let named = !repo.is_empty()
        && repo
            .chars()
            .all(|one| one.is_ascii_alphanumeric() || matches!(one, '_' | '.' | '-'));
    let numbered = !number.is_empty() && number.bytes().all(|one| one.is_ascii_digit());
    named && numbered && !is_document(repo)
}

/// A quest document's id, `PLAN-20261106-2ZPAI5`.
fn is_document(name: &str) -> bool {
    let mut parts = name.split('-');
    let (Some(kind), Some(date), Some(id), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    !kind.is_empty()
        && kind.bytes().all(|one| one.is_ascii_uppercase())
        && date.len() == 8
        && date.bytes().all(|one| one.is_ascii_digit())
        && id.len() == 6
        && id
            .bytes()
            .all(|one| one.is_ascii_digit() || one.is_ascii_uppercase())
}

fn text(one: &Value, field: &str) -> String {
    match &one[field] {
        Value::String(said) => said.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Whether a landing is worth Joel's reading.
fn worth_reading(one: &Value) -> bool {
    is_change(&text(one, "ref")) || !text(one, "person").is_empty()
}

/// Whether a landing's line for Joel asks him to try something, so it
/// waits in For You's To Try: any line but one with a part opening with
/// "Nothing", as "Nothing to try", "Nothing changes on screen" and "Core
/// refuses it now; nothing to try" do (VKCF3Z, 2026-11-11), 648 lines
/// opening so and 132 more saying it after what landed, of 1,529 on
/// 2026-11-11. A part saying "but" or "else" still asks, as "nothing
/// looks different, but Return pastes at once" does, the one line of
/// them asking him anything. Read from the words until the harness says
/// it of a landing itself.
fn tries(line: &str) -> bool {
    let says_nothing = |part: &str| {
        let mut words = part
            .split(|letter: char| !letter.is_alphabetic())
            .filter(|word| !word.is_empty());
        words
            .next()
            .is_some_and(|first| first.eq_ignore_ascii_case("nothing"))
            && !words
                .any(|word| word.eq_ignore_ascii_case("but") || word.eq_ignore_ascii_case("else"))
    };
    !line.trim().is_empty() && !line.split(['.', ';', ':', '?', '!']).any(says_nothing)
}

/// Who landed a landing, as its row and drawer say it.
struct Lander {
    address: String,
    title: String,
    id: String,
}

impl Lander {
    fn of(session: Session) -> Self {
        Self {
            address: session.address().to_owned(),
            title: session.named().0,
            id: session.id().to_owned(),
        }
    }

    /// One standing for a session that has ended, by its address and id.
    fn ended(one: &Value) -> Self {
        Self {
            address: text(one, "address"),
            title: String::new(),
            id: text(one, "session"),
        }
    }

    /// The key a landing of this session's is read by, its id's, else its
    /// address's, as the launcher kept it.
    fn key_of(&self, one: &Value) -> String {
        let by = if self.id.is_empty() {
            &self.address
        } else {
            &self.id
        };
        format!("{by} {} {}", text(one, "ref"), text(one, "at"))
    }
}

/// One landing, by the session that landed it.
fn landing(lander: &Lander, one: &Value) -> (String, Option<i64>, Value) {
    let key = lander.key_of(one);
    let url = text(one, "url");
    let what = text(one, "what");
    let at = clock::ms_of(&one["at"]);
    let said = json!({
        "key": key,
        "at": text(one, "at"),
        "ref": text(one, "ref"),
        "what": if what.is_empty() { text(one, "ref") } else { what },
        "person": text(one, "person"),
        "tries": tries(&text(one, "person")),
        "url": if url.starts_with("https://") { url } else { String::new() },
        "quest": text(one, "quest"),
        "repo": text(one, "repo"),
        "machine": text(one, "machine"),
        "session": {"address": lander.address, "title": lander.title, "id": lander.id},
    });
    (key, at, said)
}

/// What sessions landed: the harness's (`kept`, the node's `landing`
/// records) and those the sessions running or parked (`sessions`) still
/// carry, each once, worth Joel's reading. Each is a whole record, keyed
/// by its landing's key, ordered by its time, newest first, saying when
/// Joel read it (`read`), with the record of how many landed each day.
/// Answers the keys held too, which Joel's reading is kept of.
pub fn landed<'a>(
    kept: impl Iterator<Item = &'a Value>,
    sessions: &[Session<'a>],
    read: &Reading,
) -> (Vec<Value>, HashSet<String>) {
    let by_id: HashMap<&str, &Session> = sessions
        .iter()
        .filter(|one| !one.id().is_empty())
        .map(|one| (one.id(), one))
        .collect();
    let mut all: Vec<(String, Option<i64>, Value)> = Vec::new();
    let mut held = HashSet::new();
    for one in kept {
        let lander = by_id
            .get(text(one, "session").as_str())
            .map_or_else(|| Lander::ended(one), |session| Lander::of(**session));
        held.insert(lander.key_of(one));
        if worth_reading(one) {
            all.push(landing(&lander, one));
        }
    }
    for session in sessions {
        let Some(landings) = session
            .presence()
            .and_then(|presence| presence["landings"].as_array())
        else {
            continue;
        };
        let lander = Lander::of(*session);
        for one in landings {
            let key = lander.key_of(one);
            if held.insert(key) && worth_reading(one) {
                all.push(landing(&lander, one));
            }
        }
    }
    let mut days: BTreeMap<String, u64> = BTreeMap::new();
    let mut records: Vec<Value> = all
        .into_iter()
        .map(|(key, at, mut said)| {
            if let Some(at) = at {
                *days.entry(clock::date_of(at)).or_default() += 1;
            }
            // When Joel read it, `true` where he read it before the shell
            // kept when, and null while he hasn't.
            said["read"] = match read.at.get(&key) {
                Some(Some(at)) => json!(at),
                Some(None) => json!(true),
                None => Value::Null,
            };
            let read_by = format!("landed:{PLACE}:{key}");
            json!({"key": read_by, "kind": "landed", "order": -at.unwrap_or(0), "landed": said})
        })
        .collect();
    records.sort_by_key(|record| record["order"].as_i64());
    records.push(json!({"key": days_key(), "kind": "landed-days", "days": days}));
    (records, held)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str, address: &str, landings: &Value) -> Value {
        json!({"standing": "running", "presence": {"session": id, "addresses": [address],
            "questInfo": {"title": format!("The {id} Quest")}, "landings": landings}})
    }

    fn kept(session: &str, reference: &str, at: &str, person: &str) -> Value {
        json!({"session": session, "address": "GONE01", "ref": reference, "at": at,
            "what": format!("Land {reference}"), "person": person, "quest": "QEST-1",
            "repo": "o/r", "machine": "skiff",
            "url": format!("https://example.org/{reference}")})
    }

    fn keys(records: &[Value]) -> Vec<&str> {
        records
            .iter()
            .filter(|one| one["kind"] == "landed")
            .filter_map(|one| one["landed"]["key"].as_str())
            .collect()
    }

    #[test]
    fn what_landed_is_listed_newest_first_each_once_by_who_landed_it() {
        let here = session(
            "s-here",
            "HERE01",
            &json!([
                {"ref": "r#2", "at": "2026-11-11T10:00:00Z", "what": "Two"},
                {"ref": "r#9", "at": "2026-11-11T12:00:00Z", "what": "Nine"},
            ]),
        );
        let sessions = [Session(&here)];
        let landings = [
            kept("s-here", "r#2", "2026-11-11T10:00:00Z", ""),
            kept("s-gone", "r#1", "2026-11-10T09:00:00Z", "Try it."),
        ];
        let (records, held) = landed(landings.iter(), &sessions, &Reading::default());
        assert_eq!(
            keys(&records),
            [
                "s-here r#9 2026-11-11T12:00:00Z",
                "s-here r#2 2026-11-11T10:00:00Z",
                "s-gone r#1 2026-11-10T09:00:00Z",
            ],
            "newest first, the one kept and still carried once"
        );
        assert_eq!(held.len(), 3, "every landing's key held");
        let two = &records[1]["landed"];
        assert_eq!(two["what"], "Land r#2", "the kept landing's own words");
        assert_eq!(
            two["session"],
            json!({"address": "HERE01", "title": "The s-here Quest", "id": "s-here"}),
            "by the session still here"
        );
        assert_eq!(
            records[2]["landed"]["session"],
            json!({"address": "GONE01", "title": "", "id": "s-gone"}),
            "by one standing for the session ended"
        );
        assert!(
            records[0]["order"].as_i64() < records[1]["order"].as_i64(),
            "ordered by time, newest first"
        );
    }

    #[test]
    fn only_a_change_or_one_saying_what_to_try_is_worth_reading() {
        let landings = [
            kept("s", "core.nix#3009", "2026-11-11T01:00:00Z", ""),
            kept("s", "PLAN-20261106-2ZPAI5#14", "2026-11-11T02:00:00Z", ""),
            kept(
                "s",
                "PLAN-20261106-2ZPAI5#15",
                "2026-11-11T03:00:00Z",
                "Open it.",
            ),
            kept("s", "a quest note", "2026-11-11T04:00:00Z", ""),
        ];
        let (records, held) = landed(landings.iter(), &[], &Reading::default());
        assert_eq!(
            keys(&records),
            [
                "s PLAN-20261106-2ZPAI5#15 2026-11-11T03:00:00Z",
                "s core.nix#3009 2026-11-11T01:00:00Z",
            ],
            "a change, and an item saying what to try"
        );
        assert_eq!(held.len(), 4, "those not worth reading are still held");
    }

    #[test]
    fn a_link_is_kept_only_to_the_web_and_what_lacks_words_says_its_ref() {
        let mut one = kept("s", "o#1", "2026-11-11T01:00:00Z", "");
        one["url"] = json!("file:///etc/passwd");
        one["what"] = json!("");
        let (records, _) = landed([one].iter(), &[], &Reading::default());
        assert_eq!(records[0]["landed"]["url"], "", "no link but the web's");
        assert_eq!(
            records[0]["landed"]["what"], "o#1",
            "its ref where it says nothing"
        );
    }

    #[test]
    fn only_a_line_asking_joel_to_try_something_waits_for_him_to() {
        let landings = [
            kept(
                "s",
                "o#1",
                "2026-11-11T01:00:00Z",
                "Open Agents and press :q.",
            ),
            kept(
                "s",
                "o#2",
                "2026-11-11T02:00:00Z",
                "Nothing to try: the net is faster.",
            ),
            kept(
                "s",
                "o#3",
                "2026-11-11T03:00:00Z",
                "nothing changes on screen",
            ),
            kept(
                "s",
                "o#4",
                "2026-11-11T04:00:00Z",
                "Nothingness is a theme now; try it.",
            ),
            kept("s", "o#5", "2026-11-11T05:00:00Z", ""),
        ];
        let (records, _) = landed(landings.iter(), &[], &Reading::default());
        let tries: Vec<&Value> = records[..5]
            .iter()
            .map(|one| &one["landed"]["tries"])
            .collect();
        assert_eq!(
            tries,
            [
                &json!(false),
                &json!(true),
                &json!(false),
                &json!(false),
                &json!(true)
            ],
            "newest first: a line opening with nothing asks nothing, nor does no line"
        );
    }

    #[test]
    fn a_line_saying_nothing_to_try_after_what_landed_asks_nothing_either() {
        let landings = [
            kept(
                "s",
                "o#1",
                "2026-11-11T01:00:00Z",
                "Core refuses it now; nothing to try.",
            ),
            kept(
                "s",
                "o#2",
                "2026-11-11T02:00:00Z",
                "The net is faster. Nothing to try.",
            ),
            kept(
                "s",
                "o#3",
                "2026-11-11T03:00:00Z",
                "Paste from Super+V: nothing looks different, but Return pastes at once.",
            ),
            kept(
                "s",
                "o#4",
                "2026-11-11T04:00:00Z",
                "Try it: open Agents. Nothing else changes.",
            ),
        ];
        let (records, _) = landed(landings.iter(), &[], &Reading::default());
        let tries: Vec<&Value> = records[..4]
            .iter()
            .map(|one| &one["landed"]["tries"])
            .collect();
        assert_eq!(
            tries,
            [&json!(true), &json!(true), &json!(false), &json!(false)],
            "newest first: a later part saying nothing asks nothing, unless it says but or else"
        );
    }

    #[test]
    fn each_says_when_joel_read_it_and_the_days_count_what_landed() {
        let landings = [
            kept("s", "o#1", "2026-11-10T12:00:00Z", ""),
            kept("s", "o#2", "2026-11-11T12:00:00Z", ""),
            kept("s", "o#3", "2026-11-11T13:00:00Z", ""),
        ];
        let mut read = Reading::default();
        read.at.insert("s o#2 2026-11-11T12:00:00Z".into(), Some(5));
        read.at.insert("s o#1 2026-11-10T12:00:00Z".into(), None);
        let (records, _) = landed(landings.iter(), &[], &read);
        let said: Vec<&Value> = records.iter().map(|one| &one["landed"]["read"]).collect();
        assert_eq!(
            said[..3],
            [&json!(null), &json!(5), &json!(true)],
            "not read, read at 5, or read before the shell kept when"
        );
        let days = &records[3];
        assert_eq!(days["key"], "landed-days:agents");
        let counted: u64 = days["days"]
            .as_object()
            .expect("days by date")
            .values()
            .filter_map(Value::as_u64)
            .sum();
        assert_eq!(counted, 3, "each landing counted on its day");
    }

    #[test]
    fn what_joel_read_is_kept_only_while_its_landing_is() {
        let folder = std::env::temp_dir().join(format!("agents-read-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        let mut read = Reading::default();
        read.at.insert("gone".into(), Some(1));
        read.at.insert("still".into(), Some(2));
        let held: HashSet<String> = ["still", "new"].map(str::to_owned).into();
        read.mark(&["new".into(), "still".into()], 9, &held);
        assert_eq!(
            read.at,
            BTreeMap::from([("new".into(), Some(9)), ("still".into(), Some(2))]),
            "read again keeps its first time, and the gone are let go"
        );
        let path = folder.join("agents").join("landed.json");
        read.write(&path).expect("written");
        assert_eq!(Reading::read(&path), Some(read), "read back as written");
        assert_eq!(
            Reading::read(&folder.join("none.json")),
            None,
            "nothing kept"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn what_the_launcher_kept_of_joels_reading_is_taken_once() {
        let shells = std::env::temp_dir().join(format!("agents-shells-{}", std::process::id()));
        let _ = fs::remove_dir_all(&shells);
        fs::create_dir_all(shells.join("one")).expect("a shell's folder");
        fs::create_dir_all(shells.join("two")).expect("another's");
        fs::write(
            shells.join("one").join("landings-read.json"),
            r#"{"keys": ["a", "b"], "at": {"a": 7}}"#,
        )
        .expect("one shell's reading");
        fs::write(
            shells.join("two").join("landings-read.json"),
            r#"{"keys": ["a"], "at": {"a": 3}}"#,
        )
        .expect("another's");
        let read = Reading::from_shell(&shells);
        assert_eq!(
            read.at,
            BTreeMap::from([("a".into(), Some(7)), ("b".into(), None)]),
            "every shell's keys, each at its latest time"
        );
        assert_eq!(
            Reading::from_shell(&shells.join("none")),
            Reading::default(),
            "no shell, nothing read"
        );
        let _ = fs::remove_dir_all(&shells);
    }
}
