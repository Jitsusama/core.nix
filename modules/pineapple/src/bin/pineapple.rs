//! Joel's own command for the safe word: which stops stand, and resuming
//! them.
//!
//! `pineapple`, or `pineapple stops`, says each stop standing: its id,
//! whether it is still in shadow, what it reaches, who said it and where.
//! `pineapple resume <scope>` resumes the stops a scope names: `all`, a
//! stop's id, or one of its reaches as `stops` shows it, such as
//! `pr:owner/name#12` or `quest:ID`. Each record is marked resumed, never
//! removed, and the harness stops acting on it within a minute.
//!
//! Only Joel resumes, so `resume` refuses to run under pi: in an agent's
//! shell, under the host, or from a window's own `!`. Joel runs it at a
//! terminal of his own; the launcher's Resume asks `pineapple serve`,
//! which takes it only from his shell (src/serve.rs).
//!
//! `pineapple serve` is that daemon, as its service runs it: it serves
//! the stops, alerts and resume requests as Needs You's rows on its
//! socket, and knocks as each first appears. The harness
//! also refuses an agent's call to it before it starts, saying why; this
//! is the refusal a model can't argue with, since it is the program's.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use pineapple::resume::{self, Standing};
use pineapple::serve::{self as serving, Serving};
use pineapple::vocabulary::Words;
use serde_json::{Value, json};
use terminal_kit::{Exit, Failure, Format, Help, flags};

/// How many parents up a process looks for pi, far past any real tree.
const DEEPEST: usize = 64;

/// The variables pi sets in every shell it starts.
const PI_SAYS: [&str; 3] = ["PI_SESSION_ID", "PI_BASH_CALL", "PI_HOST_CONVERSATION"];

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "pineapple",
        "lists the safe word's stops, and resumes them, for you alone, never an agent",
    )
    .example("pineapple", "the stops standing, the same as stops")
    .example("pineapple resume all", "resumes every stop")
    .subcommand(
        Help::new("stops", "the stops standing, with what each reaches")
            .example("pineapple stops --json", "the same as pineapple.stops/1"),
    )
    .subcommand(
        Help::new(
            "resume",
            "resumes the stops a scope names; refused under pi",
        )
        .argument(
            "<scope>",
            "all, a stop's id, or a reach such as repo:owner/name",
        )
        .example(
            "pineapple resume pr:example/shop#12",
            "resumes the stop on that pull request",
        ),
    )
    .subcommand(
        Help::new(
            "serve",
            "the daemon showing stops in the launcher, as its service runs it",
        )
        .option(
            "--socket PATH",
            "where it listens, unless systemd hands it one",
        )
        .option(
            "--vocabulary PATH",
            "the launcher's words.json, for its rows' words",
        )
        .option(
            "--notify-send PATH",
            "the program it knocks with; none, no notice",
        )
        .option(
            "--every MS",
            "how often it looks for a change; 2 s unless given",
        )
        .example(
            "pineapple serve --socket /run/user/1000/daemons/pineapple.sock",
            "serves until stopped",
        ),
    )
    .exit(Exit::Refused, "run under pi, where only Joel may resume")
    .exit(Exit::NotFound, "no stop standing is named by the scope")
    .exit(Exit::BadInput, "serve's vocabulary can't be read")
}

fn main() -> ExitCode {
    let help = help();
    let commands = help.command_names();
    terminal_kit::main(&help, env!("CARGO_PKG_VERSION"), |context| {
        let args: Vec<&str> = context
            .args
            .iter()
            .map(String::as_str)
            .filter(|arg| !matches!(*arg, "--verbose" | "-v"))
            .collect();
        match args.as_slice() {
            [] | ["stops"] => stops(context),
            ["resume", scope] => resume(context, scope),
            ["serve", rest @ ..] => serve(rest),
            ["resume"] => Err(Failure::new(Exit::Usage, "resume takes a scope")
                .because("it resumes only the stops the scope names")
                .try_this("pineapple stops")),
            ["stops" | "resume", extra, ..] => Err(Failure::new(
                Exit::Usage,
                format!("{extra} isn't taken here"),
            )
            .try_this("pineapple --help")),
            [word, ..] => Err(flags::unknown("command", word, &commands)),
        }
    })
}

fn state() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map_or_else(
            || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state"),
            PathBuf::from,
        )
        .join("pineapple")
}

fn folder() -> PathBuf {
    state().join("stops")
}

/// The daemon, from its flags, which serves until it's stopped.
fn serve(rest: &[&str]) -> Result<(), Failure> {
    const FLAGS: [&str; 4] = ["--socket", "--vocabulary", "--notify-send", "--every"];
    let mut socket = None;
    let mut vocabulary = None;
    let mut notify = None;
    let mut every = serving::EVERY;
    let mut args = rest.iter();
    while let Some(flag) = args.next() {
        let needs = || {
            Failure::new(Exit::Usage, format!("{flag} takes a value"))
                .try_this("pineapple serve --help")
        };
        match *flag {
            "--socket" => socket = Some(PathBuf::from(args.next().ok_or_else(needs)?)),
            "--vocabulary" => vocabulary = Some(PathBuf::from(args.next().ok_or_else(needs)?)),
            "--notify-send" => notify = Some(PathBuf::from(args.next().ok_or_else(needs)?)),
            "--every" => {
                every = args
                    .next()
                    .and_then(|given| given.parse().ok())
                    .map(Duration::from_millis)
                    .ok_or_else(needs)?;
            }
            other => return Err(flags::unknown("option", other, &FLAGS)),
        }
    }
    let socket = socket
        .or_else(|| {
            std::env::var_os("XDG_RUNTIME_DIR")
                .map(|runtime| PathBuf::from(runtime).join("daemons/pineapple.sock"))
        })
        .ok_or_else(|| {
            Failure::new(
                Exit::Usage,
                "no --socket and no XDG_RUNTIME_DIR to listen in",
            )
        })?;
    let words = match vocabulary {
        None => Words::default(),
        Some(path) => std::fs::read_to_string(&path)
            .map_err(|error| error.to_string())
            .and_then(|text| Words::from_vocabulary(&text).map_err(|error| error.to_string()))
            .map_err(|why| {
                Failure::new(Exit::BadInput, "couldn't read the vocabulary")
                    .because(format!("{}: {why}", path.display()))
            })?,
    };
    serving::serve(Serving {
        socket,
        state: state(),
        words,
        notify,
        every,
    })
    .map_err(|error| Failure::new(Exit::Failed, "couldn't serve").because(error.to_string()))
}

fn stops(context: &terminal_kit::Context) -> Result<(), Failure> {
    let standing = resume::standing(&folder());
    let written = match context.format() {
        Format::Text if standing.is_empty() => context.say("no stop stands\n"),
        Format::Text => context.say(&listed(&standing)),
        _ => context.answer(&json!({
            "schema": "pineapple.stops/1",
            "stops": standing.iter().map(|stop| &stop.record).collect::<Vec<_>>(),
        })),
    };
    written.map_err(|error| couldnt_write(&error))
}

fn listed(standing: &[Standing]) -> String {
    let field = |record: &Value, key: &str| {
        record
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let mut out = String::new();
    for stop in standing {
        let shadow = stop.record.get("shadow").and_then(Value::as_bool) != Some(false);
        out.push_str(&stop.id);
        if shadow {
            out.push_str("  (noted only: nothing was stopped)");
        }
        out.push_str("\n  stops ");
        out.push_str(&resume::reaches(&stop.record).join(", "));
        out.push_str("\n  said by ");
        out.push_str(&field(&stop.record, "said_by"));
        out.push_str(", ");
        out.push_str(&field(&stop.record, "where"));
        out.push('\n');
    }
    out
}

fn resume(context: &terminal_kit::Context, scope: &str) -> Result<(), Failure> {
    if let Some(why) = under_pi() {
        return Err(Failure::new(Exit::Refused, "only Joel resumes a stop")
            .because(why)
            .try_this("run it at a terminal of your own"));
    }
    let at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        });
    let resumed = resume::resume(&folder(), scope, at, "joel").map_err(|error| {
        Failure::new(Exit::Failed, "couldn't mark a stop resumed").because(error.to_string())
    })?;
    if resumed.is_empty() {
        return Err(Failure::new(
            Exit::NotFound,
            format!("no stop standing is named by {scope}"),
        )
        .try_this("pineapple stops"));
    }
    let written = match context.format() {
        Format::Text => context.say(&format!(
            "resumed {}; agents carry on within a minute\n",
            resumed.join(", ")
        )),
        _ => context.answer(&json!({ "schema": "pineapple.resumed/1", "resumed": resumed })),
    };
    written.map_err(|error| couldnt_write(&error))
}

fn couldnt_write(error: &std::io::Error) -> Failure {
    Failure::new(Exit::Failed, "couldn't write the answer").because(error.to_string())
}

/// Why this runs under pi, or none when it doesn't: what pi sets in the
/// shells it starts, or pi itself among this process's parents, which a
/// shell can't hide.
fn under_pi() -> Option<String> {
    if let Some(var) = PI_SAYS.iter().find(|var| std::env::var_os(var).is_some()) {
        return Some(format!("{var} is set, as pi sets it in its shells"));
    }
    let mut pid = parent_of("self")?;
    for _ in 0..DEEPEST {
        if pid <= 1 {
            return None;
        }
        if is_pi(pid) {
            return Some(format!("process {pid}, a parent of this one, is pi"));
        }
        pid = parent_of(&pid.to_string())?;
    }
    None
}

fn parent_of(pid: &str) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The name in parentheses may hold spaces, so read after its end.
    let after = &stat[stat.rfind(')')? + 1..];
    after.split_whitespace().nth(1)?.parse().ok()
}

fn is_pi(pid: u32) -> bool {
    let Ok(cmdline) = std::fs::read(format!("/proc/{pid}/cmdline")) else {
        return false;
    };
    let words: Vec<String> = cmdline
        .split(|byte| *byte == 0)
        .map(|word| String::from_utf8_lossy(word).into_owned())
        .collect();
    let first = words.first().map(String::as_str).unwrap_or_default();
    first.rsplit('/').next() == Some("pi")
        || words
            .iter()
            .take(3)
            .any(|word| word.contains("pi-host") || word.contains("pi-coding-agent"))
}
