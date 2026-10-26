//! Which access token to ask Google with.
//!
//! pi's own access token, from the file, is used while it has five minutes
//! left, as pi itself judges one fresh, so a machine where pi is busy
//! seldom refreshes here at all. Otherwise this program refreshes one of its
//! own and keeps it in memory until it too nears its end. A refresh token
//! Google refused isn't offered again: signing in again from pi writes a
//! new one, which is tried as soon as the file says it.

use crate::account::Account;
use crate::api::{Access, Refused};

/// How long before its end a token stops counting as fresh.
const MARGIN: i64 = 5 * 60 * 1000;
/// How many tokens Google said were no good are remembered.
const SPURNED: usize = 4;

#[derive(Default)]
pub struct Keeper {
    /// The token this program refreshed last.
    held: Option<Access>,
    /// Access tokens Google said were no good, never offered again: the
    /// last few, since an access token lasts an hour and pi's file and
    /// this program each hold one.
    spurned: Vec<String>,
    /// A refresh token Google refused.
    refused: Option<String>,
}

impl Keeper {
    /// A token to ask with at `now` (milliseconds), refreshing through
    /// `refresh` only when neither pi's nor this program's is fresh.
    pub fn token(
        &mut self,
        account: &Account,
        now: i64,
        refresh: impl FnOnce(&Account) -> Result<Access, Refused>,
    ) -> Result<String, Refused> {
        if self.refused.as_deref() == Some(account.refresh_token.as_str()) {
            return Err(Refused::SignedOut);
        }
        let spurned = |token: &str| self.spurned.iter().any(|spurned| spurned == token);
        if !account.access_token.is_empty()
            && account.expires - now > MARGIN
            && !spurned(&account.access_token)
        {
            return Ok(account.access_token.clone());
        }
        if let Some(held) = &self.held
            && held.expires - now > MARGIN
            && !spurned(&held.token)
        {
            return Ok(held.token.clone());
        }
        match refresh(account) {
            Ok(access) => {
                let token = access.token.clone();
                self.held = Some(access);
                self.refused = None;
                Ok(token)
            }
            Err(Refused::SignedOut) => {
                self.held = None;
                self.refused = Some(account.refresh_token.clone());
                Err(Refused::SignedOut)
            }
            Err(other) => Err(other),
        }
    }

    /// Google refused even a token just refreshed: the account's sign-in
    /// isn't offered again until the file holds a new one.
    pub fn refuse(&mut self, account: &Account) {
        self.held = None;
        self.refused = Some(account.refresh_token.clone());
    }

    /// Google said `token` is no good, though it looked fresh: the next
    /// ask refreshes.
    pub fn spurn(&mut self, token: &str) {
        if self.spurned.len() == SPURNED {
            self.spurned.remove(0);
        }
        self.spurned.push(token.to_owned());
        if self.held.as_ref().is_some_and(|held| held.token == token) {
            self.held = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_000_000_000;

    fn account(expires: i64) -> Account {
        Account {
            client_id: "app".into(),
            client_secret: "secret".into(),
            refresh_token: "refresh".into(),
            access_token: "pis".into(),
            expires,
        }
    }

    /// A refresh that gives `token`, good for an hour.
    fn fresh(token: &str) -> impl FnOnce(&Account) -> Result<Access, Refused> {
        let token = token.to_owned();
        move |_| {
            Ok(Access {
                token,
                expires: NOW + 3_600_000,
            })
        }
    }

    fn never(_: &Account) -> Result<Access, Refused> {
        panic!("refreshed when it needn't have")
    }

    #[test]
    fn pis_token_is_used_while_it_has_five_minutes_left() {
        let mut keeper = Keeper::default();
        assert_eq!(
            keeper
                .token(&account(NOW + MARGIN + 1), NOW, never)
                .unwrap(),
            "pis"
        );
    }

    #[test]
    fn a_token_near_its_end_is_refreshed_and_kept() {
        let mut keeper = Keeper::default();
        let near = account(NOW + MARGIN);
        assert_eq!(keeper.token(&near, NOW, fresh("mine")).unwrap(), "mine");
        assert_eq!(keeper.token(&near, NOW + 60_000, never).unwrap(), "mine");
    }

    #[test]
    fn a_refused_refresh_token_is_not_offered_again() {
        let mut keeper = Keeper::default();
        let old = account(0);
        let refused = keeper.token(&old, NOW, |_| Err(Refused::SignedOut));
        assert_eq!(refused, Err(Refused::SignedOut));
        assert_eq!(keeper.token(&old, NOW, never), Err(Refused::SignedOut));
        let signed_in_again = Account {
            refresh_token: "new".into(),
            ..account(0)
        };
        assert_eq!(
            keeper.token(&signed_in_again, NOW, fresh("mine")).unwrap(),
            "mine"
        );
    }

    #[test]
    fn an_unreachable_google_is_asked_again_next_time() {
        let mut keeper = Keeper::default();
        let old = account(0);
        let offline = keeper.token(&old, NOW, |_| Err(Refused::Unreachable("offline".into())));
        assert!(matches!(offline, Err(Refused::Unreachable(_))));
        assert_eq!(keeper.token(&old, NOW, fresh("mine")).unwrap(), "mine");
    }

    #[test]
    fn a_spurned_token_is_refreshed_past() {
        let mut keeper = Keeper::default();
        let current = account(NOW + 3_600_000);
        keeper.spurn("pis");
        assert_eq!(keeper.token(&current, NOW, fresh("mine")).unwrap(), "mine");
        keeper.spurn("mine");
        assert_eq!(
            keeper.token(&current, NOW, fresh("again")).unwrap(),
            "again"
        );
    }

    #[test]
    fn a_refused_sign_in_waits_for_a_new_one() {
        let mut keeper = Keeper::default();
        let current = account(NOW + 3_600_000);
        keeper.refuse(&current);
        assert_eq!(keeper.token(&current, NOW, never), Err(Refused::SignedOut));
    }
}
