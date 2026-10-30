//! slack-people CREDENTIALS [ORIGIN]
//!
//! Who Slack knows by an email address, for People's Slack source
//! (SlackSource.qml): the shell writes an address a line on standard
//! input, as Joel steps into someone, and hears one JSON line back for
//! each (lib.rs's `Found`): their id in Slack and its workspace, which
//! open them there, their name, their time zone, whether they're active,
//! away or in a huddle, their status, and their direct messages with Joel
//! (lib.rs's `Conversation`), or no id for an address Slack knows nobody
//! by. Who an address is is found once and kept between runs (lib.rs's
//! `Kept`, in the cache home), so someone known is said at once, then
//! again once Slack says how they are and what they last said, which is
//! asked each time; when Slack can't be reached then, what it said last
//! is said again. What they said is only ever held while it runs.
//!
//! A line `?ADDRESS` asks ahead, for someone Joel may step into: who they
//! are and their conversation are found, once a run, not how they are,
//! and only while no step-in is waiting, a search every few seconds, so
//! asking ahead about everyone People lists stays inside Slack's limits
//! and never holds up a step-in.
//!
//! Once Slack refuses the sign-in, the line `{"signedOut":true}` is said
//! and nobody more is asked about. With no sign-in in CREDENTIALS, the
//! harness's file, it asks nothing and says `{"signedIn":false}` to each
//! line, so the shell waits on no answer. ORIGIN is Slack's, unless a
//! test names a stand-in.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

use slack_people::{Account, Found, Kept, Refused, Slack, asked, now};

/// How long the next look-up ahead waits for each search Slack was asked
/// since the last, a step-in's too: an enterprise grid's search allows
/// about twenty a minute.
const PACE: Duration = Duration::from_secs(3);

/// How long any look-up ahead waits at least, so a step-in the shell
/// writes with it goes first.
const SETTLE: Duration = Duration::from_millis(100);

/// What's said with no sign-in to ask with.
const NOT_SIGNED_IN: &str = r#"{"signedIn":false}"#;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(credentials) = args.next().map(PathBuf::from) else {
        eprintln!("usage: slack-people CREDENTIALS [ORIGIN]");
        std::process::exit(64);
    };
    let slack = Slack::new(args.next().unwrap_or_else(|| "https://slack.com".into()));
    let mut asker = Asker {
        slack,
        credentials,
        known: HashMap::new(),
        kept: Kept::at(kept_in()),
        talked: HashSet::new(),
        signed_out: false,
    };
    let (lines, heard) = mpsc::channel();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            if lines.send(line).is_err() {
                break;
            }
        }
    });
    let mut ahead = VecDeque::new();
    let mut queued = HashSet::new();
    let mut ended = false;
    let mut wait = SETTLE;
    while !asker.signed_out {
        // A step-in is taken the moment it comes; one ahead only once
        // none has for a while, and every one left once the shell is gone.
        let line = if ended {
            None
        } else if ahead.is_empty() {
            heard.recv().ok()
        } else {
            match heard.recv_timeout(wait) {
                Ok(line) => Some(line),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => {
                    ended = true;
                    None
                }
            }
        };
        let said = if let Some(line) = line {
            if let Some(email) = line.trim().strip_prefix('?') {
                let email = email.to_lowercase();
                if asked(&email) && queued.insert(email.clone()) {
                    ahead.push_back(email);
                }
                continue;
            }
            asker.step_in(&line.trim().to_lowercase())
        } else {
            let Some(email) = ahead.pop_front() else {
                if ended || line_ended(&heard) {
                    break;
                }
                continue;
            };
            let said = asker.ahead(&email);
            wait = match asker.slack.searched() {
                0 => SETTLE,
                searches => PACE * searches,
            };
            said
        };
        if !say(&said) {
            break;
        }
    }
}

/// Whether the shell has gone, with nothing more to read.
fn line_ended(heard: &mpsc::Receiver<String>) -> bool {
    matches!(heard.try_recv(), Err(mpsc::TryRecvError::Disconnected))
}

/// Where who was found is kept: the cache home's, since it's only ever
/// what Slack would say again.
fn kept_in() -> Option<PathBuf> {
    let home = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|home| home.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    Some(home.join("slack-people").join("known.json"))
}

/// Says each line to the shell, or false once it's stopped listening.
fn say(lines: &[String]) -> bool {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    lines
        .iter()
        .all(|line| writeln!(out, "{line}").and_then(|()| out.flush()).is_ok())
}

/// Slack, the sign-in, and who's been found.
struct Asker {
    slack: Slack,
    credentials: PathBuf,
    /// Who each address is, as said last while running, how they are too.
    known: HashMap<String, Found>,
    kept: Kept,
    /// Whose conversation Slack has been asked for this run.
    talked: HashSet<String>,
    signed_out: bool,
}

impl Asker {
    /// What to say as Joel steps into someone, once Slack has answered:
    /// who they are, said here at once when it's known, before Slack is
    /// asked, then what Slack says of them now, unless it's the same.
    fn step_in(&mut self, email: &str) -> Vec<String> {
        if !asked(email) {
            return Vec::new();
        }
        // The file is read for each address, so signing in again from pi
        // is taken without a restart.
        let Some(account) = Account::read(&self.credentials) else {
            return vec![NOT_SIGNED_IN.to_owned()];
        };
        let had = self
            .known
            .get(email)
            .cloned()
            .or_else(|| self.kept.get(email, now()).cloned());
        let said: Vec<String> = had.iter().filter_map(line).collect();
        // A shell gone is found by the next line said, so nothing more
        // is needed of this one.
        let _ = say(&said);
        let found = match &had {
            Some(had) if had.id.is_none() => return Vec::new(),
            Some(had) => self.slack.again(&account, had),
            None => self.slack.look_up(&account, email),
        }
        .and_then(|found| present(&self.slack, &account, found))
        .map(|found| self.talking(&account, found));
        match self.heard(found) {
            Heard::Found(found) => line(&found)
                .filter(|fresh| !said.contains(fresh))
                .into_iter()
                .collect(),
            Heard::SignedOut(line) => vec![line],
            // What was said already, from what was known, stands.
            Heard::Unreachable => Vec::new(),
        }
    }

    /// What to say of someone asked about ahead: who they are, kept or
    /// found now, with their conversation, asked once a run.
    fn ahead(&mut self, email: &str) -> Vec<String> {
        let had = self
            .known
            .get(email)
            .cloned()
            .or_else(|| self.kept.get(email, now()).cloned());
        if let Some(had) = had
            .as_ref()
            .filter(|had| had.id.is_none() || self.talked.contains(email))
        {
            return line(had).into_iter().collect();
        }
        let Some(account) = Account::read(&self.credentials) else {
            return vec![NOT_SIGNED_IN.to_owned()];
        };
        let found = match had {
            Some(had) if !had.handle.is_empty() => Ok(had),
            // Kept before handles were, so asked who they are again.
            Some(had) => self.slack.again(&account, &had),
            None => self.slack.look_up(&account, email),
        }
        .map(|found| self.talking(&account, found));
        match self.heard(found) {
            Heard::Found(found) => line(&found).into_iter().collect(),
            Heard::SignedOut(line) => vec![line],
            Heard::Unreachable => Vec::new(),
        }
    }

    /// Someone found, with their conversation with Joel as Slack finds it
    /// now, or as it was last found when Slack can't say.
    fn talking(&self, account: &Account, found: Found) -> Found {
        let conversation = match self.slack.conversation(account, &found) {
            Ok(conversation) => conversation,
            Err(refused) => {
                if let Refused::Unreachable(why) = refused {
                    eprintln!("<4>{why}");
                }
                self.known
                    .get(&found.email)
                    .and_then(|had| had.conversation.clone())
            }
        };
        Found {
            conversation,
            ..found
        }
    }

    /// Takes what Slack answered: keeps who was found, or stops asking
    /// anyone once it refuses the sign-in.
    fn heard(&mut self, found: Result<Found, Refused>) -> Heard {
        match found {
            Ok(found) => {
                self.kept.keep(&found, now());
                self.talked.insert(found.email.clone());
                self.known.insert(found.email.clone(), found.clone());
                Heard::Found(Box::new(found))
            }
            Err(Refused::SignedOut) => {
                self.signed_out = true;
                Heard::SignedOut(r#"{"signedOut":true}"#.to_owned())
            }
            Err(Refused::Unreachable(why)) => {
                eprintln!("<4>{why}");
                Heard::Unreachable
            }
        }
    }
}

/// What came of asking Slack.
enum Heard {
    /// Boxed, since what they said makes it far the largest.
    Found(Box<Found>),
    SignedOut(String),
    Unreachable,
}

/// Someone as the shell is told them.
fn line(found: &Found) -> Option<String> {
    serde_json::to_string(found).ok()
}

/// Someone found, with whether they're active or away, unless their
/// profile already says they're in a huddle. Presence Slack can't give
/// now is left unsaid rather than holding back who they are.
fn present(slack: &Slack, account: &Account, found: Found) -> Result<Found, Refused> {
    let Some(id) = found.id.as_deref().filter(|_| found.presence.is_empty()) else {
        return Ok(found);
    };
    match slack.presence(account, id) {
        Ok(presence) => Ok(Found { presence, ..found }),
        Err(Refused::Unreachable(why)) => {
            eprintln!("<4>{why}");
            Ok(found)
        }
        Err(refused) => Err(refused),
    }
}
