//! One look at the account: read who's signed in, get a token, ask for the
//! meetings, and say how it all stands.

use std::path::PathBuf;

use jiff::Zoned;

use crate::account::{self, Missing};
use crate::api::{Google, Refused};
use crate::calendar;
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
        self.state = match self.ask(now) {
            Ok(meetings) => State {
                account: Standing::Ready,
                meetings,
                heard: Some(now.timestamp().as_millisecond()),
            },
            Err(Looked::Missing) => State::without(Standing::None),
            Err(Looked::Refused(Refused::SignedOut)) => State::without(Standing::SignedOut),
            Err(Looked::Refused(Refused::Unreachable(why))) => {
                eprintln!("<4>{why}");
                State {
                    account: Standing::Unreachable,
                    ..self.state.clone()
                }
            }
        };
        &self.state
    }

    fn ask(&mut self, now: &Zoned) -> Result<Vec<calendar::Meeting>, Looked> {
        let account = account::read(&self.credentials, self.account.as_deref())
            .map_err(|_: Missing| Looked::Missing)?;
        let at = now.timestamp().as_millisecond();
        let (from, to) = calendar::window(now);
        let google = &self.google;
        let token = self
            .keeper
            .token(&account, at, |account| google.refresh(account))?;
        let answer = match google.events(&token, from, to) {
            // A token that looked fresh but wasn't, as one revoked is:
            // once more, with a new one.
            Err(Refused::SignedOut) => {
                self.keeper.spurn(&token);
                let token = self
                    .keeper
                    .token(&account, at, |account| google.refresh(account))?;
                let again = google.events(&token, from, to);
                // Refused with a token just made, the sign-in itself is no
                // good for the calendar: it waits for a new one.
                if again == Err(Refused::SignedOut) {
                    self.keeper.refuse(&account);
                }
                again?
            }
            answer => answer?,
        };
        Ok(calendar::meetings(&answer, now.time_zone()))
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
