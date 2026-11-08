//! spotify-player's saved sign-in, which this reads and, once its access
//! token has run out, refreshes and writes back, so spotify-player keeps
//! signing in with it.
//!
//! spotify-player keeps the sign-in it got through ncspot's Spotify app,
//! a PKCE app with no secret, as rspotify's token cache: one line of JSON
//! holding `access_token`, `expires_in` in seconds, `expires_at` in RFC
//! 3339, `refresh_token` and `scope`, its scopes apart by spaces. It's
//! written back in that shape, the fields in that order, anything else
//! the file held kept after them, so rspotify reads it as its own.

use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// How long before it runs out an access token counts as spent, so one
/// about to run out isn't sent and refused on its way.
const MARGIN: SignedDuration = SignedDuration::from_secs(60);

/// How long an access token lasts when Spotify's answer doesn't say,
/// which is how long Spotify gives one.
const HOUR: u64 = 3600;

/// spotify-player's sign-in, as rspotify writes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SignIn {
    pub access_token: String,
    pub expires_in: u64,
    pub expires_at: Option<String>,
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub scope: String,
    /// Whatever else the file holds, kept as it was.
    #[serde(flatten)]
    pub rest: Map<String, Value>,
}

impl SignIn {
    /// Whether the access token has run out by `now`, or will within the
    /// margin. One with no time it runs out, or one that can't be read,
    /// counts as run out, since refreshing it costs one request and
    /// sending a spent one costs a refusal.
    pub fn expired(&self, now: Timestamp) -> bool {
        self.expires_at
            .as_deref()
            .and_then(|at| at.parse::<Timestamp>().ok())
            .and_then(|at| at.checked_sub(MARGIN).ok())
            .is_none_or(|at| at <= now)
    }

    /// The sign-in after Spotify's answer to a refresh, `answer`, given at
    /// `now`. Spotify may or may not send a new refresh token, and one it
    /// doesn't send leaves the old one standing, so the old one is kept;
    /// so are the scopes, when the answer doesn't list them.
    pub fn refreshed(&self, answer: &Value, now: Timestamp) -> Option<SignIn> {
        let access_token = answer.get("access_token")?.as_str()?.to_owned();
        let expires_in = answer
            .get("expires_in")
            .and_then(Value::as_u64)
            .unwrap_or(HOUR);
        let lasts = SignedDuration::from_secs(i64::try_from(expires_in).ok()?);
        let expires_at = now.checked_add(lasts).ok()?.to_string();
        let text = |field: &str| {
            answer
                .get(field)
                .and_then(Value::as_str)
                .filter(|said| !said.is_empty())
                .map(String::from)
        };
        Some(SignIn {
            access_token,
            expires_in,
            expires_at: Some(expires_at),
            refresh_token: text("refresh_token").or_else(|| self.refresh_token.clone()),
            scope: text("scope").unwrap_or_else(|| self.scope.clone()),
            rest: self.rest.clone(),
        })
    }
}

/// What reading the saved sign-in found.
#[derive(Debug)]
pub enum Read {
    Found(SignIn),
    /// No file, or nothing in it to sign in with: spotify-player was never
    /// signed in, or was signed out.
    Missing,
}

/// The saved sign-in at `path`.
pub fn read(path: &Path) -> Read {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<SignIn>(&text).ok())
        .filter(|token| !token.access_token.is_empty() || token.refresh_token.is_some())
        .map_or(Read::Missing, Read::Found)
}

/// Writes `token` to `path` whole or not at all: to a file beside it,
/// with the old file's permissions, then over it, so spotify-player,
/// reading it at any moment, finds either the old sign-in or the new.
pub fn write(path: &Path, token: &SignIn) -> std::io::Result<()> {
    let text = serde_json::to_string(token).map_err(std::io::Error::other)?;
    let beside = beside(path, "writing");
    let mode = fs::metadata(path).map_or(0o600, |kept| kept.permissions().mode());
    let mut file = File::create(&beside)?;
    file.set_permissions(fs::Permissions::from_mode(mode))?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    fs::rename(&beside, path)
}

/// A lock beside the sign-in, held while it's refreshed, so two of these
/// asking at once, as the launcher's three reads do, refresh it once and
/// the second reads what the first wrote. It's a file of its own, since
/// the sign-in's own file is replaced as it's written.
pub fn lock(path: &Path) -> std::io::Result<File> {
    let file = File::create(beside(path, "lock"))?;
    file.lock()?;
    Ok(file)
}

/// A file beside the sign-in, named after it.
fn beside(path: &Path, what: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{what}"));
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved() -> SignIn {
        serde_json::from_str(include_str!("../fixtures/token.json")).unwrap()
    }

    fn at(when: &str) -> Timestamp {
        when.parse().unwrap()
    }

    #[test]
    fn a_token_runs_out_a_minute_early() {
        let token = saved();
        assert!(!token.expired(at("2026-10-31T14:57:00Z")));
        assert!(token.expired(at("2026-10-31T14:58:00Z")));
        assert!(token.expired(at("2026-11-07T09:00:00Z")));
    }

    #[test]
    fn a_token_with_no_end_counts_as_run_out() {
        let mut token = saved();
        token.expires_at = None;
        assert!(token.expired(at("2026-10-31T00:00:00Z")));
        token.expires_at = Some("soon".to_owned());
        assert!(token.expired(at("2026-10-31T00:00:00Z")));
    }

    #[test]
    fn a_refresh_without_a_new_refresh_token_keeps_the_old_one() {
        let answer = serde_json::json!({
            "access_token": "new-access",
            "token_type": "Bearer",
            "expires_in": 3600
        });
        let token = saved()
            .refreshed(&answer, at("2026-11-07T09:00:00Z"))
            .unwrap();
        assert_eq!(token.access_token, "new-access");
        assert_eq!(token.refresh_token.as_deref(), Some("old-refresh"));
        assert_eq!(token.scope, saved().scope);
        assert_eq!(token.expires_at.as_deref(), Some("2026-11-07T10:00:00Z"));
        assert!(!token.expired(at("2026-11-07T09:30:00Z")));
    }

    #[test]
    fn a_refresh_with_a_new_refresh_token_takes_it() {
        let answer = serde_json::json!({
            "access_token": "new-access",
            "expires_in": 1800,
            "refresh_token": "new-refresh",
            "scope": "user-read-recently-played"
        });
        let token = saved()
            .refreshed(&answer, at("2026-11-07T09:00:00Z"))
            .unwrap();
        assert_eq!(token.refresh_token.as_deref(), Some("new-refresh"));
        assert_eq!(token.scope, "user-read-recently-played");
        assert_eq!(token.expires_in, 1800);
    }

    #[test]
    fn an_answer_without_an_access_token_refreshes_nothing() {
        let answer = serde_json::json!({ "error": "invalid_grant" });
        assert_eq!(saved().refreshed(&answer, at("2026-11-07T09:00:00Z")), None);
    }

    #[test]
    fn the_sign_in_is_written_back_in_its_own_shape() {
        let folder = std::env::temp_dir().join(format!("spotify-recent-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join("token.json");
        let given = include_str!("../fixtures/token.json").trim_end();
        fs::write(&path, given).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        let Read::Found(token) = read(&path) else {
            panic!("the fixture's sign-in wasn't read");
        };
        write(&path, &token).unwrap();
        let written = fs::read_to_string(&path).unwrap();
        // The same fields in the same order, on one line, as rspotify
        // writes them, with whatever else the file held after them.
        assert_eq!(written, given);
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o644
        );
        assert!(!beside(&path, "writing").exists());
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_missing_or_empty_sign_in_is_missing() {
        let folder = std::env::temp_dir().join(format!("spotify-none-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join("token.json");
        assert!(matches!(read(&path), Read::Missing));
        fs::write(&path, "{}").unwrap();
        assert!(matches!(read(&path), Read::Missing));
        fs::remove_dir_all(&folder).unwrap();
    }
}
