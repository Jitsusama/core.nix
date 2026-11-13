//! `github-broker serve` holds the App and answers its socket;
//! `github-broker token` asks it, and prints the token alone, so
//! `GH_TOKEN=$(github-broker)` takes it whole.
//!
//! It takes the shape every command of ours does (docs/commands.md): its
//! help, usage that says what to try, and failures with named exits.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use github_broker::app::App;
use github_broker::mint::Minter;
use github_broker::wire;
use serde_json::json;
use terminal_kit::{Context, Exit, Failure, Help, flags};

/// What it does, by the first word.
const COMMANDS: [&str; 2] = ["serve", "token"];

/// The named flags each command reads, every one taking a value.
const SERVE_FLAGS: [&str; 5] = ["--app", "--installation", "--identity", "--socket", "--api"];
const TOKEN_FLAGS: [&str; 1] = ["--socket"];

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "github-broker",
        "hands a GitHub App's hour-long installation tokens to its socket's callers",
    )
    .command("serve", "holds the App and answers the socket")
    .command("token", "asks the socket and prints the token alone")
    .option("--socket PATH", "the socket to answer or ask")
    .option("--app FILE", "serve: the App, sealed with age")
    .option(
        "--installation FILE",
        "serve: the file holding its installation's id",
    )
    .option(
        "--identity FILE",
        "serve: the age identity that opens the App",
    )
    .option("--api URL", "serve: GitHub's API, if not api.github.com")
    .example("github-broker", "a token, for GH_TOKEN=$(github-broker) gh")
    .example(
        "github-broker --json",
        "the token and when it lapses, as JSON",
    )
    .exit(
        Exit::Failed,
        "GitHub refused, or the App couldn't be opened",
    )
    .exit(Exit::Unavailable, "no broker answers at the socket")
}

/// The value each named flag was given, in `known`'s order, or a usage
/// failure for a flag it doesn't read or one left without a value.
fn named(args: &[String], known: &[&str]) -> Result<Vec<Option<String>>, Failure> {
    let mut given = vec![None; known.len()];
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let Some(at) = known.iter().position(|name| name == arg) else {
            return Err(flags::unknown("flag", arg, known));
        };
        let value = args.next().ok_or_else(|| {
            Failure::new(Exit::Usage, format!("{arg} needs a value"))
                .try_this("github-broker --help")
        })?;
        given[at] = Some(value.clone());
    }
    Ok(given)
}

/// The usage failure for a flag a command can't go without.
fn missing(command: &str, flag: &str) -> Failure {
    Failure::new(Exit::Usage, format!("{command} needs {flag}")).try_this("github-broker --help")
}

fn serve(args: &[String]) -> Result<(), Failure> {
    let given = named(args, &SERVE_FLAGS)?;
    let take = |at: usize| {
        given[at]
            .clone()
            .ok_or_else(|| missing("serve", SERVE_FLAGS[at]))
    };
    let (app, installation, identity, socket) = (take(0)?, take(1)?, take(2)?, take(3)?);
    let api = given[4]
        .clone()
        .unwrap_or_else(|| "https://api.github.com".to_owned());
    let listener = wire::listener(Path::new(&socket)).map_err(|why| {
        Failure::new(Exit::Failed, "it couldn't listen")
            .because(why)
            .try_this(format!(
                "check that {socket}'s folder is there and writable"
            ))
    })?;
    let mut minter = Minter::new(
        &api,
        Box::new(move || {
            App::open(
                &PathBuf::from(&app),
                &PathBuf::from(&installation),
                &PathBuf::from(&identity),
            )
        }),
    );
    wire::serve(&listener, &mut minter);
    Ok(())
}

fn token(context: &Context, args: &[String]) -> Result<(), Failure> {
    let socket = named(args, &TOKEN_FLAGS)?
        .remove(0)
        .ok_or_else(|| missing("token", "--socket"))?;
    let token = wire::ask(Path::new(&socket)).map_err(|why| {
        if why.starts_with("no broker answers") {
            Failure::new(Exit::Unavailable, "no broker answers")
                .because(why)
                .try_this("systemctl --user start github-broker.socket")
        } else {
            Failure::new(Exit::Failed, "the broker gave no token")
                .because(why)
                .try_this("journalctl --user -u github-broker")
        }
    })?;
    let said = if context.format().is_json() {
        context.answer(&json!({
            "schema": "github-broker.token/1",
            "token": token.value,
            "expires_at": token.expires_at,
        }))
    } else {
        context.say(&format!("{}\n", token.value))
    };
    said.map_err(|why| {
        Failure::new(Exit::Failed, "it couldn't print the token").because(why.to_string())
    })
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| match context
        .args
        .split_first()
    {
        Some((command, rest)) if command == "serve" => serve(rest),
        Some((command, rest)) if command == "token" => token(context, rest),
        Some((command, _)) => Err(flags::unknown("command", command, &COMMANDS)),
        None => Err(Failure::new(Exit::Usage, "it takes serve or token")
            .try_this("github-broker token --socket \"$XDG_RUNTIME_DIR/github-broker.sock\"")),
    })
}
