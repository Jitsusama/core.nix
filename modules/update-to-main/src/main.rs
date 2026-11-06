//! update-to-main's command (src/lib.rs says what it's for).
//!
//! - `update-to-main serve --config FILE`: root's daemon, on the config's
//!   socket.
//! - `update-to-main ask [--socket FILE]`: asks for an update and prints
//!   each line as it comes, as JSON. It exits 0 when the machine runs main
//!   or already did, 1 when the update failed, 2 when it was refused and 3
//!   when the daemon couldn't be asked or stopped answering.
//! - `update-to-main follow [--config FILE] [--every SECONDS]`: an
//!   account's service that asks whenever main moves (src/follow.rs), and
//!   at least every SECONDS, 600 unless given. It runs until the system's
//!   path holds another update-to-main, so its service starts the newest.
//! - `update-to-main job FOLDER`: a job the daemon starts in an account's
//!   login session on a Mac, running the command written in FOLDER
//!   (src/session.rs).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};
use update_to_main::config::Config;
use update_to_main::follow::{self, Follower};
use update_to_main::line::Line;
use update_to_main::{ask, serve, session};

/// Where the daemon listens unless the config says otherwise, so an asker
/// needn't be told.
const SOCKET: &str = if cfg!(target_os = "macos") {
    "/var/run/update-to-main.sock"
} else {
    "/run/update-to-main/ask.sock"
};

/// Where the module writes the config, which a follower reads too.
const CONFIG: &str = "/etc/update-to-main/config.json";

/// How often a follower asks at least, for a main it can't read cheaply.
const FOLLOW_EVERY: u64 = 600;

const USAGE: &str = "usage: update-to-main serve --config FILE\n       update-to-main ask [--socket FILE]\n       update-to-main follow [--config FILE] [--every SECONDS]\n       update-to-main job FOLDER";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["serve", "--config", file] => {
            match Config::read(&PathBuf::from(file)).and_then(serve::serve) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("update-to-main: {e}");
                    ExitCode::from(3)
                }
            }
        }
        ["ask"] => asked(Path::new(SOCKET)),
        ["ask", "--socket", file] => asked(Path::new(file)),
        ["follow", rest @ ..] => {
            if let Some((config, every)) = follow_args(rest) {
                followed(&config, Duration::from_secs(every))
            } else {
                eprintln!("{USAGE}");
                ExitCode::from(64)
            }
        }
        ["job", folder] => {
            ExitCode::from(u8::try_from(session::job(Path::new(folder))).unwrap_or(1))
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(64)
        }
    }
}

fn follow_args(mut rest: &[&str]) -> Option<(PathBuf, u64)> {
    let (mut config, mut every) = (PathBuf::from(CONFIG), FOLLOW_EVERY);
    loop {
        match rest {
            [] => return Some((config, every)),
            ["--config", file, more @ ..] => (config, rest) = (PathBuf::from(file), more),
            ["--every", seconds, more @ ..] => (every, rest) = (seconds.parse().ok()?, more),
            _ => return None,
        }
    }
}

/// Follows main until the system's path holds another update-to-main, or
/// the config can't be read. The config is read again each time, since a
/// switch may change it.
fn followed(file: &Path, every: Duration) -> ExitCode {
    let started = std::env::current_exe().and_then(fs::canonicalize).ok();
    let mut follower = Follower::default();
    let mut unread = None;
    loop {
        let config = match Config::read(file) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("update-to-main: {e}");
                return ExitCode::from(3);
            }
        };
        if let Some(program) = &config.program
            && fs::canonicalize(program).ok() != started
        {
            println!("the system holds another update-to-main; starting again");
            return ExitCode::SUCCESS;
        }
        match follow::heads(&config) {
            Ok(heads) => {
                unread = None;
                let ended = fs::read_to_string(&config.ended).unwrap_or_default();
                if follower.should_ask(&heads, &ended, Instant::now(), every) {
                    let at: Vec<String> = heads
                        .iter()
                        .map(|(name, commit)| format!("{name}@{}", &commit[..commit.len().min(12)]))
                        .collect();
                    println!("asking at {}", at.join(" "));
                    let ending = ask::ask(&config.socket, &mut |l| println!("{}", l.json()))
                        .unwrap_or_else(|e| Line::Refused {
                            commit: None,
                            why: format!("{}: {e}", config.socket.display()),
                        });
                    let ended = fs::read_to_string(&config.ended).unwrap_or_default();
                    if let Some(why) = follower.asked(heads, &ending, &ended, Instant::now()) {
                        println!("stopped until someone asks by hand: {why}");
                    }
                }
            }
            // Said once until it changes, since the host may be away a while.
            Err(why) if unread.as_ref() != Some(&why) => {
                eprintln!("update-to-main: {why}");
                unread = Some(why);
            }
            Err(_) => {}
        }
        thread::sleep(follow::LOOK_EVERY);
    }
}

fn asked(socket: &Path) -> ExitCode {
    let ending = ask::ask(socket, &mut |line| println!("{}", line.json()));
    match ending {
        Ok(Line::Done { .. } | Line::Unchanged { .. }) => ExitCode::SUCCESS,
        Ok(Line::Failed { .. }) => ExitCode::from(1),
        Ok(Line::Refused { .. }) => ExitCode::from(2),
        Ok(_) => ExitCode::from(3),
        Err(e) => {
            eprintln!("update-to-main: {}: {e}", socket.display());
            ExitCode::from(3)
        }
    }
}
