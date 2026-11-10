//! `gcloud-broker access` prints a token for Google's APIs as the service
//! account, `gcloud-broker identity --audience A` one saying who it is to
//! the service A, and `gcloud-broker project` the project it lives in.
//! Each token is printed alone, kept for most of its hour, and minted with
//! the gcloud on PATH (or `--gcloud`) and the person's own login.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::SystemTime;

use gcloud_broker::account::Account;
use gcloud_broker::gcloud::{self, Ask, PATIENCE};
use gcloud_broker::held::Held;

const USAGE: &str = "usage: gcloud-broker access --account EMAIL [--gcloud PROGRAM]\n       \
                     gcloud-broker identity --audience AUDIENCE --account EMAIL \
                     [--gcloud PROGRAM]\n       gcloud-broker project --account EMAIL";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(said) => {
            println!("{said}");
            ExitCode::SUCCESS
        }
        Err(Refusal::Usage(why)) => {
            eprintln!("gcloud-broker: {why}\n{USAGE}");
            ExitCode::from(2)
        }
        Err(Refusal::Failed(why)) => {
            eprintln!("gcloud-broker: {why}");
            ExitCode::FAILURE
        }
    }
}

/// Why nothing was printed: asked wrongly, or asked rightly and refused.
enum Refusal {
    Usage(String),
    Failed(String),
}

fn run(args: &[String]) -> Result<String, Refusal> {
    let account =
        named(args, "--account")?.ok_or_else(|| Refusal::Usage("no --account given".to_owned()))?;
    let account = Account::parse(&account).map_err(Refusal::Failed)?;
    let audience = named(args, "--audience")?;
    let ask = match (args.first().map(String::as_str), audience.as_deref()) {
        (Some("project"), None) => return Ok(account.project().to_owned()),
        (Some("access"), None) => Ask::Access,
        (Some("identity"), Some(audience)) if gcloud::is_audience(audience) => {
            Ask::Identity(audience)
        }
        (Some("identity"), Some(audience)) => {
            return Err(Refusal::Failed(format!(
                "{audience:?} isn't an audience, a service's address"
            )));
        }
        (Some("identity"), None) => {
            return Err(Refusal::Usage("identity needs --audience".to_owned()));
        }
        (Some(verb @ ("project" | "access")), Some(_)) => {
            return Err(Refusal::Usage(format!("{verb} takes no --audience")));
        }
        _ => return Err(Refusal::Usage("ask access, identity or project".to_owned())),
    };
    let gcloud = named(args, "--gcloud")?.map_or_else(|| OsString::from("gcloud"), OsString::from);
    // Without a runtime folder nothing is kept, and each ask mints anew.
    let held = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|folder| !folder.is_empty())
        .map(|folder| Held::under(&PathBuf::from(folder)));
    let now = SystemTime::now();
    if let Some(token) = held
        .as_ref()
        .and_then(|held| held.token(ask, &account, now))
    {
        return Ok(token);
    }
    let token = gcloud::mint(&gcloud, ask, &account, PATIENCE).map_err(Refusal::Failed)?;
    if let Some(held) = &held
        && let Err(why) = held.keep(ask, &account, &token, now)
    {
        // The token is still good; the next ask only mints another.
        eprintln!("gcloud-broker: {why}");
    }
    Ok(token)
}

/// The value given after `name`, refusing one given twice, so an account
/// named by whoever installed the broker can't be quietly overridden.
fn named(args: &[String], name: &str) -> Result<Option<String>, Refusal> {
    let mut found = args
        .iter()
        .enumerate()
        .filter(|(_, arg)| *arg == name)
        .map(|(at, _)| args.get(at + 1).cloned());
    match (found.next(), found.next()) {
        (None, _) => Ok(None),
        (Some(Some(value)), None) => Ok(Some(value)),
        (Some(None), None) => Err(Refusal::Usage(format!("{name} needs a value"))),
        (Some(_), Some(_)) => Err(Refusal::Usage(format!("{name} is given twice"))),
    }
}
