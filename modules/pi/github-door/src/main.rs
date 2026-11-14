//! `pi-github-door serve` is the door, answering its control socket and
//! every session's own (src/lib.rs says why and how); `pi-github-door
//! relay` starts the relay a fenced command reaches it through, printing
//! its port; `pi-github-door lift` lifts a session's hourly cap, for Joel.
//!
//! It takes the shape every command of ours does (docs/commands.md): its
//! help, usage that says what to try, and failures with named exits.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener};
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use pi_github_door::control::Sessions;
use pi_github_door::door::{Door, Upstream};
use pi_github_door::graphql;
use pi_github_door::relay;
use pi_github_door::token::Token;
use serde_json::{Value, json};
use terminal_kit::{Context, Exit, Failure, Help, flags};

/// What it does, by the first word.
const COMMANDS: [&str; 4] = ["serve", "relay", "lift", "relaying"];
/// Where the system's roots are, unless told otherwise.
const SYSTEM_BUNDLE: &str = "/etc/ssl/certs/ca-certificates.crt";
/// The first socket systemd hands a service it starts.
const FIRST_PASSED_FD: i32 = 3;
/// The descriptor a relay's child takes its listener on.
const STDIN_FD: i32 = 0;

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "pi-github-door",
        "the door fenced pi commands reach GitHub through, adding the token only to calls it \
         allows",
    )
    .example(
        "pi-github-door lift 01a1d112",
        "lifts a session's hourly cap",
    )
    .subcommand(
        Help::new(
            "serve",
            "answers the control socket and every session's own",
        )
        .option("--sessions DIR", "where each session's folder is made")
        .option(
            "--control PATH",
            "the control socket to bind, if systemd passed none",
        )
        .option(
            "--system-bundle FILE",
            "the system's roots, put in each session's bundle",
        )
        .option(
            "--roots FILE",
            "the roots GitHub is checked against, if not the system's",
        )
        .option(
            "--gh PROGRAM",
            "the gh whose auth token is the person's, if not gh",
        )
        .option(
            "--resolve HOST=ADDR",
            "reach a host at an address, for a stand-in",
        )
        .example(
            "pi-github-door serve --sessions \"$XDG_RUNTIME_DIR/pi-github/sessions\"",
            "as its unit runs it",
        )
        .exit(Exit::Failed, "the door couldn't listen, or read its roots")
        .exit(Exit::Usage, "a flag is missing or unknown"),
    )
    .subcommand(
        Help::new(
            "relay",
            "starts a session's relay on localhost and prints its port",
        )
        .option("--socket PATH", "the session's socket")
        .option("--while PID", "the process whose life the relay lasts")
        .example(
            "pi-github-door relay --socket door.sock --while $$",
            "the port, for HTTPS_PROXY",
        )
        .exit(Exit::Failed, "no port, or the relay couldn't start")
        .exit(Exit::Usage, "a flag is missing or unknown"),
    )
    .subcommand(
        Help::new(
            "lift",
            "lifts a session's hourly cap for the rest of its hour",
        )
        .argument("<session>", "the session's id")
        .option("--control PATH", "the control socket, if not the usual one")
        .example(
            "pi-github-door lift 01a1d112",
            "it may call GitHub past 2,000 this hour",
        )
        .exit(Exit::NotFound, "no session goes by that id")
        .exit(Exit::Unavailable, "the door isn't answering"),
    )
    .subcommand(
        Help::new(
            "relaying",
            "the relay's own child, answering the port it was handed",
        )
        .option("--socket PATH", "the session's socket")
        .option("--while PID", "the process whose life it lasts")
        .example(
            "pi-github-door relaying --socket door.sock --while 1",
            "as relay starts it",
        )
        .exit(Exit::Usage, "a flag is missing or unknown")
        .unlisted(),
    )
}

/// Each named flag's values, in the order given, or a usage failure for
/// a flag it doesn't read or one left without a value.
fn named(command: &str, args: &[String], known: &[&str]) -> Result<Vec<(String, String)>, Failure> {
    let mut given = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if !known.contains(&arg.as_str()) {
            return Err(flags::unknown("flag", arg, known));
        }
        let value = args.next().ok_or_else(|| {
            Failure::new(Exit::Usage, format!("{arg} needs a value"))
                .try_this(format!("pi-github-door {command} --help"))
        })?;
        given.push((arg.clone(), value.clone()));
    }
    Ok(given)
}

/// The last value given for `flag`.
fn value<'a>(given: &'a [(String, String)], flag: &str) -> Option<&'a str> {
    given
        .iter()
        .rev()
        .find(|(name, _)| name == flag)
        .map(|(_, value)| value.as_str())
}

/// The usage failure for a flag a command can't go without.
fn missing(command: &str, flag: &str) -> Failure {
    Failure::new(Exit::Usage, format!("{command} needs {flag}"))
        .try_this(format!("pi-github-door {command} --help"))
}

/// The control socket systemd passed, or one bound at `path`.
fn control_listener(path: Option<&str>) -> Result<UnixListener, Failure> {
    let failed = |why: String| {
        Failure::new(Exit::Failed, "the door couldn't listen")
            .because(why)
            .try_this("systemctl --user status pi-github-door.socket")
    };
    if let Some(path) = path {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(failed(error.to_string())),
        }
        return UnixListener::bind(path).map_err(|error| failed(error.to_string()));
    }
    let ours =
        env::var("LISTEN_PID").ok().and_then(|pid| pid.parse().ok()) == Some(std::process::id());
    if !ours || env::var("LISTEN_FDS").as_deref() != Ok("1") {
        return Err(failed(
            "systemd passed no socket, and no --control was given".to_owned(),
        ));
    }
    // SAFETY: systemd passed this process one listening socket, at the
    // first descriptor it hands on, and nothing else here owns it.
    Ok(unsafe { UnixListener::from_raw_fd(FIRST_PASSED_FD) })
}

fn serve(args: &[String]) -> Result<(), Failure> {
    let known = [
        "--sessions",
        "--control",
        "--system-bundle",
        "--roots",
        "--gh",
        "--resolve",
    ];
    let given = named("serve", args, &known)?;
    let sessions = value(&given, "--sessions").ok_or_else(|| missing("serve", "--sessions"))?;
    let system_bundle = PathBuf::from(value(&given, "--system-bundle").unwrap_or(SYSTEM_BUNDLE));
    let roots_file = value(&given, "--roots").map_or_else(|| system_bundle.clone(), PathBuf::from);
    let gh = value(&given, "--gh").unwrap_or("gh");
    let mut resolve = HashMap::new();
    for (_, pair) in given.iter().filter(|(name, _)| name == "--resolve") {
        let parsed = pair.split_once('=').and_then(|(host, address)| {
            Some((host.to_owned(), address.parse::<SocketAddr>().ok()?))
        });
        let Some((host, address)) = parsed else {
            return Err(Failure::new(
                Exit::Usage,
                format!("--resolve takes HOST=ADDR, not {pair}"),
            )
            .try_this("pi-github-door serve --help"));
        };
        resolve.insert(host, address);
    }
    let roots = fs::read(&roots_file).map_err(|error| {
        Failure::new(Exit::Failed, "the door couldn't read its roots")
            .because(format!("{}: {error}", roots_file.display()))
            .try_this("give --roots a PEM bundle")
    })?;
    let upstream = Upstream::new(&roots, resolve).map_err(|why| {
        Failure::new(Exit::Failed, "the door couldn't read its roots")
            .because(why)
            .try_this("give --roots a PEM bundle")
    })?;
    let listener = control_listener(value(&given, "--control"))?;
    let door = Arc::new(Door {
        token: Token::new(vec![gh.to_owned(), "auth".to_owned(), "token".to_owned()]),
        upstream,
        allowed: graphql::allowed()
            .iter()
            .map(|entry| entry.name.as_str())
            .collect(),
    });
    let sessions = Arc::new(
        Sessions::new(door, Path::new(sessions), &system_bundle).map_err(|error| {
            Failure::new(Exit::Failed, "the door couldn't make its sessions' folder")
                .because(error.to_string())
                .try_this("check that the runtime folder is there and writable")
        })?,
    );
    sessions.serve(&listener);
    Ok(())
}

/// The socket and the watched process a relay is given.
fn relay_flags(command: &str, args: &[String]) -> Result<(PathBuf, u32), Failure> {
    let given = named(command, args, &["--socket", "--while"])?;
    let socket = value(&given, "--socket").ok_or_else(|| missing(command, "--socket"))?;
    let watch = value(&given, "--while")
        .ok_or_else(|| missing(command, "--while"))?
        .parse()
        .map_err(|_| {
            Failure::new(Exit::Usage, "--while takes a process id")
                .try_this(format!("pi-github-door {command} --help"))
        })?;
    Ok((PathBuf::from(socket), watch))
}

fn start_relay(context: &Context, args: &[String]) -> Result<(), Failure> {
    let (socket, watch) = relay_flags("relay", args)?;
    let port = relay::launch(&socket, watch).map_err(|why| {
        Failure::new(Exit::Failed, "the relay couldn't start")
            .because(why)
            .try_this("pi-github-door relay --socket PATH --while $$")
    })?;
    let said = if context.format().is_json() {
        context.answer(&json!({ "schema": "pi-github-door.relay/1", "port": port }))
    } else {
        context.say(&format!("{port}\n"))
    };
    said.map_err(|error| {
        Failure::new(Exit::Failed, "it couldn't print the port").because(error.to_string())
    })
}

fn relaying(args: &[String]) -> Result<(), Failure> {
    let (socket, watch) = relay_flags("relaying", args)?;
    // SAFETY: relay hands its child the listener as its standard input,
    // and nothing else in this process reads it.
    let listener = TcpListener::from(unsafe { OwnedFd::from_raw_fd(STDIN_FD) });
    relay::serve(&listener, &socket, watch);
    Ok(())
}

/// The control socket's usual place.
fn usual_control() -> Result<PathBuf, Failure> {
    env::var_os("XDG_RUNTIME_DIR")
        .map(|runtime| PathBuf::from(runtime).join("pi-github/control.sock"))
        .ok_or_else(|| {
            Failure::new(Exit::Config, "XDG_RUNTIME_DIR isn't set")
                .try_this("pi-github-door lift SESSION --control PATH")
        })
}

fn lift(context: &Context, args: &[String]) -> Result<(), Failure> {
    let Some((session, rest)) = args.split_first() else {
        return Err(Failure::new(Exit::Usage, "lift takes a session's id")
            .try_this("pi-github-door lift --help"));
    };
    let given = named("lift", rest, &["--control"])?;
    let control =
        value(&given, "--control").map_or_else(usual_control, |path| Ok(PathBuf::from(path)))?;
    let unavailable = |why: String| {
        Failure::new(Exit::Unavailable, "the door isn't answering")
            .because(why)
            .try_this("systemctl --user start pi-github-door.socket")
    };
    let mut stream =
        UnixStream::connect(&control).map_err(|error| unavailable(error.to_string()))?;
    writeln!(stream, "{}", json!({ "ask": "lift", "session": session }))
        .map_err(|error| unavailable(error.to_string()))?;
    let mut line = String::new();
    BufReader::new(&stream)
        .read_line(&mut line)
        .map_err(|error| unavailable(error.to_string()))?;
    let answer: Value = serde_json::from_str(&line)
        .map_err(|_| unavailable("it answered unreadably".to_owned()))?;
    if answer.get("ok") != Some(&Value::Bool(true)) {
        let why = answer
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("it refused")
            .to_owned();
        return Err(
            Failure::new(Exit::NotFound, format!("no session {session} to lift"))
                .because(why)
                .try_this("journalctl --user -u pi-github-door"),
        );
    }
    let said = if context.format().is_json() {
        context.answer(&json!({ "schema": "pi-github-door.lifted/1", "session": session }))
    } else {
        context.say(&format!(
            "lifted {session}'s cap for the rest of its hour\n"
        ))
    };
    said.map_err(|error| {
        Failure::new(Exit::Failed, "it couldn't say so").because(error.to_string())
    })
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| match context
        .args
        .split_first()
    {
        Some((command, rest)) if command == "serve" => serve(rest),
        Some((command, rest)) if command == "relay" => start_relay(context, rest),
        Some((command, rest)) if command == "relaying" => relaying(rest),
        Some((command, rest)) if command == "lift" => lift(context, rest),
        Some((command, _)) => Err(flags::unknown("command", command, &COMMANDS)),
        None => Err(Failure::new(Exit::Usage, "it takes serve, relay or lift")
            .try_this("pi-github-door --help")),
    })
}
