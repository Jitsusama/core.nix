//! slack-people CREDENTIALS [ORIGIN]
//!
//! Who Slack knows by an email address, for People's Slack source
//! (SlackSource.qml): the shell writes an address a line on standard
//! input, as Joel steps into someone, and hears one JSON line back for
//! each (lib.rs's `Found`): their id in Slack and its workspace, which
//! open them there, their name, their time zone, whether they're active,
//! away or in a huddle, and their status, or no id for an address Slack
//! knows nobody by. Who an address is is found once while it runs; how
//! they are is asked each time the shell asks, which is as Joel steps
//! into them, and when Slack can't be reached then, what it said last is
//! said again. Once Slack refuses the sign-in, the line
//! `{"signedOut":true}` is said and nobody more is asked about. With no sign-in in CREDENTIALS, the
//! harness's file, it says nothing and asks nothing. ORIGIN is Slack's,
//! unless a test names a stand-in.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::PathBuf;

use slack_people::{Account, Found, Refused, Slack, asked};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(credentials) = args.next().map(PathBuf::from) else {
        eprintln!("usage: slack-people CREDENTIALS [ORIGIN]");
        std::process::exit(64);
    };
    let slack = Slack::new(args.next().unwrap_or_else(|| "https://slack.com".into()));
    let mut known = HashMap::new();
    let mut signed_out = false;
    let stdout = std::io::stdout();
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let email = line.trim().to_lowercase();
        if signed_out || !asked(&email) {
            continue;
        }
        // The file is read for each address, so signing in again from pi
        // is taken without a restart.
        let Some(account) = Account::read(&credentials) else {
            continue;
        };
        let had: Option<&Found> = known.get(&email);
        let now = match had {
            Some(had) if had.id.is_none() => Ok(had.clone()),
            Some(had) => slack.again(&account, had),
            None => slack.look_up(&account, &email),
        }
        .and_then(|found| present(&slack, &account, found));
        let said = match now {
            Ok(found) => {
                let line = serde_json::to_string(&found).unwrap_or_default();
                known.insert(email, found);
                Some(line)
            }
            Err(Refused::SignedOut) => {
                signed_out = true;
                Some(r#"{"signedOut":true}"#.to_owned())
            }
            Err(Refused::Unreachable(why)) => {
                eprintln!("<4>{why}");
                had.and_then(|had| serde_json::to_string(had).ok())
            }
        };
        if let Some(said) = said {
            let mut out = stdout.lock();
            if writeln!(out, "{said}").and_then(|()| out.flush()).is_err() {
                break;
            }
        }
    }
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
