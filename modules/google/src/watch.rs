//! The account as it stands: one look reads who's signed in, gets a token,
//! asks for the meetings and says how it all stands; an answer to an
//! invitation is asked the same way, and changes what's held at once.

use std::path::PathBuf;

use jiff::Zoned;

use crate::account::{self, Missing};
use crate::api::{Google, Refused};
use crate::calendar::{self, Answer};
use crate::state::{Standing, State};
use crate::token::Keeper;

pub struct Watch {
    pub google: Google,
    pub credentials: PathBuf,
    /// The account to ask as, or the file's default.
    pub account: Option<String>,
    keeper: Keeper,
    state: State,
}

impl Watch {
    pub fn new(google: Google, credentials: PathBuf, account: Option<String>) -> Watch {
        Watch {
            google,
            credentials,
            account,
            keeper: Keeper::default(),
            state: State::default(),
        }
    }

    /// How the account stands at `now`, asking Google if it's signed in.
    /// Unreachable, what was held is kept, with when it was heard.
    pub fn look(&mut self, now: &Zoned) -> &State {
        let (from, to) = calendar::window(now);
        match self.asking(now, |google, token| google.events(token, from, to)) {
            Ok(answer) => {
                self.state = State {
                    account: Standing::Ready,
                    meetings: calendar::meetings(&answer, now.time_zone()),
                    heard: Some(now.timestamp().as_millisecond()),
                };
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

enum Looked {
    Missing,
    Refused(Refused),
}

impl From<Refused> for Looked {
    fn from(refused: Refused) -> Looked {
        Looked::Refused(refused)
    }
}
