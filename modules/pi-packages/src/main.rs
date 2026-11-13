//! pi-packages: each of pi's packages built into a version of its own and
//! swapped in whole (src/lib.rs says how).
//!
//! Usage:
//!   pi-packages update --config FILE [--now]
//!                                      bring every package to its branch,
//!                                      swapping at most once an hour unless
//!                                      told --now
//!   pi-packages sweep --config FILE    remove the versions no pi runs
//!   pi-packages status --config FILE   where each package's current is,
//!                                      and any hold
//!   pi-packages hold --config FILE [--for 90m|2h] WHY...
//!                                      swap nothing, however an update
//!                                      runs, until released or for a time
//!   pi-packages release --config FILE  end the hold
//!   pi-packages --help                 this, as `hold --help` is too; a
//!                                      hold's reason never opens with a
//!                                      flag (src/ask.rs)

use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, SystemTime};

use pi_packages::ask::{Asked, USAGE, asked};
use pi_packages::config::Config;
use pi_packages::hold::{self, Hold};
use pi_packages::run::{self, Done, Pace};
use pi_packages::version::Place;

/// Characters of a commit worth showing.
const SHORT: usize = 8;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    let done = asked(&words).and_then(|asked| match asked {
        Asked::Help => {
            println!("{USAGE}");
            Ok(())
        }
        Asked::Update { file, pace } => update(file, pace),
        Asked::Sweep(file) => sweep(file),
        Asked::Status(file) => status(file),
        Asked::Hold { file, why, length } => put_hold(file, why, length),
        Asked::Release(file) => release(file),
    });
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("pi-packages: {reason}");
            ExitCode::FAILURE
        }
    }
}

/// How long a hold lasts, as a person reads it.
fn how_long(now: SystemTime, hold: &Hold) -> String {
    hold.until.map_or_else(
        || "until released".to_owned(),
        |until| format!("for {} more minutes", minutes_until(now, until)),
    )
}

fn put_hold(file: &Path, why: String, length: Option<Duration>) -> Result<(), String> {
    let config = Config::read(file)?;
    let now = SystemTime::now();
    let hold = Hold {
        why,
        until: length.map(|length| now + length),
    };
    hold::put(&config, &hold)?;
    println!("swaps are held {}: {}", how_long(now, &hold), hold.why);
    Ok(())
}

fn release(file: &Path) -> Result<(), String> {
    let config = Config::read(file)?;
    if hold::release(&config)? {
        println!("the hold is over; the next update swaps in what waited");
    } else {
        println!("no hold stood");
    }
    Ok(())
}

fn short(commit: &str) -> &str {
    commit.get(..SHORT).unwrap_or(commit)
}

/// Whole minutes from `now` to `until`, rounded up, so a wait never reads
/// as none.
fn minutes_until(now: SystemTime, until: SystemTime) -> u64 {
    until
        .duration_since(now)
        .unwrap_or(Duration::ZERO)
        .as_secs()
        .div_ceil(60)
}

fn update(file: &Path, pace: Pace) -> Result<(), String> {
    let config = Config::read(file)?;
    let mut failed = Vec::new();
    let now = SystemTime::now();
    for (package, outcome) in run::update(&config, now, pace) {
        let name = package.key.display();
        match outcome {
            Err(why) => {
                println!("{name}: not updated: {why}");
                failed.push(name.to_string());
            }
            Ok(outcome) => {
                match &outcome.done {
                    Done::Current { commit } => {
                        println!("{name}: at {}, its branch's head", short(commit));
                    }
                    Done::Moved {
                        from: Some(from),
                        to,
                    } => {
                        println!("{name}: moved from {} to {}", short(from), short(to));
                    }
                    Done::Moved { from: None, to } => {
                        println!("{name}: installed at {}", short(to));
                    }
                    Done::Busy => println!("{name}: another update holds it, so this one left it"),
                    Done::OnHold { commit, hold } => println!(
                        "{name}: {} waits, since swaps are held {}: {}; \
                         `pi-packages release` ends the hold",
                        short(commit),
                        how_long(now, hold),
                        hold.why
                    ),
                    Done::Held { commit, until } => println!(
                        "{name}: {} waits {} more minutes, since installs swap in at most \
                         once an hour; `pi-packages update --now` swaps it in now",
                        short(commit),
                        minutes_until(now, *until)
                    ),
                    Done::Failed { commit, why } => {
                        println!(
                            "{name}: {} not swapped in, current left as it was: {why}",
                            short(commit)
                        );
                        failed.push(name.to_string());
                    }
                }
                if let Some(why) = &outcome.unfetched {
                    println!("{name}: could not fetch, so used the last commit seen: {why}");
                }
                for why in &outcome.unwarmed {
                    println!("{name}: a pi's cache was not warmed before the swap: {why}");
                }
                if outcome.removed > 0 {
                    println!(
                        "{name}: removed {} old versions nothing uses",
                        outcome.removed
                    );
                }
            }
        }
    }
    // One line each age-out, which the daily disk figure counts.
    if let Some(aged) = run::age_jiti(&config, now) {
        println!(
            "jiti: removed {} files, {} bytes, older than {} days",
            aged.files,
            aged.bytes,
            run::JITI_KEEPS.as_secs() / (24 * 3600)
        );
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!("not updated: {}", failed.join(", ")))
    }
}

fn sweep(file: &Path) -> Result<(), String> {
    let config = Config::read(file)?;
    let mut failed = Vec::new();
    for (package, removed) in run::sweep(&config) {
        let name = package.key.display();
        match removed {
            Ok(Some(0)) => {}
            Ok(Some(removed)) => println!("{name}: removed {removed} old versions nothing uses"),
            Ok(None) => println!("{name}: an update holds it, and removes what it can"),
            Err(why) => {
                println!("{name}: nothing removed: {why}");
                failed.push(name.to_string());
            }
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!("nothing removed: {}", failed.join(", ")))
    }
}

fn status(file: &Path) -> Result<(), String> {
    let config = Config::read(file)?;
    let now = SystemTime::now();
    if let Some(hold) = hold::standing(&config, now) {
        println!("swaps are held {}: {}", how_long(now, &hold), hold.why);
    }
    for package in &config.packages {
        let place = Place::of(&config, package);
        match place.current_commit() {
            Some(commit) => println!("{}: current is {}", package.key.display(), short(&commit)),
            None => println!("{}: nothing installed yet", package.key.display()),
        }
    }
    Ok(())
}
