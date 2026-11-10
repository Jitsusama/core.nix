//! Tokens kept between asks, in the account's runtime folder, which only
//! the account may read and which empties at logout.
//!
//! Only programs already holding the person's gcloud login can read them,
//! and a sandbox's fence leaves the folder out, so keeping them there
//! hands nobody more than they had.

use std::fs::{DirBuilder, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::account::Account;
use crate::gcloud::{Ask, is_token};

/// A minted token lasts an hour; one is handed out for forty-five minutes
/// of it, so none reaches a command with less than a quarter of an hour.
pub const KEEP: Duration = Duration::from_mins(45);

/// The folder tokens are kept in.
pub struct Held {
    folder: PathBuf,
}

impl Held {
    /// Tokens kept in a folder of their own in the runtime folder `runtime`.
    #[must_use]
    pub fn under(runtime: &Path) -> Held {
        Held {
            folder: runtime.join("gcloud-broker"),
        }
    }

    /// The token kept for what's asked of the account, while it's fresh.
    #[must_use]
    pub fn token(&self, ask: Ask<'_>, account: &Account, now: SystemTime) -> Option<String> {
        let kept = std::fs::read_to_string(self.folder.join(name(ask, account))).ok()?;
        let mut words = kept.split_whitespace();
        let (minted, what, token) = (words.next()?, words.next()?, words.next()?);
        let minted = UNIX_EPOCH + Duration::from_secs(minted.parse().ok()?);
        // A clock that went back reads as a token not yet minted, so it's
        // minted again rather than kept past its hour.
        let age = now.duration_since(minted).ok()?;
        (age < KEEP && what == kind(ask) && is_token(token)).then(|| token.to_owned())
    }

    /// Keeps a token minted `now` for what's asked of the account.
    ///
    /// # Errors
    ///
    /// When the folder or the token's file can't be written.
    pub fn keep(
        &self,
        ask: Ask<'_>,
        account: &Account,
        token: &str,
        now: SystemTime,
    ) -> Result<(), String> {
        let place = self.folder.display();
        let unwritable = |why: std::io::Error| format!("can't keep a token in {place}: {why}");
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&self.folder)
            .map_err(unwritable)?;
        let minted = now
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "the clock is before 1970".to_owned())?
            .as_secs();
        let file = self.folder.join(name(ask, account));
        // Written beside it and moved into place, so a caller at the same
        // moment reads the old token or the new one, never half of one.
        let partial = self
            .folder
            .join(format!(".{}.{}", name(ask, account), std::process::id()));
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&partial)
            .and_then(|mut written| {
                written.write_all(format!("{minted} {} {token}\n", kind(ask)).as_bytes())
            })
            .and_then(|()| std::fs::rename(&partial, &file))
            .map_err(unwritable)
    }
}

/// What a kept token is for, written beside it and checked as it's read:
/// `access`, or an identity token's audience.
fn kind(ask: Ask<'_>) -> &str {
    match ask {
        Ask::Access => "access",
        Ask::Identity(audience) => audience,
    }
}

/// The file a token is kept in, named for the account and what the token
/// is for. An audience can be longer than a file's name may be, so it's
/// named by its FNV-1a hash; a token kept for another audience with the
/// same hash fails `kind`'s check and is minted again.
fn name(ask: Ask<'_>, account: &Account) -> String {
    match ask {
        Ask::Access => format!("{}.access", account.email()),
        Ask::Identity(audience) => {
            let hash = audience.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, b| {
                (hash ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
            });
            format!("{}.identity.{hash:016x}", account.email())
        }
    }
}
