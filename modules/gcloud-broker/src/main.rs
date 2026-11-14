//! `gcloud-broker access` prints a token for Google's APIs as the service
//! account, `gcloud-broker identity --audience A` one saying who it is to
//! the service A, and `gcloud-broker project` the project it lives in.
//! Each token is printed alone, kept for most of its hour, and minted with
//! the gcloud on PATH (or `--gcloud`) and the person's own login.
//!
//! It takes the shape every command of ours does (docs/commands.md): its
//! help, usage that says what to try, and failures with named exits.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::SystemTime;

use gcloud_broker::account::Account;
use gcloud_broker::gcloud::{self, Ask, PATIENCE};
use gcloud_broker::held::Held;
use serde_json::{Value, json};
use terminal_kit::{Context, Exit, Failure, Help, flags};

/// What it's asked for, by the first word.
const COMMANDS: [&str; 3] = ["access", "identity", "project"];

/// The named flags it reads, every one taking a value.
const FLAGS: [&str; 3] = ["--account", "--audience", "--gcloud"];

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "gcloud-broker",
        "mints a service account's hour-long Google Cloud tokens with the person's login",
    )
    .subcommand(
        mints("access", "prints a token for Google's APIs as the account").example(
            "gcloud-broker access",
            "a token, for CLOUDSDK_AUTH_ACCESS_TOKEN",
        ),
    )
    .subcommand(
        mints("identity", "prints an identity token for one service")
            .example(
                "gcloud-broker identity --audience URL",
                "a token URL's service takes",
            )
            .option("--audience URL", "the service the token is for")
            .exit(Exit::BadInput, "the audience isn't one Google names"),
    )
    .subcommand(
        mints("project", "prints the project the account lives in")
            .example("gcloud-broker project --json", "the project, as JSON"),
    )
    .option(
        "--account EMAIL",
        "the service account; the wrapper names it",
    )
    .option("--gcloud PROGRAM", "the gcloud to mint with, if not PATH's")
    .example(
        "gcloud-broker access",
        "a token, for CLOUDSDK_AUTH_ACCESS_TOKEN",
    )
    .example(
        "gcloud-broker project --json",
        "the account's project, as JSON",
    )
    .exit(
        Exit::Failed,
        "gcloud refused, or said something that isn't a token",
    )
    .exit(
        Exit::BadInput,
        "the account or the audience isn't one Google names",
    )
    .exit(
        Exit::Unavailable,
        "gcloud couldn't be run, or didn't answer in time",
    )
}

/// A subcommand that asks gcloud as the account.
fn mints(name: &str, about: &str) -> Help {
    Help::new(name, about)
        .option(
            "--account EMAIL",
            "the service account; the wrapper names it",
        )
        .option("--gcloud PROGRAM", "the gcloud to mint with, if not PATH's")
        .exit(Exit::Failed, "gcloud refused, or gave no token")
        .exit(Exit::BadInput, "the account isn't one Google names")
        .exit(Exit::Unavailable, "gcloud couldn't be run in time")
}

/// The value each named flag was given, in `FLAGS`' order. A flag given
/// twice is refused, so an account named by whoever installed the broker
/// can't be quietly overridden.
fn named(args: &[String]) -> Result<(Option<&str>, [Option<String>; 3]), Failure> {
    let mut given: [Option<String>; 3] = Default::default();
    let mut command = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if !arg.starts_with('-') && command.is_none() {
            command = Some(arg.as_str());
            continue;
        }
        let Some(at) = FLAGS.iter().position(|name| name == arg) else {
            return Err(flags::unknown("flag", arg, &FLAGS));
        };
        let value = args.next().ok_or_else(|| {
            Failure::new(Exit::Usage, format!("{arg} needs a value"))
                .try_this("gcloud-broker --help")
        })?;
        if given[at].replace(value.clone()).is_some() {
            return Err(Failure::new(Exit::Usage, format!("{arg} is given twice"))
                .because("the account is the wrapper's to name, once")
                .try_this(format!("gcloud-broker {}", command.unwrap_or("access"))));
        }
    }
    Ok((command, given))
}

/// What a command line asks for, read: a token to mint, or the project.
enum Asked<'a> {
    Token(Ask<'a>),
    Project,
}

fn asked<'a>(command: Option<&str>, audience: Option<&'a str>) -> Result<Asked<'a>, Failure> {
    match (command, audience) {
        (Some("project"), None) => Ok(Asked::Project),
        (Some("access"), None) => Ok(Asked::Token(Ask::Access)),
        (Some("identity"), Some(audience)) if gcloud::is_audience(audience) => {
            Ok(Asked::Token(Ask::Identity(audience)))
        }
        (Some("identity"), Some(audience)) => Err(Failure::new(
            Exit::BadInput,
            format!("{audience:?} isn't an audience"),
        )
        .because("an audience is a service's address")
        .try_this("gcloud-broker identity --audience https://a-service.run.app")),
        (Some("identity"), None) => Err(Failure::new(Exit::Usage, "identity needs --audience")
            .try_this("gcloud-broker identity --audience https://a-service.run.app")),
        (Some(verb @ ("project" | "access")), Some(_)) => Err(Failure::new(
            Exit::Usage,
            format!("{verb} takes no --audience"),
        )
        .try_this(format!("gcloud-broker {verb}"))),
        (Some(other), _) => Err(flags::unknown("command", other, &COMMANDS)),
        (None, _) => Err(
            Failure::new(Exit::Usage, "it takes access, identity or project")
                .try_this("gcloud-broker access"),
        ),
    }
}

/// A token minted as the account, or the one kept from a mint before.
fn token(
    context: &Context,
    ask: Ask<'_>,
    account: &Account,
    gcloud: &OsString,
) -> Result<String, Failure> {
    // Without a runtime folder nothing is kept, and each ask mints anew.
    let held = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|folder| !folder.is_empty())
        .map(|folder| Held::under(&PathBuf::from(folder)));
    let now = SystemTime::now();
    if let Some(token) = held.as_ref().and_then(|held| held.token(ask, account, now)) {
        return Ok(token);
    }
    let token = gcloud::mint(gcloud, ask, account, PATIENCE).map_err(|why| {
        let exit = if why.starts_with("can't run") || why.contains("didn't answer") {
            Exit::Unavailable
        } else {
            Exit::Failed
        };
        Failure::new(exit, "gcloud gave no token")
            .because(why)
            .try_this("gcloud auth login")
    })?;
    if let Some(held) = &held
        && let Err(why) = held.keep(ask, account, &token, now)
    {
        // The token is still good; the next ask only mints another.
        let _ = context.tell(
            "caution",
            "warning",
            &format!("it couldn't keep the token: {why}"),
        );
    }
    Ok(token)
}

fn run(context: &Context) -> Result<(), Failure> {
    let (command, [account, audience, gcloud]) = named(&context.args)?;
    let asked = asked(command, audience.as_deref())?;
    let account = account.ok_or_else(|| {
        Failure::new(Exit::Usage, "no --account given")
            .because("the wrapper on PATH names it, from jitsusama.gcloud-broker")
            .try_this("gcloud-broker access")
    })?;
    let account = Account::parse(&account).map_err(|why| {
        Failure::new(Exit::BadInput, "the account isn't a service account").because(why)
    })?;
    let (text, value): (String, Value) = match asked {
        Asked::Project => (
            account.project().to_owned(),
            json!({"schema": "gcloud-broker.project/1", "project": account.project()}),
        ),
        Asked::Token(ask) => {
            let gcloud = gcloud.map_or_else(|| OsString::from("gcloud"), OsString::from);
            let minted = token(context, ask, &account, &gcloud)?;
            let (kind, audience) = match ask {
                Ask::Access => ("access", None),
                Ask::Identity(audience) => ("identity", Some(audience)),
            };
            let value = json!({
                "schema": "gcloud-broker.token/1",
                "kind": kind,
                "audience": audience,
                "account": account.email(),
                "token": minted,
            });
            (minted, value)
        }
    };
    let said = if context.format().is_json() {
        context.answer(&value)
    } else {
        context.say(&format!("{text}\n"))
    };
    said.map_err(|why| Failure::new(Exit::Failed, "it couldn't print it").because(why.to_string()))
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), run)
}
