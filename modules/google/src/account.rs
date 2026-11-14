//! The Google account, as the harness keeps it.
//!
//! The harness signs Joel in (pi's `/google-auth`) and keeps the OAuth
//! app, its accounts and each account's tokens as one JSON document, in the
//! desktop's keyring, or in a file as pi kept it before it moved sign-ins
//! there. This program only reads it: pi rewrites it each time it refreshes
//! a token, so a second writer could lose one of pi's writes or leave it
//! half written for both. A token this program refreshes is kept in memory
//! instead (token.rs).

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use serde::Deserialize;

/// What the file says of one account: enough to ask Google as it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    pub client_id: String,
    pub client_secret: String,
    pub refresh_token: String,
    /// The access token pi last held, good until `expires` (milliseconds
    /// since the epoch), which saves a refresh while pi's is still fresh.
    pub access_token: String,
    pub expires: i64,
    /// What the sign-in lets this program do, as Google's scopes, which
    /// signing in again from pi may widen.
    pub scopes: Vec<String>,
}

impl Account {
    /// Whether the sign-in lets this program ask Google `scope`.
    pub fn grants(&self, scope: &str) -> bool {
        self.scopes.iter().any(|granted| granted == scope)
    }
}

/// Why there's no account to ask as. Neither is an error to say: a
/// machine nobody signed in to Google on simply has no Google.
#[derive(Debug, PartialEq, Eq)]
pub enum Missing {
    /// No file, or one that isn't the harness's JSON.
    NoFile,
    /// A file without the OAuth app, the account or its refresh token.
    NotSignedIn,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct File {
    oauth_app: Option<App>,
    #[serde(default)]
    accounts: Vec<Named>,
    #[serde(default)]
    tokens: std::collections::HashMap<String, Tokens>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct App {
    client_id: String,
    client_secret: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Named {
    name: String,
    #[serde(default)]
    is_default: bool,
}

#[derive(Deserialize)]
struct Tokens {
    #[serde(default)]
    access_token: String,
    #[serde(default)]
    refresh_token: String,
    #[serde(default)]
    expiry_date: i64,
    /// The scopes granted, apart by spaces, as OAuth writes them.
    #[serde(default)]
    scope: String,
}

/// The account `name` names, else the sign-in's default, else its first,
/// as the harness itself picks one. The sign-in is the keyring's, printed
/// by the program `keyring`, while it has one; else the file at `path`.
pub fn read(keyring: Option<&Path>, path: &Path, name: Option<&str>) -> Result<Account, Missing> {
    let text = keyring
        .and_then(kept)
        .or_else(|| fs::read_to_string(path).ok())
        .ok_or(Missing::NoFile)?;
    parse(&text, name)
}

/// The sign-in the program `keyring` prints, or none when it can't be run,
/// fails, as it does for an item the keyring lacks, or prints nothing. What
/// it says otherwise is let go, so no part of a sign-in reaches the journal.
fn kept(keyring: &Path) -> Option<String> {
    let out = Command::new(keyring)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    (out.status.success() && !text.trim().is_empty()).then_some(text)
}

fn parse(text: &str, name: Option<&str>) -> Result<Account, Missing> {
    let file: File = serde_json::from_str(text).map_err(|_| Missing::NoFile)?;
    let app = file.oauth_app.ok_or(Missing::NotSignedIn)?;
    let chosen = match name {
        Some(name) => name.to_owned(),
        None => file
            .accounts
            .iter()
            .find(|account| account.is_default)
            .or(file.accounts.first())
            .map(|account| account.name.clone())
            .ok_or(Missing::NotSignedIn)?,
    };
    let tokens = file.tokens.get(&chosen).ok_or(Missing::NotSignedIn)?;
    if tokens.refresh_token.is_empty() || app.client_id.is_empty() {
        return Err(Missing::NotSignedIn);
    }
    Ok(Account {
        client_id: app.client_id,
        client_secret: app.client_secret,
        refresh_token: tokens.refresh_token.clone(),
        access_token: tokens.access_token.clone(),
        expires: tokens.expiry_date,
        scopes: tokens.scope.split_whitespace().map(str::to_owned).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r#"{
        "oauthApp": {"clientId": "app", "clientSecret": "secret"},
        "accounts": [{"name": "home"}, {"name": "work", "isDefault": true}],
        "tokens": {
            "home": {"access_token": "h", "refresh_token": "hr", "expiry_date": 1},
            "work": {"access_token": "w", "refresh_token": "wr", "expiry_date": 2,
                     "token_type": "Bearer", "scope": "calendar"}
        }
    }"#;

    #[test]
    fn the_default_account_is_read_when_none_is_named() {
        let account = parse(FILE, None).unwrap();
        assert_eq!(account.refresh_token, "wr");
        assert_eq!(account.access_token, "w");
        assert_eq!(account.expires, 2);
        assert_eq!(account.client_id, "app");
    }

    #[test]
    fn an_account_grants_only_the_scopes_it_was_signed_in_with() {
        let file = FILE.replace(
            r#""scope": "calendar""#,
            r#""scope": "calendar https://www.googleapis.com/auth/meetings.space.created""#,
        );
        let account = parse(&file, None).unwrap();
        assert!(account.grants("https://www.googleapis.com/auth/meetings.space.created"));
        assert!(account.grants("calendar"));
        assert!(!account.grants("meetings"), "a part of a scope was granted");
        assert!(!parse(FILE, Some("home")).unwrap().grants("calendar"));
    }

    #[test]
    fn a_named_account_is_read_over_the_default() {
        assert_eq!(parse(FILE, Some("home")).unwrap().refresh_token, "hr");
    }

    #[test]
    fn the_first_account_stands_in_for_a_default() {
        let file = FILE.replace(r#", "isDefault": true"#, "");
        assert_eq!(parse(&file, None).unwrap().refresh_token, "hr");
    }

    #[test]
    fn no_app_or_no_tokens_is_not_signed_in() {
        let file = r#"{"accounts": [], "tokens": {}}"#;
        assert_eq!(parse(file, None), Err(Missing::NotSignedIn));
        assert_eq!(parse(FILE, Some("elsewhere")), Err(Missing::NotSignedIn));
        let empty = FILE.replace(r#""refresh_token": "wr""#, r#""refresh_token": """#);
        assert_eq!(parse(&empty, None), Err(Missing::NotSignedIn));
    }

    #[test]
    fn a_missing_or_broken_file_is_no_file() {
        assert_eq!(
            read(None, Path::new("/nonexistent/credentials.json"), None),
            Err(Missing::NoFile)
        );
        assert_eq!(parse("{", None), Err(Missing::NoFile));
    }

    /// A folder of the test's own, holding the file with `file`'s sign-in
    /// and a keyring reader that runs `reader`.
    fn kept(test: &str, file: &str, reader: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        use std::os::unix::fs::PermissionsExt;
        let folder = std::env::temp_dir().join(format!("google-account-{test}"));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("the test's folder is made");
        let path = folder.join("credentials.json");
        fs::write(&path, file).expect("the file is written");
        let keyring = folder.join("keyring");
        fs::write(&keyring, format!("#!/bin/sh\n{reader}\n")).expect("the reader is written");
        fs::set_permissions(&keyring, fs::Permissions::from_mode(0o755)).expect("the reader runs");
        (keyring, path)
    }

    #[test]
    fn the_keyrings_sign_in_is_read_over_the_file() {
        let keyring = FILE.replace(r#""refresh_token": "wr""#, r#""refresh_token": "kr""#);
        let (reader, path) = kept("keyring", FILE, &format!("cat <<'END'\n{keyring}\nEND"));
        let account = read(Some(&reader), &path, None).expect("the keyring's sign-in is read");
        assert_eq!(
            account.refresh_token, "kr",
            "the file was read over the keyring"
        );
    }

    #[test]
    fn the_file_is_read_while_the_keyring_holds_no_sign_in() {
        for (test, reader) in [("refused", "exit 1"), ("empty", "exit 0")] {
            let (reader, path) = kept(test, FILE, reader);
            let account = read(Some(&reader), &path, None).expect("the file's sign-in is read");
            assert_eq!(account.refresh_token, "wr", "{test}: the file wasn't read");
        }
        let (_, path) = kept("unreadable", FILE, "");
        let nowhere = Path::new("/nonexistent/keyring");
        assert_eq!(
            read(Some(nowhere), &path, None).map(|account| account.refresh_token),
            Ok("wr".to_owned()),
            "a keyring that can't be read hid the file"
        );
    }
}
