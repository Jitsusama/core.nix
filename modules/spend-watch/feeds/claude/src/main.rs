//! claude-plan-feed: a spend-watch feed for a Claude plan, run by the watch
//! and speaking its contract on stdout while it hears the pace on stdin.
//!
//! Usage: `claude-plan-feed [CONFIG_FOLDER]`
//!
//! The config folder is Claude Code's, whose `.credentials.json` holds the
//! plan's sign-in; `config_folder` in the library says which it is when
//! none is given.

mod anthropic;

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::{env, thread};

use claude_plan_feed::{Feed, Heard, Schedule, config_folder, heard};
use jiff::Timestamp;

use anthropic::Anthropic;

fn main() -> ExitCode {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        eprintln!("claude-plan-feed: no HOME to find Claude Code's config folder in");
        return ExitCode::FAILURE;
    };
    let folder = config_folder(
        env::args_os().nth(1).map(PathBuf::from),
        env::var_os("PI_CLAUDE_CONFIG_DIR").map(PathBuf::from),
        &home,
    );
    let mut feed = Feed::new(folder.join(".credentials.json"));
    match watch(&mut feed, &mut Anthropic::new(), &listen()) {
        Ok(()) => ExitCode::SUCCESS,
        // The watch has gone, so there's no one left to tell.
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("claude-plan-feed: can't write to the watch: {error}");
            ExitCode::FAILURE
        }
    }
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
