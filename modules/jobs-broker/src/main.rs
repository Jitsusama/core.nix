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
mod cli;
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
use terminal_kit::{Exit, Failure, flags};

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
    // The kit has walked the words to a command by now, so the first is
    // always one of the three.
    terminal_kit::main(
        &cli::help(),
        env!("CARGO_PKG_VERSION"),
        |context| match context.args.split_first() {
            Some((first, rest)) if first == "run" => run(rest),
            Some((first, rest)) if first == "status" => status(rest),
            Some((first, rest)) if first == "serve" => serve(rest),
            _ => Err(cli::wrongly("jobs-broker", "jobs-broker needs a command")),
        },
    )
}

/// The flags before a command's words, each with its value, as `command`
/// takes those in `known`.
#[derive(Default)]
struct Options {
    quest: Option<String>,
    socket: Option<PathBuf>,
}

fn options(command: &str, args: &[String], known: &[&str]) -> Result<Options, Failure> {
    let mut options = Options::default();
    let mut words = args.iter();
    while let Some(flag) = words.next() {
        if !known.contains(&flag.as_str()) {
            return Err(flags::unknown("flag", flag, known).try_this(format!("{command} --help")));
        }
        let Some(value) = words.next() else {
            return Err(cli::wrongly(command, format!("{flag} needs a value")));
        };
        if flag == "--quest" {
            options.quest = Some(value.clone());
        } else {
            options.socket = Some(PathBuf::from(value));
        }
    }
    Ok(options)
}

/// `--socket PATH` when given, else the runtime folder's.
fn socket_in(given: Option<PathBuf>) -> Option<PathBuf> {
    given.or_else(|| {
        std::env::var_os("XDG_RUNTIME_DIR").map(|runtime| PathBuf::from(runtime).join(SOCKET))
    })
}

fn serve(args: &[String]) -> Result<(), Failure> {
    let given = options("jobs-broker serve", args, &cli::FLAGS[1..])?;
    let Some(socket) = socket_in(given.socket) else {
        return Err(Failure::new(Exit::Failed, "jobs-broker couldn't listen")
            .because("there's no --socket, and no XDG_RUNTIME_DIR to listen in"));
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
            return Err(Failure::new(
                Exit::Failed,
                format!("jobs-broker couldn't listen on {}", socket.display()),
            )
            .because(error.to_string()));
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

fn run(args: &[String]) -> Result<(), Failure> {
    const RUN: &str = "jobs-broker run";
    let Some(split) = args.iter().position(|arg| arg == "--") else {
        return Err(cli::wrongly(RUN, "run takes its command after --"));
    };
    let (flags, command) = (&args[..split], &args[split + 1..]);
    let Some((program, rest)) = command.split_first() else {
        return Err(cli::wrongly(RUN, "run needs a COMMAND after --"));
    };
    let given = options(RUN, flags, &cli::FLAGS)?;
    let quest = given.quest.or_else(quest_of_session);
    if let Err(why) = ask(
        socket_in(given.socket),
        &json!({"id": 1, "ask": "scope", "quest": quest}),
    ) {
        // Run nowhere else: a command asked for in a scope of its own is
        // never run outside one, as systemd-run doesn't.
        return Err(Failure::new(Exit::Failed, format!("no scope for {program}")).because(why));
    }
    let error = std::process::Command::new(program).args(rest).exec();
    Err(
        Failure::passed(127, format!("jobs-broker couldn't run {program}"))
            .because(error.to_string()),
    )
}

fn status(units: &[String]) -> Result<(), Failure> {
    if units.is_empty() {
        return Err(cli::wrongly(
            "jobs-broker status",
            "status needs a UNIT to say",
        ));
    }
    let answer = ask(
        socket_in(None),
        &json!({"id": 1, "ask": "status", "units": units}),
    )
    .map_err(|why| {
        Failure::new(Exit::Failed, "jobs-broker couldn't ask the broker").because(why)
    })?;
    let mut unread = Vec::new();
    let mut said = String::new();
    let told = answer["units"].as_array().cloned().unwrap_or_default();
    for (at, unit) in told.iter().enumerate() {
        if at > 0 {
            said.push('\n');
        }
        let name = unit["unit"].as_str().unwrap_or("?");
        if let Some(why) = unit["error"].as_str() {
            unread.push(format!("{name}: {why}"));
            continue;
        }
        for (key, value) in unit["status"].as_object().into_iter().flatten() {
            let text = match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            said.push_str(key);
            said.push('=');
            said.push_str(&text);
            said.push('\n');
        }
    }
    write_out(&mut std::io::stdout().lock(), &said).map_err(|error| {
        Failure::new(Exit::Failed, "jobs-broker couldn't write the status")
            .because(error.to_string())
    })?;
    if unread.is_empty() {
        Ok(())
    } else {
        Err(Failure::new(
            Exit::Failed,
            format!("jobs-broker couldn't read {} of the units", unread.len()),
        )
        .because(unread.join("\n")))
    }
}

/// Writes `said` whole; a reader that has stopped reading, as `head` does
/// once it has its lines, wanted no more, so that ends it without fault.
fn write_out(out: &mut impl Write, said: &str) -> std::io::Result<()> {
    match out.write_all(said.as_bytes()).and_then(|()| out.flush()) {
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reader that has hung up.
    struct HungUp;

    impl Write for HungUp {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// A disk that is full.
    struct Full;

    impl Write for Full {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::StorageFull.into())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_reader_that_hung_up_ends_the_status_without_fault() {
        assert!(
            write_out(&mut HungUp, "ActiveState=active\n").is_ok(),
            "a closed pipe was taken as a failure"
        );
    }

    #[test]
    fn any_other_failure_to_write_is_said() {
        assert!(
            write_out(&mut Full, "ActiveState=active\n").is_err(),
            "a full disk went unsaid"
        );
    }

    #[test]
    fn the_status_is_written_whole() {
        let mut out = Vec::new();
        write_out(&mut out, "a=1\nb=2\n").expect("a buffer takes every byte");
        assert_eq!(out, b"a=1\nb=2\n", "the status was cut short");
    }
}
