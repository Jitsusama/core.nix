//! Everything the daemon holds, behind the one lock the bus and the
//! shell's asks share: the notices, what arrivals are judged by, and the
//! signals owed to senders, which the daemon's loop sends in the order
//! they were owed. An arrival and every ask are here rather than in the
//! bus or the socket, so they're tested without either.
//!
//! The shell asks, over its socket (src/main.rs), what Joel does to a
//! notice; each answers `{ done: true }`, or an error the shell can say:
//!
//! - `invoke { notice, action, token? }`: the action is done, as its
//!   sender hears it, the activation token first where the shell got one,
//!   and the notice closes unless its sender asked it stay (`resident`).
//! - `reply { notice, text }`: a reply sent in place, which closes it
//!   likewise.
//! - `dismiss { notices }` and `expire { notices }`: closed, put away or
//!   its time up.
//! - `rest`, `wake` and `raise { notice }`: off the screen, back, and back
//!   unread.
//! - `mark { notices, read, how? }`, `seen`, and `settle { standing }`.
//! - `quiet { sharing }`: whether the shell holds notices while Joel
//!   shares his screen, which only the shell sees.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde_json::{Map, Value, json};

use crate::held::{Closed, Holding};
use crate::judged::{Judge, kept_quiet};
use crate::notice::{Action, Notice, Urgency};
use crate::picture::{self, Pixels};

/// A signal owed to the senders, as the specification names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Signal {
    /// `NotificationClosed`: its id and why, by the specification's code.
    Closed(u32, u32),
    /// `ActivationToken`: its id and the token to raise a window with.
    Token(u32, String),
    /// `ActionInvoked`: its id and the action's key.
    Invoked(u32, String),
    /// `NotificationReplied`: its id and the reply.
    Replied(u32, String),
}

/// A notice as its sender sent it, `Notify`'s arguments.
#[derive(Debug, Default)]
pub struct Sent {
    pub app: String,
    pub replaces: u32,
    pub icon: String,
    pub summary: String,
    pub body: String,
    /// Each action's key and words, one after the other.
    pub actions: Vec<String>,
    /// Its hints that are words, numbers or a yes or no.
    pub hints: Map<String, Value>,
    /// Its picture, where it was sent as pixels.
    pub pixels: Option<Pixels>,
    pub expire: i32,
}

/// What the daemon holds.
#[derive(Debug)]
pub struct State {
    pub holding: Holding,
    pub judge: Judge,
    /// Quiet.qml's file, read as each notice arrives.
    pub quiet: PathBuf,
    /// Whether the shell holds notices while Joel shares his screen.
    pub sharing: bool,
    /// Where pictures sent as pixels are written.
    pub pictures: PathBuf,
    /// Signals owed to senders, oldest first.
    pub owed: Vec<Signal>,
}

impl State {
    pub fn new(holding: Holding, judge: Judge, quiet: PathBuf, pictures: PathBuf) -> State {
        State {
            holding,
            judge,
            quiet,
            sharing: false,
            pictures,
            owed: Vec::new(),
        }
    }

    /// Takes a notice sent over the bus, answering the id it's known by:
    /// the one it replaces, while that's open, else one never given.
    pub fn notify(&mut self, sent: Sent, now: i64) -> u32 {
        let replacing = sent.replaces != 0 && self.holding.is_live(sent.replaces);
        let id = if replacing {
            sent.replaces
        } else {
            self.holding.new_id()
        };
        let (actions, reply) = split(&sent.actions);
        let hint = |name: &str| sent.hints.get(name).and_then(Value::as_str).unwrap_or("");
        let yes = |name: &str| {
            sent.hints
                .get(name)
                .and_then(Value::as_bool)
                .unwrap_or(false)
        };
        let urgency = sent
            .hints
            .get("urgency")
            .and_then(Value::as_u64)
            .and_then(|byte| u8::try_from(byte).ok())
            .map_or(Urgency::Normal, Urgency::from_byte);
        let path = [hint("image-path"), hint("image_path")]
            .into_iter()
            .find(|path| !path.is_empty())
            .unwrap_or("");
        let picture = match &sent.pixels {
            Some(pixels) => picture::write(pixels, &picture::file_for(&self.pictures, id, now))
                .unwrap_or_default(),
            None => picture::named(path, &sent.icon),
        };
        let mut notice = Notice {
            id,
            at: now,
            entry: hint("desktop-entry").to_owned(),
            category: hint("category").to_owned(),
            transient: yes("transient"),
            resident: yes("resident"),
            app: sent.app,
            icon: sent.icon,
            summary: sent.summary,
            body: sent.body,
            actions,
            reply,
            urgency,
            hints: sent.hints,
            expire: sent.expire,
            picture,
            ..Notice::default()
        };
        if replacing {
            let was = self.holding.get(id).map(|was| was.picture.clone());
            if let Some(was) = was.filter(|was| *was != notice.picture) {
                picture::forget(&was, &self.pictures);
            }
        } else {
            let quiet = self.sharing || kept_quiet(&self.quiet, now);
            self.judge.judge(&mut notice, quiet);
        }
        let passing = notice.picture.clone();
        if let Some(why) = self.holding.arrive(notice) {
            picture::forget(&passing, &self.pictures);
            self.owed.push(Signal::Closed(id, why as u32));
        }
        for gone in self.holding.trim() {
            picture::forget(&gone, &self.pictures);
        }
        id
    }

    /// Its sender withdrew one, `CloseNotification`.
    pub fn withdraw(&mut self, id: u32, now: i64) {
        self.close(id, Closed::Withdrawn, now);
    }

    fn close(&mut self, id: u32, why: Closed, now: i64) -> bool {
        let picture = self
            .holding
            .get(id)
            .map(|notice| (notice.transient, notice.picture.clone()));
        let closed = self.holding.close(id, why, now);
        if closed {
            self.owed.push(Signal::Closed(id, why as u32));
            if let Some((true, picture)) = picture {
                picture::forget(&picture, &self.pictures);
            }
        }
        closed
    }

    /// Answers one of the shell's asks, and whether it changed anything
    /// held.
    pub fn ask(&mut self, said: &Value, now: i64) -> (Value, bool) {
        let one = said["notice"]
            .as_u64()
            .and_then(|id| u32::try_from(id).ok());
        let many: Vec<u32> = said["notices"]
            .as_array()
            .map(|ids| {
                ids.iter()
                    .filter_map(Value::as_u64)
                    .filter_map(|id| u32::try_from(id).ok())
                    .collect()
            })
            .unwrap_or_default();
        let done = |changed: bool| (json!({ "done": true }), changed);
        match said["ask"].as_str().unwrap_or("") {
            "invoke" => self.invoke(one, said["action"].as_str().unwrap_or(""), said, now),
            "reply" => self.reply(one, said["text"].as_str().unwrap_or(""), now),
            "dismiss" => done(self.close_all(&many, Closed::Dismissed, now)),
            "expire" => done(self.close_all(&many, Closed::Expired, now)),
            "rest" => done(one.is_some_and(|id| self.holding.rest(id))),
            "wake" => done(one.is_some_and(|id| self.holding.wake(id))),
            "raise" => done(one.is_some_and(|id| self.holding.raise(id, now))),
            "mark" => {
                let read = said["read"].as_bool().unwrap_or(true);
                done(self.holding.mark(&many, read, said["how"].as_str(), now))
            }
            "seen" => done(self.holding.seen()),
            "settle" => {
                let standing: BTreeSet<String> = said["standing"]
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default();
                done(self.holding.settle(&standing))
            }
            "quiet" => {
                self.sharing = said["sharing"].as_bool().unwrap_or(false);
                done(false)
            }
            other => (refused("unknown-ask", &format!("no ask {other}")), false),
        }
    }

    /// Closes every one named, saying whether any was, rather than
    /// stopping at the first closed.
    fn close_all(&mut self, ids: &[u32], why: Closed, now: i64) -> bool {
        let mut any = false;
        for id in ids {
            any |= self.close(*id, why, now);
        }
        any
    }

    fn invoke(&mut self, id: Option<u32>, action: &str, said: &Value, now: i64) -> (Value, bool) {
        let Some(notice) = id
            .and_then(|id| self.holding.get(id))
            .filter(|notice| notice.live)
        else {
            return (refused("not-open", "that notice is no longer open"), false);
        };
        if !notice.actions.iter().any(|offered| offered.id == action) {
            return (
                refused("not-offered", &format!("it offers no {action}")),
                false,
            );
        }
        let (id, resident) = (notice.id, notice.resident);
        if let Some(token) = said["token"].as_str().filter(|token| !token.is_empty()) {
            self.owed.push(Signal::Token(id, token.to_owned()));
        }
        self.owed.push(Signal::Invoked(id, action.to_owned()));
        // Read as it's acted on where the ask says, as a row's acts each
        // read it, in the one change rather than another ask after.
        let read = said["read"] == true && self.holding.mark(&[id], true, None, now);
        let closed = !resident && self.close(id, Closed::Dismissed, now);
        (json!({ "done": true }), closed || read)
    }

    fn reply(&mut self, id: Option<u32>, text: &str, now: i64) -> (Value, bool) {
        let Some(notice) = id
            .and_then(|id| self.holding.get(id))
            .filter(|notice| notice.live)
        else {
            return (refused("not-open", "that notice is no longer open"), false);
        };
        if notice.reply.is_none() {
            return (refused("not-offered", "it takes no reply"), false);
        }
        let (id, resident) = (notice.id, notice.resident);
        self.owed.push(Signal::Replied(id, text.to_owned()));
        let closed = !resident && self.close(id, Closed::Dismissed, now);
        (json!({ "done": true }), closed)
    }
}

/// An ask refused, as the daemon interface says one.
fn refused(code: &str, message: &str) -> Value {
    json!({ "error": { "code": code, "message": message } })
}

/// The actions as offered, each key then its words, and the reply in
/// place, offered as the `inline-reply` action whose words are what its
/// line says before anything's typed. A list of odd length is offered
/// whole but for its last key, which has no words.
fn split(said: &[String]) -> (Vec<Action>, Option<String>) {
    let mut actions = Vec::new();
    let mut reply = None;
    for [id, text] in said.as_chunks::<2>().0 {
        if id == "inline-reply" {
            reply.get_or_insert_with(|| text.clone());
        } else {
            actions.push(Action {
                id: id.clone(),
                text: text.clone(),
            });
        }
    }
    (actions, reply)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A state of its own, in a folder named for this file as well as
    /// the case: every file's cases run at once in one process, so a
    /// folder named by the case and the process alone was judged.rs's
    /// quiet case's too, and this one emptied it as that read it.
    fn state(name: &str) -> State {
        let folder =
            std::env::temp_dir().join(format!("notices-daemon-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("a folder");
        State::new(
            Holding::default(),
            Judge::default(),
            folder.join("quiet"),
            folder.join("pictures"),
        )
    }

    fn sent(summary: &str, actions: &[&str]) -> Sent {
        Sent {
            app: "mail".into(),
            summary: summary.into(),
            actions: actions.iter().map(|one| (*one).to_owned()).collect(),
            expire: -1,
            ..Sent::default()
        }
    }

    fn with_hint(mut sent: Sent, name: &str, value: Value) -> Sent {
        sent.hints.insert(name.into(), value);
        sent
    }

    #[test]
    fn each_sent_gets_an_id_of_its_own_and_one_replacing_keeps_its() {
        let mut state = state("ids");
        let first = state.notify(sent("one", &[]), 10);
        let second = state.notify(sent("two", &[]), 11);
        assert_ne!(first, second);
        let again = state.notify(
            Sent {
                replaces: first,
                ..sent("one again", &[])
            },
            12,
        );
        assert_eq!(again, first);
        assert_eq!(state.holding.get(first).expect("held").summary, "one again");
        let gone = state.notify(
            Sent {
                replaces: 99,
                ..sent("unknown", &[])
            },
            13,
        );
        assert!(gone != 99 && gone > second, "replacing none is a new one");
    }

    #[test]
    fn its_actions_reply_and_hints_are_read_as_its_sender_said_them() {
        let mut state = state("hints");
        let mut said = sent(
            "Ana",
            &[
                "default",
                "Open",
                "inline-reply",
                "Reply to Ana",
                "read",
                "Mark Read",
            ],
        );
        said = with_hint(said, "urgency", json!(2));
        said = with_hint(said, "desktop-entry", json!("slack"));
        said = with_hint(said, "category", json!("im.received"));
        said = with_hint(said, "resident", json!(true));
        said = with_hint(said, "image-path", json!("/home/someone/ana.png"));
        let id = state.notify(said, 10);
        let notice = state.holding.get(id).expect("held");
        let keys: Vec<&str> = notice
            .actions
            .iter()
            .map(|action| action.id.as_str())
            .collect();
        assert_eq!(keys, ["default", "read"]);
        assert_eq!(notice.reply.as_deref(), Some("Reply to Ana"));
        assert_eq!(notice.urgency, Urgency::Critical);
        assert_eq!((notice.urgent, notice.resident), (true, true));
        assert_eq!(notice.entry, "slack");
        assert_eq!(notice.category, "im.received");
        assert_eq!(notice.picture, "file:///home/someone/ana.png");
    }

    #[test]
    fn quiet_in_its_file_or_from_the_share_holds_one_back() {
        let mut state = state("quiet");
        std::fs::write(&state.quiet, "5 0\n").expect("written");
        let held = state.notify(sent("held", &[]), 10);
        std::fs::write(&state.quiet, "").expect("written");
        let shown = state.notify(sent("shown", &[]), 11);
        state.ask(&json!({ "ask": "quiet", "sharing": true }), 12);
        let shared = state.notify(sent("while shared", &[]), 13);
        let resting = |id| state.holding.get(id).expect("held").resting;
        assert_eq!(
            (resting(held), resting(shown), resting(shared)),
            (true, false, true)
        );
    }

    #[test]
    fn one_held_back_and_transient_closes_at_once_and_its_sender_is_told() {
        let mut state = state("transient");
        std::fs::write(&state.quiet, "5 0\n").expect("written");
        let id = state.notify(
            with_hint(sent("passing", &[]), "transient", json!(true)),
            10,
        );
        assert!(
            state.holding.get(id).is_none(),
            "a passing notice held back is closed, not kept"
        );
        assert_eq!(state.owed, [Signal::Closed(id, 2)]);
    }

    #[test]
    fn an_action_done_sends_its_token_first_then_closes_it() {
        let mut state = state("invoke");
        let id = state.notify(sent("mail", &["default", "Open"]), 10);
        let (answer, changed) = state.ask(
            &json!({ "ask": "invoke", "notice": id, "action": "default", "token": "tok" }),
            11,
        );
        assert_eq!(answer, json!({ "done": true }));
        assert!(
            changed,
            "an action done closes its notice, which changes it"
        );
        assert_eq!(
            state.owed,
            [
                Signal::Token(id, "tok".into()),
                Signal::Invoked(id, "default".into()),
                Signal::Closed(id, 2)
            ]
        );
        let (refused, _) = state.ask(
            &json!({ "ask": "invoke", "notice": id, "action": "default" }),
            12,
        );
        assert_eq!(refused["error"]["code"], "not-open");
    }

    #[test]
    fn an_action_done_that_says_so_reads_it() {
        let mut state = state("invoke-read");
        let id = state.notify(sent("mail", &["default", "Open"]), 10);
        state.ask(
            &json!({ "ask": "invoke", "notice": id, "action": "default", "read": true }),
            11,
        );
        assert!(
            state.holding.get(id).is_some_and(|notice| notice.read),
            "read as it's done"
        );
    }

    #[test]
    fn an_action_not_offered_is_refused_and_one_resident_stays_open() {
        let mut state = state("resident");
        let id = state.notify(
            with_hint(sent("song", &["next", "Next"]), "resident", json!(true)),
            10,
        );
        let (refused, _) = state.ask(
            &json!({ "ask": "invoke", "notice": id, "action": "default" }),
            11,
        );
        assert_eq!(refused["error"]["code"], "not-offered");
        state.ask(
            &json!({ "ask": "invoke", "notice": id, "action": "next" }),
            12,
        );
        assert_eq!(state.owed, [Signal::Invoked(id, "next".into())]);
        assert!(
            state.holding.is_live(id),
            "a resident notice stays open once acted on"
        );
    }

    #[test]
    fn a_reply_is_sent_only_where_one_is_taken() {
        let mut state = state("reply");
        let plain = state.notify(sent("plain", &[]), 10);
        let chat = state.notify(sent("chat", &["inline-reply", "Reply"]), 11);
        let (refused, _) = state.ask(
            &json!({ "ask": "reply", "notice": plain, "text": "hi" }),
            12,
        );
        assert_eq!(refused["error"]["code"], "not-offered");
        state.ask(
            &json!({ "ask": "reply", "notice": chat, "text": "on my road" }),
            13,
        );
        assert_eq!(
            state.owed,
            [
                Signal::Replied(chat, "on my road".into()),
                Signal::Closed(chat, 2)
            ]
        );
    }

    #[test]
    fn put_away_expired_and_withdrawn_each_tell_its_sender_why() {
        let mut state = state("closes");
        let (a, b, c) = (
            state.notify(sent("a", &[]), 1),
            state.notify(sent("b", &[]), 2),
            state.notify(sent("c", &[]), 3),
        );
        state.ask(&json!({ "ask": "dismiss", "notices": [a] }), 4);
        state.ask(&json!({ "ask": "expire", "notices": [b] }), 5);
        state.withdraw(c, 6);
        state.withdraw(c, 7);
        assert_eq!(
            state.owed,
            [
                Signal::Closed(a, 2),
                Signal::Closed(b, 1),
                Signal::Closed(c, 3)
            ]
        );
    }

    #[test]
    fn marked_seen_settled_and_rested_as_the_shell_asks() {
        let mut state = state("marks");
        let id = state.notify(
            with_hint(sent("to-do", &[]), "x-waiting", json!("todo:1")),
            1,
        );
        let mut changed = Vec::new();
        for (at, ask) in [
            json!({ "ask": "rest", "notice": id }),
            json!({ "ask": "wake", "notice": id }),
            json!({ "ask": "seen" }),
            json!({ "ask": "settle", "standing": [] }),
        ]
        .iter()
        .enumerate()
        {
            changed.push(state.ask(ask, 2 + i64::try_from(at).expect("few")).1);
        }
        assert_eq!(
            changed,
            [true, true, true, true],
            "rest, wake, seen, settle"
        );
        assert_eq!(state.holding.get(id).map(|notice| notice.read), Some(true));
        let unmarked = state.ask(&json!({ "ask": "mark", "notices": [id], "read": false }), 6);
        let raised = state.ask(&json!({ "ask": "raise", "notice": id }), 7);
        let (unknown, shouted) = state.ask(&json!({ "ask": "shout" }), 8);
        assert_eq!(unknown["error"]["code"], "unknown-ask");
        // Marked unread and raised change it; an unknown ask doesn't.
        assert_eq!((unmarked.1, raised.1, shouted), (true, true, false));
    }
}
