//! The account as it stands: one look reads who's signed in, gets a token,
//! asks for the meetings and the mail that wants Joel, and says how it all
//! stands; an answer to an invitation, a meeting cancelled or moved, and
//! mail marked read or archived, are asked the same way, and change what's
//! held at once.

use std::path::PathBuf;

use jiff::{Timestamp, Zoned};
use serde_json::Value;

use crate::account::{self, Missing};
use crate::api::{Google, Refused};
use crate::calendar::{self, Answer};
use crate::mail::{self, Mail, Tidy};
use crate::state::{Standing, State};
use crate::token::Keeper;

pub struct Watch {
    pub google: Google,
    pub credentials: PathBuf,
    /// The account to ask as, or the file's default.
    pub account: Option<String>,
    keeper: Keeper,
    state: State,
    /// The calendars Joel may change events on, as last heard.
    writable: Vec<String>,
    /// The mail marked read since it was last looked at, so marking one
    /// unread again brings it back at once, before Gmail lists it again.
    read: Vec<Mail>,
}

impl Watch {
    pub fn new(google: Google, credentials: PathBuf, account: Option<String>) -> Watch {
        Watch {
            google,
            credentials,
            account,
            keeper: Keeper::default(),
            state: State::default(),
            writable: Vec::new(),
            read: Vec::new(),
        }
    }

    /// How the account stands at `now`, asking Google if it's signed in.
    /// Unreachable, what was held is kept, with when it was heard. The
    /// mail is asked after the meetings, so Gmail failing alone keeps the
    /// mail held and costs the meetings nothing. Which calendars Joel may
    /// change events on is asked with the meetings, and kept as last heard
    /// when only that can't be.
    pub fn look(&mut self, now: &Zoned) -> &State {
        let (from, to) = calendar::window(now);
        match self.asking(now, |google, token| google.events(token, from, to)) {
            Ok(mut answer) => {
                match self.asking(now, Google::writable) {
                    Ok(writable) => self.writable = writable,
                    Err(Looked::Refused(Refused::Unreachable(why))) => eprintln!("<4>{why}"),
                    Err(looked) => {
                        self.fell(looked, Standing::Unreachable);
                        return &self.state;
                    }
                }
                answer["writable"] = Value::from(self.writable.clone());
                self.state.account = Standing::Ready;
                self.state.meetings = calendar::meetings(&answer, now.time_zone());
                self.state.heard = Some(now.timestamp().as_millisecond());
            }
            Err(looked) => {
                self.fell(looked, Standing::Unreachable);
                return &self.state;
            }
        }
        let held = std::mem::take(&mut self.state.mail);
        match self.asking(now, |google, token| wanted(google, token, &held)) {
            Ok((found, unread)) => {
                self.state.mail = found;
                self.state.unread = unread;
                // What Gmail lists now says what's unread, any marked
                // unread again from elsewhere included.
                self.read.clear();
            }
            Err(Looked::Refused(Refused::Unreachable(why))) => {
                eprintln!("<4>{why}");
                self.state.mail = held;
            }
            Err(looked) => self.fell(looked, Standing::Unreachable),
        }
        &self.state
    }

    /// Answers the invitation to the meeting `id` as `with`, and holds the
    /// meeting as answered so the shell hears it at once, before the next
    /// look says it again. An answer Google didn't take changes nothing
    /// held, and Google still answering the looks is still ready.
    pub fn answer(&mut self, id: &str, with: Answer, now: &Zoned) -> &State {
        match self.asking(now, |google, token| google.answer(token, id, with)) {
            Ok(()) => {
                for meeting in &mut self.state.meetings {
                    if meeting.id == id {
                        meeting.answer = Some(with);
                    }
                }
            }
            Err(looked) => self.fell(looked, self.state.account),
        }
        &self.state
    }

    /// Cancels the meeting `id` and lets it go from what's held, so the
    /// shell hears it at once. Only a meeting held as Joel's is cancelled:
    /// deleting one he's only asked to would take it off his calendar alone
    /// and tell nobody, which isn't cancelling it. One Google didn't take
    /// changes nothing held.
    pub fn cancel(&mut self, id: &str, now: &Zoned) -> &State {
        let his = self
            .state
            .meetings
            .iter()
            .any(|meeting| meeting.id == id && meeting.holds);
        if !his {
            eprintln!("<4>a meeting asked to be cancelled isn't one Joel holds");
            return &self.state;
        }
        match self.asking(now, |google, token| google.cancel(token, id)) {
            Ok(()) => self.state.meetings.retain(|meeting| meeting.id != id),
            Err(looked) => self.fell(looked, self.state.account),
        }
        &self.state
    }

    /// Moves the meeting `id` to run from `start` to `end`, through the
    /// calendar Google lets Joel change it on, and holds it moved so the
    /// shell hears it at once. Only a meeting held as one he may change is
    /// moved, and never one lasting all day, whose times are days. One
    /// Google didn't take changes nothing held.
    pub fn reschedule(&mut self, id: &str, start: i64, end: i64, now: &Zoned) -> &State {
        let through = self
            .state
            .meetings
            .iter()
            .find(|meeting| meeting.id == id && !meeting.all_day)
            .and_then(|meeting| meeting.through.clone());
        let (Some(through), Ok(from), Ok(to)) = (
            through,
            Timestamp::from_millisecond(start),
            Timestamp::from_millisecond(end),
        ) else {
            eprintln!("<4>a meeting asked to be moved isn't one Joel may change");
            return &self.state;
        };
        let moved = self.asking(now, |google, token| {
            google.reschedule(token, &through, id, from, to)
        });
        match moved {
            Ok(()) => {
                for meeting in &mut self.state.meetings {
                    if meeting.id == id {
                        meeting.start = start;
                        meeting.end = end;
                    }
                }
                self.state
                    .meetings
                    .sort_by_key(|meeting| (meeting.start, meeting.end));
            }
            Err(looked) => self.fell(looked, self.state.account),
        }
        &self.state
    }

    /// Marks the mail `id` read or archives it, as `how` says, and lets it
    /// go from what's held so the shell hears it at once: it wants Joel no
    /// more either way. Held, it was unread in the inbox, so the inbox has
    /// one fewer unread. Mail Google didn't take changes nothing held.
    pub fn tidy(&mut self, id: &str, how: Tidy, now: &Zoned) -> &State {
        match self.asking(now, |google, token| google.unlabel(token, id, how.label())) {
            Ok(()) => {
                if let Some(at) = self.state.mail.iter().position(|mail| mail.id == id) {
                    let gone = self.state.mail.remove(at);
                    self.state.unread = self.state.unread.map(|unread| unread.saturating_sub(1));
                    if how == Tidy::Read {
                        self.read.retain(|mail| mail.id != id);
                        self.read.push(gone);
                    }
                }
            }
            Err(looked) => self.fell(looked, self.state.account),
        }
        &self.state
    }

    /// Marks the mail `id` unread again, as Gmail's own button does, and,
    /// for one marked read here since it was last looked at, holds it as
    /// wanting Joel at once, in its place by when it came. Other mail
    /// comes back as the next look finds it. Mail Google didn't take
    /// changes nothing held.
    pub fn unread(&mut self, id: &str, now: &Zoned) -> &State {
        match self.asking(now, |google, token| google.label(token, id, "UNREAD")) {
            Ok(()) => {
                if let Some(at) = self.read.iter().position(|mail| mail.id == id) {
                    let back = self.read.remove(at);
                    if !self.state.mail.iter().any(|mail| mail.id == id) {
                        self.state.mail.push(back);
                        self.state
                            .mail
                            .sort_by_key(|mail| std::cmp::Reverse(mail.at));
                        self.state.unread = self.state.unread.map(|unread| unread + 1);
                    }
                }
            }
            Err(looked) => self.fell(looked, self.state.account),
        }
        &self.state
    }

    /// What's held once Google didn't answer: nothing, for an account gone
    /// or signed out, since it may not be Joel's to see any more; what was
    /// held, standing as `unreachable` says, when Google couldn't be
    /// reached.
    fn fell(&mut self, looked: Looked, unreachable: Standing) {
        match looked {
            Looked::Missing => self.state = State::without(Standing::None),
            Looked::Refused(Refused::SignedOut) => {
                self.state = State::without(Standing::SignedOut);
            }
            Looked::Refused(Refused::Unreachable(why)) => {
                eprintln!("<4>{why}");
                self.state.account = unreachable;
            }
        }
    }

    /// Asks Google as the account, with a token it'll take.
    fn asking<T>(
        &mut self,
        now: &Zoned,
        ask: impl Fn(&Google, &str) -> Result<T, Refused>,
    ) -> Result<T, Looked> {
        let account = account::read(&self.credentials, self.account.as_deref())
            .map_err(|_: Missing| Looked::Missing)?;
        let at = now.timestamp().as_millisecond();
        let google = &self.google;
        let token = self
            .keeper
            .token(&account, at, |account| google.refresh(account))?;
        match ask(google, &token) {
            // A token that looked fresh but wasn't, as one revoked is:
            // once more, with a new one.
            Err(Refused::SignedOut) => {
                self.keeper.spurn(&token);
                let token = self
                    .keeper
                    .token(&account, at, |account| google.refresh(account))?;
                let again = ask(google, &token);
                // Refused with a token just made, the sign-in itself is no
                // good for the calendar: it waits for a new one.
                if matches!(again, Err(Refused::SignedOut)) {
                    self.keeper.refuse(&account);
                }
                Ok(again?)
            }
            answer => Ok(answer?),
        }
    }
}

/// The mail that wants Joel, newest first, and how many in the inbox are
/// unread. Mail already held isn't asked for again, since what Gmail
/// lists of it never changes.
fn wanted(
    google: &Google,
    token: &str,
    held: &[Mail],
) -> Result<(Vec<Mail>, Option<u64>), Refused> {
    let mut found = Vec::new();
    for id in mail::listed(&google.wanting(token)?) {
        if let Some(kept) = held.iter().find(|kept| kept.id == id) {
            found.push(kept.clone());
            continue;
        }
        match google.mail(token, &id) {
            Ok(answer) => found.extend(mail::read(&answer)),
            // Gone between the list and the ask, as mail archived
            // elsewhere is; the next look lists it again if it's not.
            Err(Refused::Unreachable(_)) => {}
            Err(refused) => return Err(refused),
        }
    }
    found.sort_by_key(|mail| std::cmp::Reverse(mail.at));
    Ok((found, mail::unread(&google.inbox(token)?)))
}

enum Looked {
    Missing,
    Refused(Refused),
}

impl From<Refused> for Looked {
    fn from(refused: Refused) -> Looked {
        Looked::Refused(refused)
    }
}
