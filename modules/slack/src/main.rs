//! slack-people CREDENTIALS [ORIGIN]
//!
//! Who Slack knows by an email address, for People's Slack source
//! (SlackSource.qml): the shell writes an address a line on standard
//! input, as Joel steps into someone, and hears one JSON line back for
//! each (lib.rs's `Found`): their id in Slack and its workspace, which
//! open them there, their name and their time zone, or no id for an
//! address Slack knows nobody by. Each address is asked about once while it runs, and once
//! Slack refuses the sign-in, the line `{"signedOut":true}` is said and
//! nobody more is asked about. With no sign-in in CREDENTIALS, the
//! harness's file, it says nothing and asks nothing. ORIGIN is Slack's,
//! unless a test names a stand-in.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::PathBuf;

use slack_people::{Account, Refused, Slack, asked};

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
        let said = match known.get(&email) {
            Some(line) => Some(String::clone(line)),
            None => match slack.look_up(&account, &email) {
                Ok(found) => {
                    let line = serde_json::to_string(&found).unwrap_or_default();
                    known.insert(email, line.clone());
                    Some(line)
                }
                Err(Refused::SignedOut) => {
                    signed_out = true;
                    Some(r#"{"signedOut":true}"#.to_owned())
                }
                Err(Refused::Unreachable(why)) => {
                    eprintln!("<4>{why}");
                    None
                }
            },
        };
        if let Some(said) = said {
            let mut out = stdout.lock();
            if writeln!(out, "{said}").and_then(|()| out.flush()).is_err() {
                break;
            }
        }
    }
}
