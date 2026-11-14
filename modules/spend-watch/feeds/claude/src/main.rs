//! claude-plan-feed: a spend-watch feed for a Claude plan, run by the watch
//! and speaking its contract on stdout while it hears the pace on stdin.
//!
//! Usage: `claude-plan-feed [CONFIG_FOLDER]`
//!
//! The config folder is Claude Code's, whose `.credentials.json` holds the
//! plan's sign-in; `config_folder` in the library says which it is when
//! none is given.
//!
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit. Its stdout is the feed contract spend-watch
//! reads in every form, so `--format` says only how a failure is told.

mod anthropic;

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::{env, thread};

use claude_plan_feed::{Feed, Heard, Schedule, config_folder, heard};
use jiff::Timestamp;
use terminal_kit::{Exit, Failure, Help};

use anthropic::Anthropic;

fn help() -> Help {
    Help::new(
        "claude-plan-feed",
        "how a Claude plan stands, for spend-watch",
    )
    .command(
        "[CONFIG_FOLDER]",
        "speaks the feed contract, hearing the pace on stdin",
    )
    .example("claude-plan-feed ~/.claude", "the plan signed in there")
    .exit(Exit::Failed, "it couldn't write to the watch")
    .exit(Exit::Config, "no folder given or named, and no HOME")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let given = match context.args.as_slice() {
            [] => None,
            [folder] => Some(PathBuf::from(folder)),
            _ => {
                return Err(Failure::new(
                    Exit::Usage,
                    "claude-plan-feed takes one config folder at most",
                )
                .try_this("claude-plan-feed --help"));
            }
        };
        let named = env::var_os("PI_CLAUDE_CONFIG_DIR")
            .filter(|named| !named.is_empty())
            .map(PathBuf::from);
        // HOME is only where a folder is looked for, so one given or named
        // needs none.
        let home = env::var_os("HOME")
            .filter(|home| !home.is_empty())
            .map(PathBuf::from)
            .or_else(|| (given.is_some() || named.is_some()).then(PathBuf::new))
            .ok_or_else(|| {
                Failure::new(
                    Exit::Config,
                    "there's no HOME to find Claude Code's config folder in",
                )
                .try_this("claude-plan-feed ~/.claude")
            })?;
        let folder = config_folder(given, named, &home);
        let mut feed = Feed::new(folder.join(".credentials.json"));
        match watch(&mut feed, &mut Anthropic::new(), &listen()) {
            // The watch has gone, so there's no one left to tell.
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
            Err(error) => Err(Failure::new(Exit::Failed, "it couldn't write to the watch")
                .because(error.to_string())),
        }
    })
}

/// Reads the plan whenever the schedule says, until stdout closes.
fn watch(feed: &mut Feed, endpoint: &mut Anthropic, heard: &Receiver<Heard>) -> io::Result<()> {
    say(&feed.introduction())?;
    let mut schedule = Schedule::new(Timestamp::now());
    let mut listening = true;
    loop {
        let wait = schedule.wait(Timestamp::now());
        let next = if listening {
            heard.recv_timeout(wait)
        } else {
            thread::sleep(wait);
            Err(RecvTimeoutError::Timeout)
        };
        match next {
            Ok(Heard::Pace(pace)) => schedule.pace(pace),
            Ok(Heard::Refresh) => schedule.refresh(Timestamp::now()),
            // The watch has stopped writing, so the last pace holds.
            Err(RecvTimeoutError::Disconnected) => listening = false,
            Err(RecvTimeoutError::Timeout) => {
                let now = Timestamp::now();
                let read = feed.read(endpoint, now);
                say(&read.lines)?;
                schedule.read(&read.outcome, now);
            }
        }
    }
}

/// What spend-watch says on stdin, as it's heard, from a thread of its own
/// so a read is never held up waiting for a line.
fn listen() -> Receiver<Heard> {
    let (tell, heard_here) = mpsc::channel();
    thread::spawn(move || {
        for line in io::stdin().lock().lines().map_while(Result::ok) {
            if let Some(said) = heard(&line)
                && tell.send(said).is_err()
            {
                return;
            }
        }
    });
    heard_here
}

fn say(lines: &[String]) -> io::Result<()> {
    let mut out = io::stdout().lock();
    for line in lines {
        writeln!(out, "{line}")?;
    }
    out.flush()
}
