//! The jobs broker: what a fenced pi command asks of the account's
//! systemd, which the fence keeps it from reaching.
//!
//! The agent fence (the harness's lib/internal/fence.ts) hides the session
//! bus from every command pi runs, so nothing it runs can start, stop or
//! reconfigure what Joel runs. That took with it the two things agents'
//! commands asked the account's systemd for: a scope of their own around
//! a command, as nix-offload's uploads and the systemd-run stand-in ask
//! with `--slice-inherit`, and how a unit stands, as `systemctl --user
//! status` says. The broker answers just those, on a socket the fence lets
//! through, `%t/daemons/jobs-broker.sock`:
//!
//! - `scope` moves the process asking, known by the pidfd the kernel gives
//!   for the connection and never by a pid it names, into a scope of its
//!   own in agents' work, `pi-jobs-<quest>-<pid>.scope` in its quest's
//!   slice under `jobs-work.slice`, and only when it already runs as an
//!   agent's (src/place.rs). It sets no property the process asks for.
//! - `status` says a fixed set of each named unit's properties, none of
//!   which holds a command line or an environment (src/systemd.rs).
//!
//! It starts, stops and changes no unit but the scopes it makes. The
//! journal reads without the bus, so `journalctl --user` needs nothing
//! here.
//!
//! `jobs-broker run [--quest Q] -- COMMAND...` asks for a scope, then
//! becomes the command in it; `jobs-broker status UNIT...` prints each
//! unit's standing as `systemctl show` does.

mod answer;
mod peer;
mod place;
mod systemd;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::ExitCode;

use daemon_serve::asks::Domain;
use daemon_serve::serve::{Shared, accept, listen};
use serde_json::{Value, json};

/// The broker as its clients know it: a domain holding no records, only
/// its own two asks.
const DOMAIN: Domain = Domain {
    name: "jobs-broker",
    kinds: &[],
    sole: &[],
    named_only: &[],
};

/// Where the broker listens, below the runtime folder.
const SOCKET: &str = "daemons/jobs-broker.sock";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("serve") => serve(&args[1..]),
        Some("run") => run(&args[1..]),
        Some("status") => status(&args[1..]),
        _ => {
            eprintln!("usage: jobs-broker run [--quest QUEST] -- COMMAND...");
            eprintln!("       jobs-broker status UNIT...");
            eprintln!("       jobs-broker serve [--socket PATH]");
            ExitCode::from(2)
        }
    }
}

/// `--socket PATH` when given, else the runtime folder's.
fn socket_in(args: &[String]) -> Option<PathBuf> {
    args.iter()
        .position(|arg| arg == "--socket")
        .and_then(|at| args.get(at + 1))
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("XDG_RUNTIME_DIR").map(|runtime| PathBuf::from(runtime).join(SOCKET))
        })
}

fn serve(args: &[String]) -> ExitCode {
    let Some(socket) = socket_in(args) else {
        eprintln!("jobs-broker: no --socket and no XDG_RUNTIME_DIR to listen in");
        return ExitCode::FAILURE;
    };
    let shared = Shared::new(
        DOMAIN,
        Box::new(|said| (said["ask"] == "status").then(|| answer::status(said))),
    );
    shared.peered(
        &["scope"],
        Box::new(|said, stream| Some(answer::scope(said, stream))),
    );
    // Each waits on systemd, so none holds up the next ask on its socket.
    shared.slow(&["scope", "status"]);
    shared.change(Vec::new(), json!({"systemd": "answering"}));
    let listener = match listen(&socket) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("jobs-broker: can't listen on {}: {error}", socket.display());
            return ExitCode::FAILURE;
        }
    };
    accept(shared, listener);
    loop {
        std::thread::park();
    }
}

/// One ask of the broker, and its answer.
fn ask(socket: Option<PathBuf>, asked: &Value) -> Result<Value, String> {
    let socket = socket.ok_or("no XDG_RUNTIME_DIR to find the broker in")?;
    let stream = UnixStream::connect(&socket)
        .map_err(|error| format!("can't reach the broker at {}: {error}", socket.display()))?;
    let mut writing = stream.try_clone().map_err(|error| error.to_string())?;
    writeln!(writing, "{asked}").map_err(|error| error.to_string())?;
    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|error| error.to_string())?;
    let answer: Value = serde_json::from_str(&line).map_err(|_| "the broker hung up".to_owned())?;
    match answer.get("error") {
        Some(error) => Err(error["message"].as_str().unwrap_or("refused").to_owned()),
        None => Ok(answer),
    }
}

/// The quest pi names the session's, as its slice is named: lowercase,
/// its dashes gone.
fn quest_of_session() -> Option<String> {
    let quest: String = std::env::var("PI_QUEST")
        .ok()?
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|letter| letter.to_ascii_lowercase())
        .collect();
    place::is_quest(&quest).then_some(quest)
}

fn run(args: &[String]) -> ExitCode {
    let Some(split) = args.iter().position(|arg| arg == "--") else {
        eprintln!("usage: jobs-broker run [--quest QUEST] -- COMMAND...");
        return ExitCode::from(2);
    };
    let (options, command) = (&args[..split], &args[split + 1..]);
    let Some((program, rest)) = command.split_first() else {
        eprintln!("jobs-broker: run needs a command after --");
        return ExitCode::from(2);
    };
    let quest = options
        .iter()
        .position(|arg| arg == "--quest")
        .and_then(|at| options.get(at + 1).cloned())
        .or_else(quest_of_session);
    if let Err(why) = ask(
        socket_in(options),
        &json!({"id": 1, "ask": "scope", "quest": quest}),
    ) {
        // Run nowhere else: a command asked for in a scope of its own is
        // never run outside one, as systemd-run doesn't.
        eprintln!("jobs-broker: no scope for {program}: {why}");
        return ExitCode::FAILURE;
    }
    let error = std::process::Command::new(program).args(rest).exec();
    eprintln!("jobs-broker: can't run {program}: {error}");
    ExitCode::from(127)
}

fn status(units: &[String]) -> ExitCode {
    if units.is_empty() {
        eprintln!("usage: jobs-broker status UNIT...");
        return ExitCode::from(2);
    }
    let answer = match ask(
        socket_in(&[]),
        &json!({"id": 1, "ask": "status", "units": units}),
    ) {
        Ok(answer) => answer,
        Err(why) => {
            eprintln!("jobs-broker: {why}");
            return ExitCode::FAILURE;
        }
    };
    let mut failed = false;
    let told = answer["units"].as_array().cloned().unwrap_or_default();
    for (at, unit) in told.iter().enumerate() {
        if at > 0 {
            println!();
        }
        let name = unit["unit"].as_str().unwrap_or("?");
        if let Some(why) = unit["error"].as_str() {
            eprintln!("{name}: {why}");
            failed = true;
            continue;
        }
        for (key, value) in unit["status"].as_object().into_iter().flatten() {
            match value {
                Value::String(text) => println!("{key}={text}"),
                other => println!("{key}={other}"),
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
