//! update-to-main's command (src/lib.rs says what it's for).
//!
//! - `update-to-main serve --config FILE`: root's daemon, on the config's
//!   socket.
//! - `update-to-main ask [--socket FILE]`: asks for an update and prints
//!   each line as it comes, as JSON. It exits 0 when the machine runs main
//!   or already did, 1 when the update failed, 2 when it was refused and 3
//!   when the daemon couldn't be asked or stopped answering.
//! - `update-to-main stop [--socket FILE]`: stops the update the daemon is
//!   fetching or building, and prints what it did. A switch runs on.
//! - `update-to-main pause|resume [--socket FILE]`: refuses every update,
//!   stopping the one that runs, until resumed; and `update-to-main
//!   hold|lift COMMIT [--socket FILE]` refuses any main holding the
//!   commit, by its full id, until lifted (src/held.rs). Each prints what
//!   holds now, and exits 0, or 2 when the daemon refused the ask.
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
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use update_to_main::config::Config;
use update_to_main::follow::{self, Follower};
use update_to_main::follows::Followed;
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

const USAGE: &str = "usage: update-to-main serve --config FILE
       update-to-main ask [--socket FILE]
       update-to-main stop [--socket FILE]
       update-to-main pause|resume [--socket FILE]
       update-to-main hold|lift COMMIT [--socket FILE]
       update-to-main follow [--config FILE] [--every SECONDS]
       update-to-main job FOLDER";

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
        ["stop"] => stopped(Path::new(SOCKET)),
        ["stop", "--socket", file] => stopped(Path::new(file)),
        [how @ ("pause" | "resume")] => held(Path::new(SOCKET), how),
        [how @ ("pause" | "resume"), "--socket", file] => held(Path::new(file), how),
        [how @ ("hold" | "lift"), commit] => held(Path::new(SOCKET), &format!("{how} {commit}")),
        [how @ ("hold" | "lift"), commit, "--socket", file] => {
            held(Path::new(file), &format!("{how} {commit}"))
        }
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
    let mut unanswered = None;
    let mut seen = None;
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
        match follow::heads(&config, seen.as_ref()) {
            Ok(heads) => {
                unread = None;
                seen = Some(heads.clone());
                let ended = fs::read_to_string(&config.ended).unwrap_or_default();
                if let Some(at) = follow::profile_changed(&config.profile, Instant::now()) {
                    follower.saw_switch(at);
                }
                if follower.should_ask(&heads, &ended, Instant::now(), every) {
                    let at: Vec<String> = heads
                        .iter()
                        .map(|(name, commit)| format!("{name}@{}", &commit[..commit.len().min(12)]))
                        .collect();
                    println!("asking at {}", at.join(" "));
                    // Said like an answer's own ending, since otherwise an
                    // ask that never reached the daemon leaves no trace;
                    // once until it changes, since it's asked every minute.
                    let ending = match heard(&config, &heads) {
                        Ok(ending) => {
                            unanswered = None;
                            ending
                        }
                        Err(e) => {
                            let line = ask::unanswered(&config.socket, &e);
                            if unanswered.as_ref() != Some(&line) {
                                println!("{}", line.json());
                                unanswered = Some(line.clone());
                            }
                            line
                        }
                    };
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

/// Asks for an update at `heads` and hears it to its ending, looking at
/// main meanwhile and stopping the update once it has long fetched heads
/// main has moved past (`follow::stale`), so the next ask fetches the new.
fn heard(config: &Config, heads: &Followed) -> std::io::Result<Line> {
    let (told, lines) = mpsc::channel::<Line>();
    let socket = config.socket.clone();
    let asking = thread::spawn(move || {
        ask::follow(&socket, &mut |line| {
            println!("{}", line.json());
            let _ = told.send(line.clone());
        })
    });
    let (mut latest, mut since, mut stopped) = (None, Instant::now(), false);
    loop {
        match lines.recv_timeout(follow::LOOK_EVERY) {
            Ok(line) => {
                if latest.as_ref() != Some(&line) {
                    since = Instant::now();
                }
                latest = Some(line);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if stopped {
                    continue;
                }
                let Ok(now) = follow::heads(config, Some(heads)) else {
                    continue;
                };
                if follow::stale(heads, &now, latest.as_ref(), since.elapsed()) {
                    println!("main moved past the heads this update builds; stopping it");
                    match ask::stop(&config.socket) {
                        Ok(line) => println!("{}", line.json()),
                        Err(e) => eprintln!("update-to-main: stopping the update: {e}"),
                    }
                    stopped = true;
                }
            }
        }
    }
    asking
        .join()
        .unwrap_or_else(|_| Err(std::io::Error::other("the ask's thread panicked")))
}

fn stopped(socket: &Path) -> ExitCode {
    match ask::stop(socket) {
        Ok(line) => {
            println!("{}", line.json());
            ExitCode::SUCCESS
        }
        Err(e) => {
            if let Line::Refused { why, .. } = ask::unanswered(socket, &e) {
                eprintln!("update-to-main: {why}");
            }
            ExitCode::from(3)
        }
    }
}

fn held(socket: &Path, how: &str) -> ExitCode {
    match ask::once(socket, how) {
        Ok(Line::Refused { why, .. }) if update_to_main::held::says(&why) => {
            println!("{why}");
            ExitCode::SUCCESS
        }
        Ok(line) => {
            eprintln!("update-to-main: {}", line.json());
            ExitCode::from(2)
        }
        Err(e) => {
            if let Line::Refused { why, .. } = ask::unanswered(socket, &e) {
                eprintln!("update-to-main: {why}");
            }
            ExitCode::from(3)
        }
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
            if let Line::Refused { why, .. } = ask::unanswered(socket, &e) {
                eprintln!("update-to-main: {why}");
            }
            ExitCode::from(3)
        }
    }
}
