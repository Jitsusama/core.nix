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
//! - Each landing's drawer is a record of its own (`DRAWER`), the landing
//!   naming it by its key, told only to a watch naming it, as the shell
//!   does for the drawers it's about to draw. The drawers were 1.3 MB of
//!   the shell's 3.1 MB of landings, held for 2,290 rows of which Joel
//!   reads a few (BXNMUC's heap study, 2026-11-13).
//! - Those waiting in For You's To Try, and those read since, for its
//!   History, are served as `TRY` records naming their landings, so For
//!   You reads the few rather than sifting all of them at each change.
//!
//! Times go out as the harness wrote them, and the shell words them
//! (clock.rs).
//!
//! Long because its cases are most of it: each of what a landing, its
//! drawer and its try say is held to one, and the making is one pass
//! over the landings that splitting would make two files reading alike.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::hash::BuildHasher;
use std::io;
use std::path::Path;

use serde_json::{Value, json};

use crate::clock;
use crate::row::Words;
use crate::served::PLACE;
use crate::session::Session;

/// The kind of a landing's drawer, served apart from it.
pub const DRAWER: &str = "landed-drawer";

/// The kind of a landing For You holds, in To Try or in its History.
pub const TRY: &str = "try";

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
/// waits in For You's To Try and is named by it: one naming something
/// for him to do (`acts`), but not one with a part opening with
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
    !line.trim().is_empty()
        && !line.split(['.', ';', ':', '?', '!']).any(says_nothing)
        && acts(line)
}

/// The verbs a part of a line opens with when it asks Joel to do
/// something, as "Open Machine › Heat", "After the next switch, open
/// Agents" or "Try clock-follows in a terminal" do. Of the 122 lines
/// landed on 2026-11-13 (UTC) that asked something before this was read,
/// 70 named nothing for him to do, as "Terminal sessions held by mycelium
/// now let go about 25 ms after their program ends" doesn't, and To Try
/// was those to sift
/// (Joel, 2026-11-13: "hard to see what matters most"); they stay in
/// Landed. A line written to the rule of 2026-11-13 (WCJQSG, ecd3b7c2)
/// opens with one.
const ACTS: [&str; 40] = [
    "ask", "call", "change", "check", "choose", "click", "close", "copy", "dictate", "drag",
    "find", "give", "go", "hold", "join", "launch", "listen", "look", "move", "open", "paste",
    "pick", "play", "plug", "press", "put", "read", "reboot", "reopen", "run", "say", "scroll",
    "select", "send", "speak", "start", "step", "talk", "try", "type",
];

/// Whether `line` names something for Joel to do: a part of it, between
/// its stops and commas or after an "and" or "then", opens with one of
/// `ACTS`.
fn acts(line: &str) -> bool {
    line.split(['.', ';', ':', '?', '!', ','])
        .flat_map(|part| part.split(" and ").flat_map(|part| part.split(" then ")))
        .filter_map(|part| {
            part.split(|letter: char| !letter.is_alphabetic() && letter != '\'')
                .find(|word| !word.is_empty())
        })
        .any(|first| ACTS.iter().any(|act| first.eq_ignore_ascii_case(act)))
}

/// A line's first sentence, its stop left off, and what follows it: a
/// stop ends one only before a space or the line's end, so one in
/// "core.nix" doesn't.
fn first_sentence(line: &str) -> (&str, &str) {
    let line = line.trim();
    let ends = line.char_indices().find(|&(at, letter)| {
        matches!(letter, '.' | '?' | '!')
            && line[at + letter.len_utf8()..]
                .chars()
                .next()
                .is_none_or(char::is_whitespace)
    });
    match ends {
        Some((at, '.')) => (&line[..at], line[at + 1..].trim_start()),
        Some((at, letter)) => (
            &line[..at + letter.len_utf8()],
            line[at + letter.len_utf8()..].trim_start(),
        ),
        None => (line, ""),
    }
}

/// How long a landing asking Joel to try something waits in To Try, from
/// when it landed: past it, it's still in Landed, but no longer counted
/// as waiting on him. On 2026-11-11, 778 unread landings asked him to try
/// something "after the next switch", from as far back as 8 Nov, and
/// nobody tries 778 things; an ask he took too long on should go (Joel,
/// 2026-11-11: "if it is no longer applicable because i took too long
/// and things have changed, it should disappear"). A day holds about 50.
/// It's weighed each time the rows are made, which a session changing
/// does every few seconds.
const TRY_FOR_MS: i64 = 24 * 60 * 60 * 1000;

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
fn landing(lander: &Lander, one: &Value, now: i64) -> (String, Option<i64>, Value) {
    let key = lander.key_of(one);
    let url = text(one, "url");
    let what = text(one, "what");
    let what = if what.is_empty() {
        text(one, "ref")
    } else {
        what
    };
    let person = text(one, "person");
    let at = clock::ms_of(&one["at"]);
    // Named by what it asks Joel to do, the line's first sentence, where
    // it asks anything, since that's what he scans To Try for; else by
    // what landed (VKCF3Z, c2d01d53). The same wherever it's listed, as
    // one thing has one name.
    let name = if tries(&person) {
        first_sentence(&person).0.to_owned()
    } else {
        what.clone()
    };
    let said = json!({
        "key": key,
        "at": text(one, "at"),
        "ref": text(one, "ref"),
        "name": name,
        "what": what,
        "person": person,
        "tries": tries(&person) && at.is_some_and(|at| now - at < TRY_FOR_MS),
        "url": if url.starts_with("https://") { url } else { String::new() },
        "quest": text(one, "quest"),
        "repo": text(one, "repo"),
        "machine": text(one, "machine"),
        "session": {"address": lander.address, "title": lander.title, "id": lander.id},
    });
    (key, at, said)
}

/// A landing's `try` record, where it waits in To Try or was read since.
fn tried(record: &Value) -> Option<Value> {
    let said = &record["landed"];
    (said["tries"] == true && !said["read"].is_boolean()).then(|| {
        json!({"key": format!("{TRY}:{PLACE}:{}", text(said, "key")), "kind": TRY,
            "order": record["order"], "try": {"landing": record["key"], "read": said["read"]}})
    })
}

/// Gives each landing in `records` its drawer, the same in Landed and in
/// For You, as the shell once worked out for itself (Landed.js): who
/// landed it and that it's a landing as its eyebrow; what landed as its
/// title; when it landed, its repository and machine as facts; what to
/// try as its words, since a sentence never fits beside the row (VKCF3Z,
/// 2026-11-10). Who landed it is named as Agents names it elsewhere, by
/// its session's title, else, for a session ended, by the title of the
/// quest it landed for (`titles`, by the quest's id), never by an
/// address; its ref and its quest's id are nerdy bits, folded away
/// where y copies them, since no identifier stands among what Joel
/// reads (VKCF3Z, 2026-11-12). Its time goes out for the shell to word.
/// Each drawer is a record of its own, `landed-drawer:agents:<key>`, its
/// landing naming it by that key in `drawer`.
pub fn drawn<S: BuildHasher>(
    records: &mut Vec<Value>,
    words: &Words,
    titles: &HashMap<&str, &str, S>,
) {
    let mut apart = Vec::new();
    for record in records.iter_mut().filter(|one| one["kind"] == "landed") {
        let key = format!("{DRAWER}:{PLACE}:{}", text(&record["landed"], "key"));
        let drawer = drawer(&record["landed"], words, titles);
        record["landed"]["drawer"] = json!(key);
        apart.push(json!({"key": key, "kind": DRAWER, "drawer": drawer}));
    }
    records.extend(apart);
}

fn drawer<S: BuildHasher>(one: &Value, words: &Words, titles: &HashMap<&str, &str, S>) -> Value {
    let named = |id: &str| words.names.word(id).to_owned();
    let quest = text(one, "quest");
    // A running session with no title is named by its address, which
    // Joel never reads (VKCF3Z, ee3bac47), so it's named by its quest too.
    let session = one["session"]["title"]
        .as_str()
        .filter(|title| *title != one["session"]["address"])
        .unwrap_or("");
    let who = if session.is_empty() {
        titles.get(quest.as_str()).copied().unwrap_or("")
    } else {
        session
    };
    let reference = text(one, "ref");
    let mut facts: Vec<Value> = clock::ms_of(&one["at"])
        .map(|at| json!({"label": named("landed"), "value": "", "at": {"at": at, "as": "dated"}}))
        .into_iter()
        .collect();
    let said = |pairs: Vec<(&str, String)>| {
        pairs
            .into_iter()
            .filter(|(_, value)| !value.is_empty())
            .map(|(fact, value)| json!({"label": named(fact), "value": value}))
            .collect::<Vec<_>>()
    };
    let kind = if is_change(&reference) {
        "change"
    } else {
        "item"
    };
    let eyebrow = json!([{"said": who, "keep": 2}, {"said": words.landed.word("kind"), "keep": 1}]);
    let person = text(one, "person");
    if !tries(&person) {
        facts.extend(said(vec![
            ("repo", text(one, "repo")),
            ("machine", text(one, "machine")),
        ]));
        return json!({
            "title": text(one, "what"),
            "eyebrow": eyebrow,
            "facts": facts,
            "body": person,
            "nerd": said(vec![(kind, reference), ("task", quest)]),
        });
    }
    // One asking Joel to try something leads with it, as its row is named,
    // whole on as many lines as it takes, and the rest of the line after
    // as prose, never cut, since it's one line by contract rather than a
    // log; what landed is the one fact after when, and where it landed
    // folds with its ids (Joel, 2026-11-13: "it truncates text, make me
    // cursor in and manually scroll"; VKCF3Z, c2d01d53).
    let (lead, rest) = first_sentence(&person);
    facts.extend(said(vec![("changed", text(one, "what"))]));
    json!({
        "title": lead,
        "whole": true,
        "prose": true,
        "eyebrow": eyebrow,
        "facts": facts,
        "body": rest,
        "nerd": said(vec![
            (kind, reference),
            ("task", quest),
            ("repo", text(one, "repo")),
            ("machine", text(one, "machine")),
        ]),
    })
}

/// What sessions landed: the harness's (`kept`, the node's `landing`
/// records) and those the sessions running or parked (`sessions`) still
/// carry, each once, worth Joel's reading. Each is a whole record, keyed
/// by its landing's key, ordered by its time, newest first, saying when
/// Joel read it (`read`), with the record of how many landed each day.
/// Answers the keys held too, which Joel's reading is kept of. Those
/// asking him to try something wait in To Try a day from `now`, in
/// milliseconds since 1970, each with a `try` record naming it: `read`
/// null while it waits, and when he read it once he has, for History.
/// One read before when was kept has no time to say, so it's neither's.
pub fn landed<'a>(
    kept: impl Iterator<Item = &'a Value>,
    sessions: &[Session<'a>],
    read: &Reading,
    now: i64,
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
            all.push(landing(&lander, one, now));
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
                all.push(landing(&lander, one, now));
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
    let tries: Vec<Value> = records.iter().filter_map(tried).collect();
    records.extend(tries);
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

    /// The moment the tests' landings are read at, a day after the first
    /// of them landed, so all of them ask what they ask.
    fn now() -> i64 {
        clock::ms_of(&json!("2026-11-11T12:00:00Z")).expect("a time")
    }

    #[test]
    fn a_landing_waits_in_to_try_a_day_then_goes_quiet_in_landed() {
        let landings = [
            kept(
                "s",
                "o#1",
                "2026-11-10T11:59:00Z",
                "After the next switch, open Agents.",
            ),
            kept(
                "s",
                "o#2",
                "2026-11-10T12:01:00Z",
                "After the next switch, open Agents.",
            ),
        ];
        let (records, _) = landed(landings.iter(), &[], &Reading::default(), now());
        let tries: Vec<(&Value, &Value)> = records[..2]
            .iter()
            .map(|one| (&one["landed"]["ref"], &one["landed"]["tries"]))
            .collect();
        assert_eq!(
            tries,
            [
                (&json!("o#2"), &json!(true)),
                (&json!("o#1"), &json!(false))
            ],
            "newest first: what landed within the day still asks, the one before is only landed"
        );
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
        let (records, held) = landed(landings.iter(), &sessions, &Reading::default(), now());
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
        let (records, held) = landed(landings.iter(), &[], &Reading::default(), now());
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
        let (records, _) = landed([one].iter(), &[], &Reading::default(), now());
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
        let (records, _) = landed(landings.iter(), &[], &Reading::default(), now());
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
        let (records, _) = landed(landings.iter(), &[], &Reading::default(), now());
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
        let (records, _) = landed(landings.iter(), &[], &read, now());
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

    fn words() -> Words {
        Words::from_vocabulary(crate::said::LAUNCHERS).expect("the launcher's words read")
    }

    #[test]
    fn a_running_session_with_no_title_is_named_by_its_quest_never_its_address() {
        let here = json!({"standing": "running",
            "presence": {"session": "s-bare", "addresses": ["BARE01"], "landings": []}});
        let sessions = [Session(&here)];
        let landings = [kept(
            "s-bare",
            "core.nix#2141",
            "2026-11-11T10:00:00Z",
            "Try it.",
        )];
        let (mut records, _) = landed(landings.iter(), &sessions, &Reading::default(), now());
        let titles = HashMap::from([("QEST-1", "The Lighthouse Quest")]);
        drawn(&mut records, &words(), &titles);
        assert_eq!(
            drawer_of(&records, 0)["eyebrow"][0]["said"],
            "The Lighthouse Quest",
            "a session named only by its address is named by the quest it landed for"
        );
    }

    #[test]
    fn a_landings_drawer_names_who_landed_it_and_folds_its_identifiers() {
        let here = session("s-here", "HERE01", &json!([]));
        let sessions = [Session(&here)];
        let mut bare = kept(
            "s-gone",
            "PLAN-20261106-2ZPAI5#4",
            "2026-11-10T09:00:00Z",
            "Try it.",
        );
        bare["quest"] = json!("QEST-UNKNOWN");
        bare["repo"] = json!("");
        let landings = [
            kept(
                "s-here",
                "core.nix#2140",
                "2026-11-11T10:00:00Z",
                "Nothing changes on screen.",
            ),
            kept("s-gone", "core.nix#2138", "2026-11-11T09:00:00Z", "Try it."),
            bare,
        ];
        let (mut records, _) = landed(landings.iter(), &sessions, &Reading::default(), now());
        let titles = HashMap::from([("QEST-1", "The Lighthouse Quest")]);
        drawn(&mut records, &words(), &titles);
        let at = clock::ms_of(&json!("2026-11-11T10:00:00Z")).expect("a time");
        assert_eq!(
            drawer_of(&records, 0),
            &json!({
                "title": "Land core.nix#2140",
                "eyebrow": [{"said": "The s-here Quest", "keep": 2},
                    {"said": "landing", "keep": 1}],
                "facts": [
                    {"label": "Landed", "value": "", "at": {"at": at, "as": "dated"}},
                    {"label": "Repository", "value": "o/r"},
                    {"label": "Machine", "value": "skiff"},
                ],
                "body": "Nothing changes on screen.",
                "nerd": [{"label": "Change", "value": "core.nix#2140"},
                    {"label": "Quest", "value": "QEST-1"}],
            }),
            "who landed it by its session's title, when as a time the shell words, its ids folded"
        );
        assert_eq!(
            drawer_of(&records, 1)["eyebrow"][0]["said"],
            "The Lighthouse Quest",
            "a session ended named by the quest it landed for, never its address"
        );
        let unknown = drawer_of(&records, 2);
        assert_eq!(
            (&unknown["eyebrow"][0]["said"], &unknown["nerd"][0]["label"]),
            (&json!(""), &json!("Item")),
            "one whose quest isn't known names nobody, and a plan's ref is an item"
        );
        assert_eq!(
            unknown["facts"].as_array().map(Vec::len),
            Some(2),
            "what isn't known isn't said"
        );
        assert!(
            records
                .iter()
                .filter(|one| one["kind"] != "landed" && one["kind"] != DRAWER)
                .all(|one| one.get("drawer").is_none()),
            "the days' count is drawn no drawer"
        );
        assert_eq!(
            records.iter().filter(|one| one["kind"] == DRAWER).count(),
            3,
            "each landing's drawer is a record of its own"
        );
    }

    #[test]
    fn a_landing_asking_joel_something_leads_with_it_whole_and_folds_where_it_landed() {
        let here = session("s-here", "HERE01", &json!([]));
        let sessions = [Session(&here)];
        let landings = [kept(
            "s-here",
            "core.nix#2140",
            "2026-11-11T10:00:00Z",
            "Open Machine \u{203a} Heat; it costs the laptop less now. It reads slow chips every \
             five seconds.",
        )];
        let (mut records, _) = landed(landings.iter(), &sessions, &Reading::default(), now());
        drawn(&mut records, &words(), &HashMap::<&str, &str>::new());
        let at = clock::ms_of(&json!("2026-11-11T10:00:00Z")).expect("a time");
        assert_eq!(
            records[0]["landed"]["name"],
            "Open Machine \u{203a} Heat; it costs the laptop less now",
            "named by what it asks, its first sentence without its stop"
        );
        assert_eq!(
            drawer_of(&records, 0),
            &json!({
                "title": "Open Machine \u{203a} Heat; it costs the laptop less now",
                "whole": true,
                "prose": true,
                "eyebrow": [{"said": "The s-here Quest", "keep": 2},
                    {"said": "landing", "keep": 1}],
                "facts": [
                    {"label": "Landed", "value": "", "at": {"at": at, "as": "dated"}},
                    {"label": "What Changed", "value": "Land core.nix#2140"},
                ],
                "body": "It reads slow chips every five seconds.",
                "nerd": [{"label": "Change", "value": "core.nix#2140"},
                    {"label": "Quest", "value": "QEST-1"},
                    {"label": "Repository", "value": "o/r"},
                    {"label": "Machine", "value": "skiff"}],
            }),
            "its title is its name, whole; the rest of the line its prose; what landed its one \
             fact after when; where it landed folded with its ids"
        );
    }

    #[test]
    fn a_line_naming_nothing_for_joel_to_do_isnt_his_to_try_and_is_named_by_what_landed() {
        let landings = [
            kept(
                "s",
                "o#1",
                "2026-11-11T11:00:00Z",
                "Terminal sessions held by mycelium now let go about 25 ms after their program \
                 ends.",
            ),
            kept(
                "s",
                "o#2",
                "2026-11-11T10:00:00Z",
                "After the next switch, open Agents.",
            ),
            kept(
                "s",
                "o#3",
                "2026-11-11T09:00:00Z",
                "Once core pins it, type ergodox --help and then press Return.",
            ),
        ];
        let (records, _) = landed(landings.iter(), &[], &Reading::default(), now());
        let named: Vec<(&Value, &Value)> = records[..3]
            .iter()
            .map(|one| (&one["landed"]["name"], &one["landed"]["tries"]))
            .collect();
        assert_eq!(
            named,
            [
                (&json!("Land o#1"), &json!(false)),
                (&json!("After the next switch, open Agents"), &json!(true)),
                (
                    &json!("Once core pins it, type ergodox --help and then press Return"),
                    &json!(true)
                ),
            ],
            "only a line with a part opening with an act asks, and is named by it"
        );
    }

    #[test]
    fn a_lines_first_sentence_ends_only_at_a_stop_before_a_space() {
        assert_eq!(
            first_sentence("Try core.nix's help in a terminal. Then more."),
            ("Try core.nix's help in a terminal", "Then more."),
            "a stop inside a word goes on; the first before a space ends it, left off"
        );
        assert_eq!(
            first_sentence("Does it wake? It should."),
            ("Does it wake?", "It should."),
            "a question keeps its mark"
        );
        assert_eq!(
            first_sentence("Open Agents"),
            ("Open Agents", ""),
            "one with no stop is all its first sentence"
        );
    }

    /// The drawer the `at`th landing names, served apart from it.
    fn drawer_of(records: &[Value], at: usize) -> &Value {
        let key = &records[at]["landed"]["drawer"];
        assert!(
            key.is_string(),
            "a landing names its drawer by its key, not {key}"
        );
        &records
            .iter()
            .find(|one| one["key"] == *key && one["kind"] == DRAWER)
            .expect("the drawer a landing names is served")["drawer"]
    }

    #[test]
    fn each_landing_for_you_holds_is_a_try_naming_it_and_when_it_was_read() {
        let try_it = "After the next switch, open Agents.";
        let landings = [
            kept("s", "o#1", "2026-11-11T11:00:00Z", try_it),
            kept("s", "o#2", "2026-11-11T10:00:00Z", try_it),
            kept("s", "o#3", "2026-11-11T09:00:00Z", try_it),
            kept("s", "o#4", "2026-11-11T08:00:00Z", "Nothing to try."),
        ];
        let read = Reading {
            at: BTreeMap::from([
                ("s o#2 2026-11-11T10:00:00Z".to_owned(), Some(1000)),
                ("s o#3 2026-11-11T09:00:00Z".to_owned(), None),
            ]),
        };
        let (records, _) = landed(landings.iter(), &[], &read, now());
        let tries: Vec<(&Value, &Value)> = records
            .iter()
            .filter(|one| one["kind"] == TRY)
            .map(|one| (&one["try"]["landing"], &one["try"]["read"]))
            .collect();
        assert_eq!(
            tries,
            [
                (
                    &json!("landed:agents:s o#1 2026-11-11T11:00:00Z"),
                    &Value::Null
                ),
                (
                    &json!("landed:agents:s o#2 2026-11-11T10:00:00Z"),
                    &json!(1000)
                ),
            ],
            "the one waiting, then the one read with when; none read before when was kept, \
             none asking nothing"
        );
    }
}
