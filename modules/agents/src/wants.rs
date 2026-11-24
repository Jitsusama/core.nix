//! What each session asks Joel, as For You's Needs You lists it: one
//! `want` record per session running or parked whose want is truly his
//! (`session::asks_joel`), never one whose want is an action, since that's
//! one of its own items, which For You lists itself (Holding.wanting).
//! Each says what it asked, since when, whether it's parked, and the
//! answers it offers to a single question (harness #1394), the one it
//! recommends first, so the shell reads them rather than parsing every
//! session's presence (KRINS0 item 2). Answering is asked of the daemon
//! too (`answer_of`), which finds the want by the session's address, so
//! the shell never names a want's id. Each says too whether it rings the
//! shell's bell (`rings`) and who it is (`known`), by which the shell
//! rings once for each (Sessions.qml's stopped), so the shell judges no
//! want itself (KRINS0 item 4, ending #3853's stopgap). Joel may put one
//! off for an hour (`PutOff`), and it says so and rings for none of it, so
//! the shell leaves it out of what needs him until then.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde_json::{Value, json};

use crate::acts;
use crate::clock;
use crate::session::{self, JoelCan, Lives, Session};

/// The kind of a session's want record.
pub const KIND: &str = "want";

/// The place its keys are under, Agents'.
const PLACE: &str = "agents";

/// The want records served last, for answering one: shared between the
/// daemon's loop, which holds them, and its asks.
#[derive(Clone, Default)]
pub struct Wants(Arc<Mutex<Vec<Value>>>);

impl Wants {
    /// Holds the wants among `records` from now on.
    pub fn hold<'a>(&self, records: impl Iterator<Item = &'a Value>) {
        let held = records
            .filter(|record| record["kind"] == KIND)
            .cloned()
            .collect();
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = held;
    }

    /// Answers a want, steered through the node at `node`, refused where
    /// there's none; None for any ask but an answer.
    pub fn answer(&self, said: &Value, node: Option<&Path>, now_ms: u128) -> Option<Value> {
        let held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let steer = answer_of(said, &held, now_ms)?;
        drop(held);
        Some(match (steer, node) {
            (Err(why), _) => json!({"error": {"code": "not-an-ask", "message": why}}),
            (Ok(steer), Some(node)) => acts::done(Ok(steer), node),
            (Ok(_), None) => json!({"error": {"code": "unsteered",
                "message": "this machine runs no node to steer it through"}}),
        })
    }

    /// Who the want `{ "ask": "not-now", "address", "known" }` names is
    /// (`known`), refused where it names no ask Joel saw, where its session
    /// wants nothing, or where it asks other than the `known` he saw, which
    /// still needs him; None for any other ask.
    pub fn put_off(&self, said: &Value) -> Option<Result<String, String>> {
        if said["ask"] != "not-now" {
            return None;
        }
        let address = said["address"].as_str().unwrap_or("").to_uppercase();
        if address.is_empty() {
            return Some(Err("not-now names no session's address".to_owned()));
        }
        let Some(seen) = said["known"].as_str().filter(|seen| !seen.is_empty()) else {
            return Some(Err("not-now names no ask Joel saw".to_owned()));
        };
        let held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let known = held
            .iter()
            .find(|one| one["address"].as_str().unwrap_or("").to_uppercase() == address)
            .and_then(|one| one["known"].as_str())
            .map(str::to_owned);
        Some(match (known, seen) {
            (None, _) => Err(format!("{address} wants nothing of Joel to put off")),
            (Some(known), seen) if known != seen => Err(format!(
                "{address} asks something new, which still needs Joel"
            )),
            (Some(known), _) => Ok(known),
        })
    }
}

/// How long Not Now puts a want off: an hour, after which it needs Joel
/// again if it still stands (WCJQSG, 2026-11-16, on Joel's "i can't get
/// this highlighted notice to go away" of a yes waiting five hours).
pub const PUT_OFF_FOR_MS: u128 = 3_600_000;

/// The file beside the daemon's others that keeps the wants put off, so a
/// daemon started again within the hour still holds them off.
pub const PUT_OFF_FILE: &str = "wants-put-off.json";

/// The wants Joel put off, by who each is (`known`), each until when, in
/// milliseconds.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PutOff(BTreeMap<String, u128>);

impl PutOff {
    /// What `path` keeps at `now_ms`, each held at most an hour from then
    /// whatever the file says or the clock did, or none put off when it
    /// isn't there or can't be read whole.
    pub fn read(path: &Path, now_ms: u128) -> Self {
        let said: Value = fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or(Value::Null);
        let kept = said.as_object().map(|by| {
            by.iter()
                .filter_map(|(known, until)| {
                    let until = u128::from(until.as_u64()?).min(now_ms + PUT_OFF_FOR_MS);
                    Some((known.clone(), until))
                })
                .collect()
        });
        Self(kept.unwrap_or_default())
    }

    /// Writes what's put off to `path` whole, renamed into place.
    pub fn write(&self, path: &Path) -> io::Result<()> {
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)?;
        }
        let writing = path.with_extension("json.new");
        let said: serde_json::Map<String, Value> = self
            .0
            .iter()
            .map(|(known, until)| (known.clone(), json!(u64::try_from(*until).unwrap_or(0))))
            .collect();
        fs::write(&writing, Value::Object(said).to_string())?;
        fs::rename(&writing, path)
    }

    /// Joel puts off the want `known` at `now_ms`, for an hour; those whose
    /// hour is over are let go, so what's kept never grows.
    pub fn put(&mut self, known: &str, now_ms: u128) {
        self.0.retain(|_, until| *until > now_ms);
        self.0.insert(known.to_owned(), now_ms + PUT_OFF_FOR_MS);
    }

    /// Whether the want `known` is put off at `now_ms`.
    pub fn holds(&self, known: &str, now_ms: u128) -> bool {
        self.0.get(known).is_some_and(|until| *until > now_ms)
    }

    /// Whether what `one` wants of Joel now is put off at `now_ms`.
    pub fn holds_for(&self, one: Session, now_ms: u128) -> bool {
        one.presence()
            .and_then(|presence| presence.get("wants"))
            .filter(|wants| wants.is_object())
            .is_some_and(|wants| self.holds(&known(one.address(), wants), now_ms))
    }

    /// How long from `now_ms` until the soonest want still put off needs
    /// Joel again, so its record is made anew then, and at most a minute,
    /// since a sleep stops the wait's clock but not the hour's; none while
    /// none is.
    pub fn wait(&self, now_ms: u128) -> Option<Duration> {
        self.0
            .values()
            .filter(|until| **until > now_ms)
            .min()
            .map(|until| Duration::from_millis(u64::try_from(until - now_ms).unwrap_or(u64::MAX)))
            .map(|wait| wait.min(Duration::from_mins(1)))
    }
}

/// A want record per session asking Joel, the oldest ask first, one Joel
/// put off saying so at `now_ms`.
pub fn records(
    sessions: &[Session],
    joel_can: &JoelCan,
    (put_off, now_ms): (&PutOff, u128),
) -> Vec<Value> {
    let mut wanting: Vec<(String, Value)> = sessions
        .iter()
        .filter(|one| matches!(one.lives(), Lives::Running | Lives::Parked))
        .filter(|one| session::asks_joel(one.presence(), joel_can))
        .filter_map(|one| {
            let wants = one.presence()?.get("wants")?;
            if wants["kind"] == "action" {
                return None;
            }
            let title = one.named().0;
            let asked = wants["said"]
                .as_str()
                .filter(|said| !said.is_empty())
                .map_or_else(|| title.clone(), str::to_owned);
            let since = wants["since"].as_str().unwrap_or("").to_owned();
            let known = known(one.address(), wants);
            let held_off = put_off.holds(&known, now_ms);
            let record = json!({
                "key": format!("{KIND}:{PLACE}:{}", one.address()),
                "kind": KIND,
                "address": one.address(),
                "title": title,
                "asked": asked,
                // The want's own kind and words, as the bell's notice says
                // them, since the presence may let them go before this does.
                "wants": wants["kind"].as_str().unwrap_or(""),
                "said": wants["said"].as_str().unwrap_or(""),
                "whole": whole(wants),
                "since": since,
                "parked": one.lives() == Lives::Parked,
                "answers": answers(wants),
                "rings": rings(wants) && !held_off,
                "putOff": held_off,
                "known": known,
            });
            Some((since, record))
        })
        .collect();
    wanting.sort_by(|a, b| a.0.cmp(&b.0));
    wanting.into_iter().map(|(_, record)| record).collect()
}

/// The most of a want's whole question read, in characters, as the
/// harness writes it: one past this is none of its writing, so it's left
/// for the short line (85PKB2, 2026-11-24).
const WHOLE_AT_MOST: usize = 4000;

/// What a want asks in full, as the harness keeps it beside its short
/// line (`asked`: paragraphs, list lines and indentation kept), so Talk and
/// For You show the whole question with its choices (Joel's bar of
/// 2026-11-24, that any surface he is in answers a question in full); none
/// where it says no more than its short line, or past `WHOLE_AT_MOST`.
fn whole(wants: &Value) -> &str {
    wants["asked"]
        .as_str()
        .filter(|asked| !asked.trim().is_empty())
        .filter(|asked| asked.chars().count() <= WHOLE_AT_MOST)
        .unwrap_or("")
}

/// Whether a want stops its session on Joel's keys now, a question or a
/// yes, which the shell's bell rings for: never a run that ended blocked
/// while he was away, which waits for him to look (WCJQSG's pivot,
/// 2026-11-12, on Joel's "my computer constantly dinging instead of being
/// brought to what i need to interact with").
fn rings(wants: &Value) -> bool {
    matches!(wants["kind"].as_str(), Some("question" | "approval"))
}

/// Who a want is, whatever its words say as they count up, so one
/// rewording itself doesn't ring again: its session's address and the
/// harness's own id for it, else its kind and when it began, and its
/// words only when it says no start.
fn known(address: &str, wants: &Value) -> String {
    if let Some(id) = wants["id"].as_str().filter(|id| !id.is_empty()) {
        return format!("{address} {id}");
    }
    let kind = wants["kind"].as_str().unwrap_or("");
    let begun = wants["since"]
        .as_str()
        .or_else(|| wants["said"].as_str())
        .unwrap_or("");
    format!("{address} {kind} {begun}")
}

/// What a want offers, as the shell once read it for itself: null when
/// it offers no answers, else the want's id as the session knows it,
/// `<kind>@<since>`, its answers with the one it recommends first and the
/// rest in its order, each `{ value, label, description, recommended }`,
/// and whether it takes Joel's own words.
fn answers(wants: &Value) -> Value {
    let ask = &wants["ask"];
    let (Some(given), Some(since)) = (ask["choices"].as_array(), clock::ms_of(&wants["since"]))
    else {
        return Value::Null;
    };
    let recommended = ask["recommended"].as_str().unwrap_or("");
    let choices: Vec<Value> = given
        .iter()
        .filter_map(|one| {
            let value = one["value"].as_str().filter(|value| !value.is_empty())?;
            let label = one["label"]
                .as_str()
                .filter(|label| !label.is_empty())
                .unwrap_or(value);
            Some(json!({
                "value": value,
                "label": label,
                "description": one["description"].as_str().unwrap_or(""),
                "recommended": !recommended.is_empty() && value == recommended,
            }))
        })
        .collect();
    if choices.is_empty() {
        return Value::Null;
    }
    let (first, rest): (Vec<Value>, Vec<Value>) = choices
        .into_iter()
        .partition(|one| one["recommended"] == true);
    json!({
        "want": format!("{}@{}", wants["kind"].as_str().unwrap_or(""), clock::text_of(since)),
        "choices": first.into_iter().chain(rest).collect::<Vec<_>>(),
        "own": ask["allowOther"] == true,
    })
}

/// The node's ask that answers a session's want, as `acts::done` takes
/// it, from `{ "ask": "answer", "address", "value" }` or `"other"` for
/// Joel's own words: the want found among `wants` by its address, so a
/// want gone or answered since is refused rather than answered, as is one
/// other than the `want` the answer names where it names one, so words
/// given to what he was shown never answer a newer ask. None for any
/// other ask.
pub fn answer_of(said: &Value, wants: &[Value], now_ms: u128) -> Option<Result<Value, String>> {
    if said["ask"] != "answer" {
        return None;
    }
    let address = said["address"].as_str().unwrap_or("").to_uppercase();
    let Some(want) = wants
        .iter()
        .find(|one| one["address"].as_str().unwrap_or("").to_uppercase() == address)
        .and_then(|one| one["answers"]["want"].as_str())
    else {
        return Some(Err(format!(
            "{address} asks Joel nothing he can answer here"
        )));
    };
    if let Some(named) = said["want"].as_str().filter(|named| !named.is_empty())
        && named != want
    {
        return Some(Err(format!("{address} asks something else now")));
    }
    let mut steer = json!({
        "act": "answer",
        "want": want,
        "from": "joel",
        "at": clock::text_of(i64::try_from(now_ms).unwrap_or(0)),
    });
    if let Some(value) = said["value"].as_str().filter(|value| !value.is_empty()) {
        steer["value"] = json!(value);
    } else if let Some(other) = said["other"]
        .as_str()
        .map(str::trim)
        .filter(|o| !o.is_empty())
    {
        steer["other"] = json!(other);
    } else {
        return Some(Err("an answer needs a value or Joel's own words".to_owned()));
    }
    Some(Ok(
        json!({"ask": "steer", "address": address, "steer": steer}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asking(address: &str, standing: &str, wants: &Value) -> Value {
        json!({"standing": standing, "presence": {"addresses": [address],
            "questInfo": {"title": format!("{address} Quest")}, "wants": wants}})
    }

    fn served(sessions: &[Value]) -> Vec<Value> {
        served_at(sessions, &PutOff::default(), 0)
    }

    fn served_at(sessions: &[Value], put_off: &PutOff, now_ms: u128) -> Vec<Value> {
        let sessions: Vec<Session> = sessions.iter().map(Session).collect();
        records(&sessions, &JoelCan::default(), (put_off, now_ms))
    }

    #[test]
    fn a_want_put_off_rings_for_nothing_for_an_hour_then_needs_joel_again() {
        let approval = json!({"kind": "approval", "said": "Run it",
            "since": "2026-11-16T08:32:01.765Z"});
        let sessions = [
            asking("KQL3KB", "running", &approval),
            asking("SKIFF", "running", &json!({"kind": "question"})),
        ];
        let mut put_off = PutOff::default();
        put_off.put("KQL3KB approval 2026-11-16T08:32:01.765Z", 1_000);
        let said = |now_ms| -> Vec<(Value, Value)> {
            served_at(&sessions, &put_off, now_ms)
                .iter()
                .map(|one| (one["putOff"].clone(), one["rings"].clone()))
                .collect()
        };
        // Served oldest asked first, SKIFF's, saying no time, before it.
        assert_eq!(
            said(1_000),
            [(json!(false), json!(true)), (json!(true), json!(false))],
            "the one put off says so and rings for nothing, and the other is as it was"
        );
        assert_eq!(
            said(1_000 + PUT_OFF_FOR_MS - 1),
            said(1_000),
            "for all of the hour"
        );
        assert_eq!(
            said(1_000 + PUT_OFF_FOR_MS),
            [(json!(false), json!(true)), (json!(false), json!(true))],
            "and once it's over the want needs Joel again"
        );
        let held = |at: usize, now_ms| put_off.holds_for(Session(&sessions[at]), now_ms);
        assert_eq!(
            [
                held(0, 1_000),
                held(1, 1_000),
                held(0, 1_000 + PUT_OFF_FOR_MS)
            ],
            [true, false, false],
            "a session's own want is put off for the hour, as its row reads it"
        );
    }

    #[test]
    fn what_is_put_off_is_kept_waited_for_and_let_go_once_over() {
        let mut put_off = PutOff::default();
        put_off.put("SKIFF question-1", 0);
        put_off.put("BUOY approval-1", 600_000);
        assert_eq!(
            put_off.wait(PUT_OFF_FOR_MS - 10_000),
            Some(Duration::from_secs(10)),
            "the soonest still put off is waited for"
        );
        assert_eq!(
            put_off.wait(600_000),
            Some(Duration::from_mins(1)),
            "a minute at most, lest a sleep carry the wait past its hour"
        );
        assert_eq!(
            put_off.wait(PUT_OFF_FOR_MS + 600_000),
            None,
            "and none once all are over"
        );
        put_off.put("KELP approval-2", PUT_OFF_FOR_MS);
        assert!(
            !put_off.0.contains_key("SKIFF question-1"),
            "one whose hour is over is let go as another is put off"
        );
        let path = std::env::temp_dir()
            .join(format!("agents-put-off-{}", std::process::id()))
            .join(PUT_OFF_FILE);
        put_off.write(&path).expect("kept");
        assert_eq!(
            PutOff::read(&path, PUT_OFF_FOR_MS),
            put_off,
            "read back as it was written"
        );
        let stepped_back = PutOff::read(&path, 0);
        assert!(
            stepped_back.holds("KELP approval-2", PUT_OFF_FOR_MS - 1)
                && !stepped_back.holds("KELP approval-2", PUT_OFF_FOR_MS),
            "read with the clock stepped back, held only an hour from then"
        );
        let _ = fs::remove_dir_all(path.parent().expect("its folder"));
        assert_eq!(
            PutOff::read(&path, 0),
            PutOff::default(),
            "and none when there's no file"
        );
    }

    #[test]
    fn not_now_names_the_want_its_session_asks() {
        let held = Wants::default();
        held.hold(
            served(&[asking(
                "SKIFF",
                "running",
                &json!({"kind": "approval", "id": "approval-1"}),
            )])
            .iter(),
        );
        assert_eq!(
            held.put_off(&json!({"ask": "not-now", "address": "skiff",
                "known": "SKIFF approval-1"})),
            Some(Ok("SKIFF approval-1".to_owned())),
            "the ask Joel saw, by its address whatever its case"
        );
        assert!(
            matches!(
                held.put_off(&json!({"ask": "not-now", "address": "SKIFF"})),
                Some(Err(_))
            ),
            "one naming no ask he saw is refused"
        );
        assert!(
            matches!(
                held.put_off(&json!({"ask": "not-now", "address": "BUOY",
                    "known": "BUOY approval-1"})),
                Some(Err(_))
            ),
            "a session wanting nothing is refused"
        );
        assert!(
            matches!(
                held.put_off(&json!({"ask": "not-now", "address": "SKIFF",
                    "known": "SKIFF approval-0"})),
                Some(Err(_))
            ),
            "a newer ask than the one he saw still needs him"
        );
        assert!(
            matches!(held.put_off(&json!({"ask": "not-now"})), Some(Err(_))),
            "and one naming no session is refused"
        );
        assert_eq!(
            held.put_off(&json!({"ask": "answer"})),
            None,
            "another ask is none of this"
        );
    }

    #[test]
    fn each_session_asking_joel_is_a_want_the_oldest_first() {
        let question = json!({"kind": "question", "said": "which tide?",
            "since": "2026-11-13T05:00:00Z", "ask": {"recommended": "high",
                "allowOther": true, "choices": [
                    {"value": "low", "label": "Low Tide"},
                    {"value": "high", "label": "High Tide", "description": "at noon"},
                    {"value": ""}]}});
        let approval = json!({"kind": "approval", "since": "2026-11-13T04:00:00Z"});
        let wants = served(&[
            asking("SKIFF", "running", &question),
            asking("BUOY", "parked", &approval),
            asking(
                "KELP",
                "running",
                &json!({"kind": "action", "since": "2026-11-13T03:00:00Z"}),
            ),
            asking("REEF", "closed", &question),
            asking("TIDE", "running", &Value::Null),
        ]);
        let keys: Vec<&str> = wants.iter().filter_map(|one| one["key"].as_str()).collect();
        assert_eq!(
            keys,
            ["want:agents:BUOY", "want:agents:SKIFF"],
            "only those asking Joel, never an action nor one closed, the oldest first"
        );
        assert_eq!(
            wants[0],
            json!({"key": "want:agents:BUOY", "kind": "want", "address": "BUOY",
                "title": "BUOY Quest", "asked": "BUOY Quest", "wants": "approval", "said": "",
                "whole": "", "since": "2026-11-13T04:00:00Z",
                "parked": true, "answers": null, "rings": true, "putOff": false,
                "known": "BUOY approval 2026-11-13T04:00:00Z"}),
            "one asking nothing in words is named by its session, and offers no answers"
        );
        assert_eq!(
            wants[1]["answers"],
            json!({"want": "question@2026-11-13T05:00:00.000Z", "own": true, "choices": [
                {"value": "high", "label": "High Tide", "description": "at noon",
                    "recommended": true},
                {"value": "low", "label": "Low Tide", "description": "", "recommended": false}]}),
            "its answers the recommended first, each labelled, its id as the session knows it"
        );
        assert_eq!(
            (&wants[1]["asked"], &wants[1]["wants"], &wants[1]["said"]),
            (
                &json!("which tide?"),
                &json!("question"),
                &json!("which tide?")
            ),
            "what it asked, in its words, and what kind of want it is"
        );
    }

    /// What a want asks in full is passed on whole, its lines kept, so Talk
    /// and For You say the question as it was put; one longer than the
    /// harness writes, or blank, is none, and the short line stands.
    #[test]
    fn a_want_says_its_whole_question_lines_kept() {
        let asked = "Which tide do we sail on?\n\n- Low: calmer\n  - at dawn\n- High: faster";
        let want = |put: &Value| {
            json!({"kind": "question", "said": "Which tide do we sail on?",
                "asked": put, "since": "2026-11-13T05:00:00Z"})
        };
        let wants = served(&[
            asking("SKIFF", "running", &want(&json!(asked))),
            asking(
                "BUOY",
                "running",
                &want(&json!("x".repeat(WHOLE_AT_MOST + 1))),
            ),
            asking("KELP", "running", &want(&json!(" \n "))),
            asking("REEF", "running", &want(&json!(7))),
        ]);
        let whole: Vec<(&str, &str)> = wants
            .iter()
            .map(|one| {
                (
                    one["address"].as_str().unwrap_or(""),
                    one["whole"].as_str().unwrap_or("?"),
                )
            })
            .collect();
        assert_eq!(
            whole,
            [("SKIFF", asked), ("BUOY", ""), ("KELP", ""), ("REEF", "")],
            "the whole as written, else none"
        );
        assert_eq!(
            wants[0]["asked"], "Which tide do we sail on?",
            "its short line is still what it asked"
        );
    }

    #[test]
    fn a_question_and_a_yes_ring_and_a_run_stuck_does_not() {
        let kinds = ["question", "approval", "blocked", "action"];
        let rang: Vec<bool> = kinds
            .iter()
            .map(|kind| rings(&json!({ "kind": kind })))
            .collect();
        assert_eq!(
            rang,
            [true, true, false, false],
            "only a question and a yes"
        );
        assert!(!rings(&Value::Null), "nor does no want");
        let wants = served(&[
            asking("SKIFF", "running", &json!({"kind": "question"})),
            asking("KELP", "running", &json!({"kind": "blocked"})),
        ]);
        let served: Vec<(&Value, &Value)> = wants
            .iter()
            .map(|one| (&one["address"], &one["rings"]))
            .collect();
        assert_eq!(
            served,
            [
                (&json!("SKIFF"), &json!(true)),
                (&json!("KELP"), &json!(false))
            ],
            "each want served says whether it rings, a run stuck listed but silent"
        );
    }

    #[test]
    fn a_want_is_known_by_who_it_is_never_by_its_words() {
        let since = "2026-11-06T21:00:00.000Z";
        let approval = |fields: Value| {
            let mut wants = json!({"kind": "approval", "since": since});
            if let (Some(wants), Some(fields)) = (wants.as_object_mut(), fields.as_object()) {
                wants.extend(fields.clone());
            }
            known("8IDU4E", &wants)
        };
        assert_eq!(
            approval(json!({"said": "Holds 2 calls"})),
            approval(json!({"said": "Holds 3 calls"})),
            "a want reworded is the same want"
        );
        assert_ne!(
            approval(json!({})),
            approval(json!({"since": "2026-11-06T21:05:00.000Z"})),
            "one raised again later is another"
        );
        assert_ne!(
            approval(json!({})),
            approval(json!({"kind": "question"})),
            "as is another kind begun at once"
        );
        assert_eq!(
            approval(json!({"id": "approval-1"})),
            "8IDU4E approval-1",
            "the harness's own id comes first"
        );
        assert_eq!(
            known("8IDU4E", &json!({"kind": "question", "said": "Push it?"})),
            "8IDU4E question Push it?",
            "its words only when it gives no start"
        );
    }

    #[test]
    fn an_answer_is_steered_to_the_want_its_session_asks() {
        let wants = served(&[asking(
            "SKIFF",
            "running",
            &json!({"kind": "question", "since": "2026-11-13T05:00:00Z",
                "ask": {"choices": [{"value": "high"}]}}),
        )]);
        assert_eq!(
            answer_of(
                &json!({"ask": "answer", "address": "skiff", "value": "high"}),
                &wants,
                0
            ),
            Some(Ok(
                json!({"ask": "steer", "address": "SKIFF", "steer": {"act": "answer",
                "want": "question@2026-11-13T05:00:00.000Z", "from": "joel",
                "at": "1970-01-01T00:00:00.000Z", "value": "high"}})
            )),
            "by its address, whatever its case, the want's id found here"
        );
        assert_eq!(
            answer_of(
                &json!({"ask": "answer", "address": "SKIFF", "other": " both "}),
                &wants,
                0
            )
            .and_then(Result::ok)
            .map(|ask| ask["steer"]["other"].clone()),
            Some(json!("both")),
            "Joel's own words, trimmed"
        );
        assert!(
            matches!(
                answer_of(
                    &json!({"ask": "answer", "address": "BUOY", "value": "x"}),
                    &wants,
                    0
                ),
                Some(Err(_))
            ),
            "a session asking nothing is refused"
        );
        assert!(
            matches!(
                answer_of(&json!({"ask": "answer", "address": "SKIFF"}), &wants, 0),
                Some(Err(_))
            ),
            "an answer with no words is refused"
        );
        assert_eq!(
            answer_of(
                &json!({"ask": "answer", "address": "SKIFF", "value": "high",
                    "want": "question@2026-11-13T05:00:00.000Z"}),
                &wants,
                0
            )
            .and_then(Result::ok)
            .map(|ask| ask["steer"]["want"].clone()),
            Some(json!("question@2026-11-13T05:00:00.000Z")),
            "one naming the want it asks is steered to it"
        );
        assert_eq!(
            answer_of(
                &json!({"ask": "answer", "address": "SKIFF", "value": "high",
                    "want": "question@2026-11-13T04:00:00.000Z"}),
                &wants,
                0
            ),
            Some(Err("SKIFF asks something else now".to_owned())),
            "one naming a want it no longer asks is refused"
        );
        assert_eq!(
            answer_of(&json!({"ask": "talk"}), &wants, 0),
            None,
            "another ask is none of this"
        );
    }
}
