//! pi-packages: each of pi's packages built into a release of its own and
//! swapped in whole (src/lib.rs says how).
//!
//! Usage:
//!   pi-packages update --config FILE   bring every package to its branch
//!   pi-packages status --config FILE   where each package's current is

use std::path::Path;
use std::process::ExitCode;
use std::time::SystemTime;

use pi_packages::config::Config;
use pi_packages::release::Place;
use pi_packages::run::{self, Done};

const USAGE: &str = "usage: pi-packages update --config FILE | status --config FILE";

/// Characters of a commit worth showing.
const SHORT: usize = 8;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    let done = match words.as_slice() {
        ["update", "--config", file] => update(Path::new(file)),
        ["status", "--config", file] => status(Path::new(file)),
        _ => Err(USAGE.into()),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("pi-packages: {reason}");
            ExitCode::FAILURE
        }
    }
}

fn short(commit: &str) -> &str {
    commit.get(..SHORT).unwrap_or(commit)
}

fn update(file: &Path) -> Result<(), String> {
    let config = Config::read(file)?;
    let mut failed = Vec::new();
    for (package, outcome) in run::update(&config, SystemTime::now()) {
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
                if outcome.removed > 0 {
                    println!(
                        "{name}: removed {} old releases nothing uses",
                        outcome.removed
                    );
                }
            }
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!("not updated: {}", failed.join(", ")))
    }
}

fn status(file: &Path) -> Result<(), String> {
    let config = Config::read(file)?;
    for package in &config.packages {
        let place = Place::of(&config, package);
        match place.current_commit() {
            Some(commit) => println!("{}: current is {}", package.key.display(), short(&commit)),
            None => println!("{}: nothing installed yet", package.key.display()),
        }
    }
    Ok(())
}
