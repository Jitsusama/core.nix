//! session-backup: each settled pi session log, packed, sealed and copied
//! once into a Google Drive folder (src/lib.rs says how).
//!
//!   session-backup run --config FILE [--most N]   send what has settled
//!   session-backup status --config FILE           what is sent and waiting
//!   session-backup fetch --config FILE NAME --to PATH
//!                                                 read one copy back
//!
//! It takes the shape every command of ours does (docs/commands.md): its
//! help, usage that says what to try, failures with named exits, and each
//! answer as text or JSON.

use std::fmt::Write as _;
use std::path::Path;
use std::process::ExitCode;
use std::time::SystemTime;

use serde_json::json;
use session_backup::backup;
use session_backup::config::Config;
use session_backup::fetch;
use session_backup::index::Index;
use session_backup::ledger::Ledger;
use session_backup::names;
use terminal_kit::{Context, Exit, Failure, Help, flags};

/// What it does, by the first word.
const COMMANDS: [&str; 3] = ["run", "status", "fetch"];

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "session-backup",
        "copies each settled pi session log, packed and sealed, into a Google Drive folder",
    )
    .subcommand(
        sub("status", "what is sent, and what waits to go")
            .example("session-backup status", "the same as bare session-backup"),
    )
    .subcommand(
        sub("run", "sends what has settled, as its timer does each hour")
            .example("session-backup run", "sends every settled log")
            .example("session-backup run --most 5", "five at most")
            .option("--most N", "send N logs at most")
            .exit(
                Exit::Failed,
                "Drive or the login refused, or a log couldn't be sealed",
            ),
    )
    .subcommand(
        sub("fetch", "reads one copy back to a path")
            .example(
                "session-backup fetch NAME --to log.jsonl",
                "NAME's log, opened",
            )
            .argument("<name>", "the copy's name, as status lists it")
            .option("--to PATH", "where the copy goes")
            .exit(Exit::Failed, "Drive refused, or it couldn't be opened"),
    )
    .option("--config FILE", "its settings, which the wrapper names")
    .example(
        "session-backup",
        "what is sent and what waits, as status says",
    )
    .example("session-backup --json", "the same, as JSON")
    .exit(
        Exit::Failed,
        "Drive or the login refused, or a log couldn't be read or sealed",
    )
    .exit(Exit::Config, "the settings can't be read")
}

/// A subcommand, reading the settings as every one does.
fn sub(name: &str, about: &str) -> Help {
    Help::new(name, about)
        .option("--config FILE", "its settings, which the wrapper names")
        .exit(Exit::Config, "the settings can't be read")
}

/// A command line read: the command, its settings, and what else it named.
struct Line<'a> {
    command: &'a str,
    config: &'a str,
    most: Option<usize>,
    to: Option<&'a str>,
    name: Option<&'a str>,
}

fn usage(what: impl Into<String>) -> Failure {
    Failure::new(Exit::Usage, what).try_this("session-backup --help")
}

fn line(args: &[String]) -> Result<Line<'_>, Failure> {
    let Some((command, rest)) = args.split_first() else {
        return Err(usage("it takes run, status or fetch"));
    };
    if !COMMANDS.contains(&command.as_str()) {
        return Err(flags::unknown("command", command, &COMMANDS));
    }
    let (mut config, mut most, mut to, mut name) = (None, None, None, None);
    let mut rest = rest.iter();
    while let Some(word) = rest.next() {
        let mut value = || {
            rest.next()
                .map(String::as_str)
                .ok_or_else(|| usage(format!("{word} needs a value")))
        };
        match (command.as_str(), word.as_str()) {
            (_, "--config") => config = Some(value()?),
            ("run", "--most") => {
                let given = value()?;
                most = Some(
                    given
                        .parse()
                        .map_err(|_| usage(format!("--most takes a count, not {given}")))?,
                );
            }
            ("fetch", "--to") => to = Some(value()?),
            ("fetch", named) if !named.starts_with('-') && name.is_none() => name = Some(named),
            (_, other) => return Err(usage(format!("{command} takes no {other}"))),
        }
    }
    let config = config.ok_or_else(|| {
        usage(format!("{command} needs --config"))
            .because("the session-backup on PATH names it, from jitsusama.session-backup")
    })?;
    if command == "fetch" && (name.is_none() || to.is_none()) {
        return Err(usage("fetch needs a copy's NAME and --to PATH"));
    }
    Ok(Line {
        command,
        config,
        most,
        to,
        name,
    })
}

/// A refusal from the library, said as a failure of the kind given.
fn failed(exit: Exit, what: &str) -> impl Fn(String) -> Failure {
    move |why| {
        Failure::new(exit, what)
            .because(why)
            .try_this("journalctl --user -u session-backup")
    }
}

fn printed(said: std::io::Result<()>) -> Result<(), Failure> {
    said.map_err(|why| Failure::new(Exit::Failed, "it couldn't print it").because(why.to_string()))
}

fn run(context: &Context, config: &Config, most: Option<usize>) -> Result<(), Failure> {
    let done = backup::run(config, SystemTime::now(), most)
        .map_err(failed(Exit::Failed, "the run stopped"))?;
    let keep_days = config.keep.map(|keep| keep.as_secs() / 86_400);
    if context.format().is_json() {
        return printed(context.answer(&json!({
            "schema": "session-backup.run/1",
            "held": done.held,
            "sent": done.sent,
            "bytes": done.bytes,
            "found": done.found,
            "before": done.before,
            "indexed": done.indexed,
            "unread": done.unread,
            "removed": done.removed,
            "freed": done.freed,
            "keep_days": keep_days,
            "busy": done.busy,
        })));
    }
    if done.held {
        return printed(context.say("another run is sending, so this one leaves it to that\n"));
    }
    // Writing to a String can't fail, so what write! returns is let go.
    let mut said = format!(
        "sent {} logs ({} MB sealed), found {} already on Drive; {} were sent before\n",
        done.sent,
        done.bytes / 1_000_000,
        done.found,
        done.before
    );
    if done.indexed > 0 {
        let _ = writeln!(
            said,
            "indexed {} logs, their rows sent sealed",
            done.indexed
        );
    }
    if let Some(why) = &done.unread {
        let _ = writeln!(said, "read no log back from Drive for its row: {why}");
    }
    if done.removed > 0 {
        let _ = writeln!(
            said,
            "removed {} logs Drive holds that were past their {} days here ({} MB)",
            done.removed,
            keep_days.unwrap_or(0),
            done.freed / 1_000_000
        );
    }
    if let Some(why) = &done.busy {
        let _ = writeln!(
            said,
            "Drive stayed busy, so the rest wait for the next run: {why}"
        );
    }
    printed(context.say(&said))
}

fn fetched(context: &Context, config: &Config, name: &str, to: &str) -> Result<(), Failure> {
    let bytes = fetch::fetch(config, name, Path::new(to))
        .map_err(failed(Exit::Failed, &format!("it couldn't fetch {name}")))?;
    if context.format().is_json() {
        return printed(context.answer(&json!({
            "schema": "session-backup.fetch/1",
            "name": name,
            "to": to,
            "bytes": bytes,
        })));
    }
    printed(context.say(&format!("fetched {name} to {to} ({bytes} bytes)\n")))
}

fn status(context: &Context, config: &Config) -> Result<(), Failure> {
    let unread = failed(Exit::Failed, "it couldn't read what is sent");
    let ledger = Ledger::open(&config.ledger).map_err(&unread)?;
    let settled = names::settled(
        &config.sessions,
        &config.machine,
        config.settled,
        SystemTime::now(),
    )
    .map_err(&unread)?;
    let waiting = settled.iter().filter(|log| !ledger.has(&log.name)).count();
    // Without an index no log has a row, and how many lack one isn't known.
    let rows = match &config.index {
        None => None,
        Some(path) => {
            let index = Index::open(path).map_err(&unread)?;
            let rows = format!("{}/index/", config.machine);
            let logs = ledger.sent().filter(|(name, _)| !name.starts_with(&rows));
            Some(logs.fold((0, 0), |(all, rowless), (name, _)| {
                (all + 1, rowless + usize::from(!index.has(name)))
            }))
        }
    };
    if context.format().is_json() {
        return printed(context.answer(&json!({
            "schema": "session-backup.status/1",
            "sent": ledger.len(),
            "waiting": waiting,
            "logs_sent": rows.map(|(all, _)| all),
            "rowless": rows.map(|(_, rowless)| rowless),
        })));
    }
    let mut said = format!(
        "{} objects sent; {waiting} settled logs waiting to go\n",
        ledger.len()
    );
    if let Some((all, rowless)) = rows {
        let _ = writeln!(
            said,
            "{rowless} of the {all} logs sent have no row in the index yet"
        );
    }
    printed(context.say(&said))
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let line = line(&context.args)?;
        let config = Config::read(Path::new(line.config)).map_err(|why| {
            Failure::new(Exit::Config, "its settings can't be read")
                .because(why)
                .try_this("session-backup, which names the settings jitsusama.session-backup wrote")
        })?;
        match (line.command, line.name, line.to) {
            ("run", _, _) => run(context, &config, line.most),
            ("fetch", Some(name), Some(to)) => fetched(context, &config, name, to),
            _ => status(context, &config),
        }
    })
}
