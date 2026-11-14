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
//! terminal of his own, or presses Resume in the launcher. The harness
//! also refuses an agent's call to it before it starts, saying why; this
//! is the refusal a model can't argue with, since it is the program's.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use pineapple::resume::{self, Standing};
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
        "lists the safe word's stops, and resumes them, for Joel alone",
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
    .exit(Exit::Refused, "run under pi, where only Joel may resume")
    .exit(Exit::NotFound, "no stop standing is named by the scope")
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

fn folder() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map_or_else(
            || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state"),
            PathBuf::from,
        )
        .join("pineapple/stops")
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
            out.push_str("  (shadow)");
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
            .try_this("run it at a terminal of your own, or press Resume in the launcher"));
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
