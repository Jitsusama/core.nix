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
    /// The keyring couldn't be read, though it may well hold a sign-in,
    /// and the file holds none: the sign-in last read still stands.
    Unreadable,
}

/// What the keyring's reader gave.
#[derive(Debug, PartialEq, Eq)]
enum Kept {
    /// The sign-in, as the reader printed it.
    Held(String),
    /// None, as for an item the keyring lacks: secret-tool then fails
    /// saying nothing, or a reader prints nothing.
    Lacks,
    /// The reader couldn't be run, or failed saying why, as secret-tool
    /// does when the Secret Service can't be reached.
    Unreadable,
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
/// A keyring that couldn't be read, with no file, is `Unreadable` rather
/// than no sign-in, since nothing says Joel signed out.
pub fn read(keyring: Option<&Path>, path: &Path, name: Option<&str>) -> Result<Account, Missing> {
    let kept = keyring.map_or(Kept::Lacks, kept);
    let text = match kept {
        Kept::Held(text) => text,
        Kept::Lacks => fs::read_to_string(path).map_err(|_| Missing::NoFile)?,
        Kept::Unreadable => fs::read_to_string(path).map_err(|_| Missing::Unreadable)?,
    };
    parse(&text, name)
}

/// What the program `keyring` gives. Why it couldn't be read goes to the
/// journal, as the first line of what it said, unless that line could be
/// part of a sign-in; what it prints is never told.
fn kept(keyring: &Path) -> Kept {
    let out = match Command::new(keyring)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(out) => out,
        Err(error) => {
            eprintln!("<4>the keyring's reader couldn't be run: {}", error.kind());
            return Kept::Unreadable;
        }
    };
    let said = String::from_utf8_lossy(&out.stderr);
    let why = said.lines().find(|line| !line.trim().is_empty());
    match (out.status.success(), why) {
        (true, _) => match String::from_utf8(out.stdout) {
            Ok(text) if !text.trim().is_empty() => Kept::Held(text),
            _ => Kept::Lacks,
        },
        (false, None) => Kept::Lacks,
        (false, Some(why)) => {
            eprintln!(
                "<4>the keyring couldn't be read ({}): {}",
                out.status,
                told(why)
            );
            Kept::Unreadable
        }
    }
}

/// A line the keyring's reader said, as the journal may hold it: at most
/// 120 characters, and none of it if it could be part of a sign-in.
fn told(line: &str) -> String {
    if line.contains(['{', '"']) {
        return "(what it said is kept out, as it could hold a sign-in)".to_owned();
    }
    line.trim().chars().take(120).collect()
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
        let folder = std::env::temp_dir().join(format!("google-account-{test}"));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("the test's folder is made");
        let path = folder.join("credentials.json");
        fs::write(&path, file).expect("the file is written");
        let keyring = folder.join("keyring");
        // Written by a shell of its own, never by this process: a test
        // forking beside this one would hold the reader open for writing
        // until its child ran, and running the reader meanwhile fails as a
        // text file busy, so the file is read over it (seen once on a
        // builder, 2026-11-14).
        let written = std::process::Command::new("/bin/sh")
            .args(["-c", r#"printf '%s\n' "$2" > "$1" && chmod 755 "$1""#, "sh"])
            .arg(&keyring)
            .arg(format!("#!/bin/sh\n{reader}"))
            .status()
            .expect("the reader's shell runs");
        assert!(written.success(), "the reader wasn't written");
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

    #[test]
    fn a_keyring_failing_to_say_why_is_unreadable_and_one_failing_silently_lacks_it() {
        for (test, reader, missing) in [
            (
                "failing",
                "echo 'Could not connect' >&2; exit 1",
                Missing::Unreadable,
            ),
            ("lacking", "exit 1", Missing::NoFile),
        ] {
            let (reader, path) = kept(test, FILE, reader);
            fs::remove_file(&path).expect("the file is taken away");
            assert_eq!(
                read(Some(&reader), &path, None),
                Err(missing),
                "{test}: the keyring was misread"
            );
        }
        let (_, path) = kept("unrunnable", FILE, "");
        fs::remove_file(&path).expect("the file is taken away");
        assert_eq!(
            read(Some(Path::new("/nonexistent/keyring")), &path, None),
            Err(Missing::Unreadable),
            "a keyring's reader that couldn't be run read as no sign-in"
        );
    }

    #[test]
    fn what_the_keyring_said_is_told_short_and_never_if_it_could_be_a_sign_in() {
        assert_eq!(
            told(" Could not connect "),
            "Could not connect",
            "why wasn't told"
        );
        assert_eq!(told(&"x".repeat(200)).len(), 120, "a long line wasn't cut");
        assert!(
            !told(r#"{"refresh_token": "kr""#).contains("kr"),
            "a sign-in reached the journal"
        );
    }
}
