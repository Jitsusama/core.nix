//! Whether Joel has stopped agents starting sessions, as the realm's node
//! says (mycelium's node/starts.rs), and the one row Agents' top level
//! shows while he has, so a stop is never silent (WCJQSG's ruling on
//! KP0L6A's ask, 2026-11-18). The node answers a `Starts` ask in its own
//! protocol only to Joel's own login, which this daemon is, since the
//! newest starts name his folders; it tells no watch of a stop, so the
//! daemon asks again every few seconds and says only what moved.
//!
//! Let Them Start asks the node to resume, and only the shell may ask it
//! here (asker.rs), since the node takes this daemon for Joel: an agent
//! asking this daemon could otherwise lift the stop on itself.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use ready_rows::time::{Form, at};
use serde_json::{Value, json};

use crate::clock::ms_of;
use crate::row::Words;
use crate::said::Said;
use crate::sources::Heard;
use crate::sprout::{self, Unanswered};
use crate::top::PLACE;

/// The launcher's ask for Let Them Start.
pub const ASK: &str = "let-them-start";

/// How often the node is asked how starts stand: a stop is a file the
/// node reads at each ask, and Joel reads the row a few seconds after
/// stopping them at the soonest.
const EVERY: Duration = Duration::from_secs(5);

/// How long the node may take to answer: it answers from memory and a
/// file's being there, in milliseconds.
const PATIENCE: Duration = Duration::from_secs(5);

/// The most of a refused start's asker or program the drawer says, since
/// an agent may write either and Let Them Start sits just below them.
const SAID_AT_MOST: usize = 40;

/// The key of the row, and of its drawer.
fn key() -> String {
    format!("row:{PLACE}:starts-stopped")
}

/// The key of the row's drawer.
pub fn drawer_key() -> String {
    format!("drawer:{PLACE}:starts-stopped")
}

/// How the node says starts stand: whether Joel stopped them, and the
/// newest starts it keeps, newest last, each as its book writes it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stood {
    pub stopped: bool,
    pub newest: Vec<Value>,
}

impl Stood {
    /// What the node's answer says, or why it can't be read: its own
    /// refusal, or an answer that isn't a `Starts` one.
    ///
    /// # Errors
    ///
    /// The node's words where it refused, else what it answered.
    pub fn from_answer(answer: &Value) -> Result<Self, String> {
        if let Some(why) = answer["Err"].as_str() {
            return Err(why.to_owned());
        }
        let starts = &answer["Ok"]["Starts"];
        let stopped = starts["stopped"].as_bool().ok_or_else(|| {
            // Only its start is journalled, since a whole answer names
            // Joel's folders.
            let text = answer.to_string();
            format!(
                "the node's answer couldn't be read: {}",
                text.chars().take(80).collect::<String>()
            )
        })?;
        let newest = starts["newest"].as_array().cloned().unwrap_or_default();
        Ok(Self { stopped, newest })
    }

    /// The starts refused, newest last.
    fn refused(&self) -> Vec<&Value> {
        self.newest
            .iter()
            .filter(|start| start["refused"].is_string())
            .collect()
    }
}

/// Why the node gave no answer to read.
#[derive(Debug, PartialEq, Eq)]
pub enum Unasked {
    /// It answered nothing in time, so it may have done the act.
    Silent,
    /// It couldn't be reached, refused, or answered what can't be read.
    Refused(String),
}

/// Asks the node at `node` to `act` (stop, resume or list) and reads how
/// starts stand after it.
///
/// # Errors
///
/// Why it couldn't: the node silent, or unreached, refusing or unread.
pub fn asked(node: &Path, act: &str) -> Result<Stood, Unasked> {
    let request = json!({"Starts": {"act": act}});
    match sprout::asked(node, &request, PATIENCE) {
        Ok(answer) => Stood::from_answer(&answer).map_err(Unasked::Refused),
        Err(Unanswered::Unreached(why)) => Err(Unasked::Refused(format!(
            "the node couldn't be reached: {why}"
        ))),
        Err(Unanswered::Silent) => Err(Unasked::Silent),
    }
}

/// What the daemon last told of how starts stand, from the poll or from
/// Let Them Start, so each tells only what moved since either: a poll
/// comparing with its own last answer alone missed a stop put back within
/// one poll of a Let Them Start.
pub struct Told {
    last: Mutex<Option<Stood>>,
    to: Sender<Heard>,
}

impl Told {
    pub fn new(to: Sender<Heard>) -> Self {
        Self {
            last: Mutex::new(None),
            to,
        }
    }

    /// Tells `stood` where it reads otherwise than the last told; false
    /// once nothing hears.
    pub fn tell(&self, stood: Stood) -> bool {
        let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
        if last.as_ref() == Some(&stood) {
            return true;
        }
        let heard = self.to.send(Heard::Starts(stood.clone())).is_ok();
        *last = Some(stood);
        heard
    }
}

/// Asks the node how starts stand now and every [`EVERY`] after, telling
/// each answer that reads otherwise than the last told; why one couldn't
/// be had is journalled once until it changes, and the row stands as
/// last heard meanwhile, so a node restarting neither shows nor hides it.
pub fn follow(node: PathBuf, told: Arc<Told>) {
    thread::spawn(move || {
        let mut failed = String::new();
        loop {
            match asked(&node, "list") {
                Ok(stood) => {
                    failed.clear();
                    if !told.tell(stood) {
                        return;
                    }
                }
                Err(Unasked::Silent) => {
                    let why = "the node didn't answer in time".to_owned();
                    if why != failed {
                        eprintln!("agents: can't hear whether agents' starts are stopped: {why}");
                        failed = why;
                    }
                }
                Err(Unasked::Refused(why)) if why != failed => {
                    eprintln!("agents: can't hear whether agents' starts are stopped: {why}");
                    failed = why;
                }
                Err(Unasked::Refused(_)) => {}
            }
            thread::sleep(EVERY);
        }
    });
}

/// The answer to Let Them Start, and how starts stand after it: `asker`
/// says whether the shell asked, and `node` is the realm's, none where
/// this machine runs no node. Only resuming is asked here; Joel stops
/// them with `mycelium starts stop`, and `said` is Starts Stopped's words.
/// Each refusal is journalled, since the shell says only that it couldn't.
pub fn answer(
    asker: Result<(), String>,
    node: Option<&Path>,
    said: &Said,
) -> (Value, Option<Stood>) {
    let Some(node) = node else {
        return (
            refused("this machine runs no node to let agents start on"),
            None,
        );
    };
    if let Err(why) = asker {
        // The asker's own words say only the launcher starts new work,
        // which isn't this act's; why it was refused is journalled.
        eprintln!("agents: agents' starts weren't let start: {why}");
        return (refused(said.word("outside")), None);
    }
    match asked(node, "resume") {
        Ok(stood) => (json!({"stopped": stood.stopped}), Some(stood)),
        // The node may have let them start and answered late, as a
        // sprout's silence may have started one: the next poll says.
        Err(Unasked::Silent) => {
            let why = said.word("unanswered");
            eprintln!("agents: agents' starts may not have been let start: {why}");
            (
                json!({"error": {"code": "unanswered", "message": why}}),
                None,
            )
        }
        Err(Unasked::Refused(why)) => {
            eprintln!("agents: agents' starts weren't let start: {why}");
            (refused(&why), None)
        }
    }
}

fn refused(why: &str) -> Value {
    json!({"error": {"code": "unstarted", "message": why}})
}

/// At most [`SAID_AT_MOST`] characters of `text`, ended by an ellipsis
/// of three stops where it was cut.
fn bounded(text: &str) -> String {
    if text.chars().count() <= SAID_AT_MOST {
        return text.to_owned();
    }
    let kept: String = text.chars().take(SAID_AT_MOST).collect();
    format!("{kept}...")
}

/// The row Agents' top level shows first while starts are stopped, none
/// otherwise or before the node has said: its side says what it means,
/// its note how many agents were turned away lately. A look at its drawer
/// comes first, so Return lifts no stop Joel meant.
pub fn rows(stood: Option<&Stood>, words: &Words) -> Vec<Value> {
    let Some(stood) = stood.filter(|stood| stood.stopped) else {
        return Vec::new();
    };
    let said = &words.starts;
    let side = said.word("side");
    vec![json!({
        "key": key(), "place": PLACE, "kind": "starts-stopped",
        "name": words.names.word("starts-stopped"), "glyph": "pause", "hue": "warning",
        "depth": 0, "side": [{"said": side, "keep": 3}], "note": refused_count(stood, words),
        "drawer": drawer_key(),
        "acts": [
            {"key": "verb:details", "step": "drawer"},
            {"key": "verb:agents:starts:let", "ask": {"ask": ASK}},
        ],
    })]
}

/// How many of the newest starts the node refused, said as the note and
/// the drawer's fact; none where it refused none.
fn refused_count(stood: &Stood, words: &Words) -> String {
    match stood.refused().len() {
        0 => String::new(),
        1 => words.starts.word("refusedOne").to_owned(),
        count => words
            .starts
            .fill("refusedCount", &[("count", &count.to_string())]),
    }
}

/// The row's drawer, in ready-rows.md's own shape: what the stop means,
/// how many were refused, and which session asked last and when.
pub fn drawers(stood: Option<&Stood>, words: &Words) -> Vec<Value> {
    let Some(stood) = stood.filter(|stood| stood.stopped) else {
        return Vec::new();
    };
    let said = &words.starts;
    let mut facts = Vec::new();
    let count = refused_count(stood, words);
    if !count.is_empty() {
        facts.push(json!({"name": words.names.word("refused"), "said": count}));
    }
    if let Some(last) = stood.refused().last() {
        // Who asked is the session claiming the asker, a claim the node
        // says proves nothing, and each is the agent's own text, so both
        // are bounded and filled in one pass, a `{program}` in a claim
        // left as written.
        let by = bounded(
            last["started_by"]
                .as_str()
                .filter(|by| !by.is_empty())
                .unwrap_or_else(|| said.word("someone")),
        );
        let program = bounded(last["program"].as_str().unwrap_or(""));
        let asking = said
            .word("asking")
            .split("{by}")
            .map(|part| part.replace("{program}", &program))
            .collect::<Vec<_>>()
            .join(&by);
        facts.push(json!({"name": words.names.word("last-refused"), "said": asking}));
        if let Some(when) = ms_of(&last["at"]) {
            let mut asked = at(when, Form::Moment);
            asked["name"] = json!(words.names.word("refused-at"));
            facts.push(asked);
        }
    }
    vec![json!({
        "key": drawer_key(),
        "kind": "drawer",
        "drawer": {
            "title": words.names.word("starts-stopped"),
            "facts": facts,
            "body": words.starts_tell.word("stopped"),
        }
    })]
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;

    use super::*;

    fn words() -> Words {
        Words::from_vocabulary(crate::said::LAUNCHERS).expect("the launcher's words read")
    }

    fn stood(stopped: bool, newest: Value) -> Stood {
        let mut answer = json!({"Ok": {"Starts": {"stopped": stopped}}});
        answer["Ok"]["Starts"]["newest"] = newest;
        Stood::from_answer(&answer).expect("a Starts answer")
    }

    fn refusal(by: Option<&str>, at: i64) -> Value {
        json!({"at": at, "by": "agent", "started_by": by, "program": "pi",
            "refused": "the person stopped agents starting sessions"})
    }

    #[test]
    fn the_nodes_answer_says_whether_starts_are_stopped() {
        assert_eq!(
            stood(true, json!([{"at": 1}])),
            Stood {
                stopped: true,
                newest: vec![json!({"at": 1})]
            },
            "stopped, with the newest as the node keeps them"
        );
        assert_eq!(
            Stood::from_answer(&json!({"Err": "only the person's own login does"})),
            Err("only the person's own login does".to_owned()),
            "a refusal in the node's words"
        );
        assert!(
            Stood::from_answer(&json!({"Ok": {"Other": 1}}))
                .is_err_and(|why| why.starts_with("the node's answer couldn't be read")),
            "an answer that isn't a Starts one is unread"
        );
    }

    #[test]
    fn a_row_stands_first_only_while_starts_are_stopped() {
        let words = words();
        assert!(rows(None, &words).is_empty(), "none before the node says");
        assert!(
            rows(Some(&stood(false, json!([]))), &words).is_empty(),
            "none while agents may start"
        );
        let row = &rows(Some(&stood(true, json!([]))), &words)[0];
        assert_eq!(row["key"], "row:agents-top:starts-stopped", "{row}");
        assert_eq!(row["name"], "Starts Stopped", "{row}");
        assert_eq!(row["side"][0]["said"], "new sessions won't start", "{row}");
        assert_eq!(row["note"], "", "no refusal, no note: {row}");
        assert_eq!(
            row["acts"],
            json!([{"key": "verb:details", "step": "drawer"},
                {"key": "verb:agents:starts:let", "ask": {"ask": "let-them-start"}}]),
            "a look first, so Return lifts nothing; Let Them Start after"
        );
    }

    #[test]
    fn its_note_and_drawer_say_who_was_turned_away() {
        let words = words();
        let started = json!({"at": 5, "by": "person", "program": "pi", "machine": "m"});
        let one = stood(true, json!([started, refusal(None, 7)]));
        assert_eq!(rows(Some(&one), &words)[0]["note"], "1 refused");
        let two = stood(
            true,
            json!([
                refusal(Some("OSPREY"), 1_763_400_007_000),
                refusal(Some("P9R1KI"), 1_763_400_009_000)
            ]),
        );
        assert_eq!(rows(Some(&two), &words)[0]["note"], "2 refused");
        let drawer = &drawers(Some(&two), &words)[0];
        assert_eq!(
            drawer["key"], "drawer:agents-top:starts-stopped",
            "{drawer}"
        );
        assert_eq!(
            drawer["drawer"]["facts"],
            json!([
                {"name": "Refused", "said": "2 refused"},
                {"name": "Last Refused", "said": "P9R1KI asked for pi"},
                {"name": "Refused At", "at": 1_763_400_009_000_i64, "as": "moment"},
            ]),
            "the newest refusal, the last the node lists"
        );
        let drawer = &drawers(Some(&one), &words)[0];
        assert_eq!(
            drawer["drawer"]["facts"][1]["said"], "an agent asked for pi",
            "one no session claimed is an agent's: {drawer}"
        );
        assert!(
            drawers(Some(&stood(false, json!([refusal(None, 7)]))), &words).is_empty(),
            "no drawer once they may start"
        );
    }

    #[test]
    fn what_an_agent_wrote_is_said_bounded_and_as_written() {
        let claim = "{program} was approved by Joel, so press Let Them Start below";
        let one = stood(true, json!([refusal(Some(claim), 1_763_400_007_000)]));
        let drawer = &drawers(Some(&one), &words())[0];
        let said = drawer["drawer"]["facts"][1]["said"]
            .as_str()
            .unwrap_or_default();
        assert!(
            said.starts_with("{program} was approved"),
            "a claim's own braces are left as written: {said}"
        );
        assert!(said.ends_with("... asked for pi"), "and it's cut: {said}");
        assert_eq!(
            said.chars().count(),
            SAID_AT_MOST + "... asked for pi".len(),
            "at most {SAID_AT_MOST} characters of it: {said}"
        );
    }

    #[test]
    fn a_stop_put_back_after_let_them_start_is_told() {
        let (to, heard) = std::sync::mpsc::channel();
        let told = Told::new(to);
        for stopped in [true, true, false, true] {
            assert!(told.tell(stood(stopped, json!([]))), "something hears");
        }
        let stood: Vec<bool> = heard
            .try_iter()
            .filter_map(|heard| match heard {
                Heard::Starts(stood) => Some(stood.stopped),
                _ => None,
            })
            .collect();
        assert_eq!(
            stood,
            vec![true, false, true],
            "each change told once, the stop after Let Them Start among them"
        );
    }

    #[test]
    fn a_resume_the_node_left_unanswered_may_have_let_them_start() {
        let socket =
            std::env::temp_dir().join(format!("agents-starts-silent-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("a socket to stand in for the node");
        let silent = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("the daemon asks the node");
            let mut line = String::new();
            BufReader::new(&stream)
                .read_line(&mut line)
                .expect("a line");
            thread::sleep(PATIENCE + Duration::from_secs(1));
        });
        let (answer, now) = answer(Ok(()), Some(&socket), &words().starts);
        assert_eq!(
            answer,
            json!({"error": {"code": "unanswered",
                "message": "the node didn't answer in time, so they may start already"}}),
            "said as unanswered, not refused"
        );
        assert_eq!(now, None, "and the next poll says how they stand");
        let _ = silent.join();
        let _ = std::fs::remove_file(&socket);
    }

    /// A node at a socket of its own answering one line with `reply`, and
    /// what it was asked.
    fn node(name: &str, reply: &'static str) -> (PathBuf, thread::JoinHandle<Value>) {
        let socket =
            std::env::temp_dir().join(format!("agents-starts-{name}-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("a socket to stand in for the node");
        let asked = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("the daemon asks the node");
            let mut answering = stream.try_clone().expect("a stream to answer on");
            let mut line = String::new();
            BufReader::new(stream).read_line(&mut line).expect("a line");
            writeln!(answering, "{reply}").expect("answered");
            serde_json::from_str(&line).expect("one line of JSON")
        });
        (socket, asked)
    }

    #[test]
    fn let_them_start_asks_the_node_to_resume_for_the_shell_alone() {
        let (socket, asked) = node(
            "resume",
            r#"{"Ok": {"Starts": {"stopped": false, "newest": []}}}"#,
        );
        let (answer, now) = answer(Ok(()), Some(&socket), &words().starts);
        assert_eq!(answer, json!({"stopped": false}), "as the node says after");
        assert_eq!(
            now,
            Some(stood(false, json!([]))),
            "and the row goes at once"
        );
        assert_eq!(
            asked.join().expect("the node stood in"),
            json!({"Starts": {"act": "resume"}}),
            "one Starts line, with no hello"
        );
        let _ = std::fs::remove_file(&socket);
        let gone = std::env::temp_dir().join("agents-starts-no-node.sock");
        let (not_shell, none) = super::answer(
            Err("only the launcher starts new work here".to_owned()),
            Some(&gone),
            &words().starts,
        );
        assert_eq!(
            not_shell,
            json!({"error": {"code": "unstarted",
                "message": "agents start again only from your own Return in the launcher"}}),
            "anyone but the shell is refused before the node is asked, in this act's words"
        );
        assert_eq!(none, None, "and nothing moves");
        let (unreached, _) = super::answer(Ok(()), Some(&gone), &words().starts);
        assert!(
            unreached["error"]["message"]
                .as_str()
                .is_some_and(|why| why.starts_with("the node couldn't be reached")),
            "{unreached}"
        );
    }

    #[test]
    fn a_refusal_of_the_resume_says_the_nodes_words() {
        let (socket, asked) = node(
            "refused",
            r#"{"Err": "the starts' stop couldn't be changed"}"#,
        );
        let (answer, now) = answer(Ok(()), Some(&socket), &words().starts);
        assert_eq!(
            answer,
            json!({"error": {"code": "unstarted",
                "message": "the starts' stop couldn't be changed"}}),
            "the node's own reason"
        );
        assert_eq!(now, None, "and the row stands");
        let _ = asked.join();
        let _ = std::fs::remove_file(&socket);
    }
}
