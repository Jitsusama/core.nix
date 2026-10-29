//! session-backup: each settled pi session log, packed, sealed and copied
//! once into a Google Drive folder (src/lib.rs says how).
//!
//! Usage:
//!   session-backup run --config FILE [--most N]   send what has settled
//!   session-backup status --config FILE           what is sent and waiting
//!   session-backup fetch --config FILE NAME --to PATH
//!                                                 read one copy back

use std::path::Path;
use std::process::ExitCode;
use std::time::SystemTime;

use session_backup::backup;
use session_backup::config::Config;
use session_backup::fetch;
use session_backup::ledger::Ledger;
use session_backup::names;

const USAGE: &str = "usage: session-backup run --config FILE [--most N] | status --config FILE \
     | fetch --config FILE NAME --to PATH";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    let done = match words.as_slice() {
        ["run", "--config", file] => run(Path::new(file), None),
        ["run", "--config", file, "--most", most] => match most.parse() {
            Ok(most) => run(Path::new(file), Some(most)),
            Err(_) => Err(USAGE.into()),
        },
        ["status", "--config", file] => status(Path::new(file)),
        ["fetch", "--config", file, name, "--to", to] => {
            fetched(Path::new(file), name, Path::new(to))
        }
        _ => Err(USAGE.into()),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("session-backup: {reason}");
            ExitCode::FAILURE
        }
    }
}

fn run(file: &Path, most: Option<usize>) -> Result<(), String> {
    let config = Config::read(file)?;
    let done = backup::run(&config, SystemTime::now(), most)?;
    if done.held {
        println!("another run is sending, so this one leaves it to that");
        return Ok(());
    }
    println!(
        "sent {} logs ({} MB sealed), found {} already on Drive; {} were sent before",
        done.sent,
        done.bytes / 1_000_000,
        done.found,
        done.before
    );
    if done.indexed > 0 {
        println!("indexed {} logs, their rows sent sealed", done.indexed);
    }
    if done.removed > 0 {
        println!(
            "removed {} logs Drive holds that were past their {} days here ({} MB)",
            done.removed,
            config.keep.map_or(0, |keep| keep.as_secs() / 86_400),
            done.freed / 1_000_000
        );
    }
    if let Some(why) = done.busy {
        println!("Drive stayed busy, so the rest wait for the next run: {why}");
    }
    Ok(())
}

fn fetched(file: &Path, name: &str, to: &Path) -> Result<(), String> {
    let config = Config::read(file)?;
    let bytes = fetch::fetch(&config, name, to)?;
    println!("fetched {name} to {} ({bytes} bytes)", to.display());
    Ok(())
}

fn status(file: &Path) -> Result<(), String> {
    let config = Config::read(file)?;
    let ledger = Ledger::open(&config.ledger)?;
    let settled = names::settled(
        &config.sessions,
        &config.host,
        config.settled,
        SystemTime::now(),
    )?;
    let waiting = settled.iter().filter(|log| !ledger.has(&log.name)).count();
    println!(
        "{} objects sent; {waiting} settled logs waiting to go",
        ledger.len()
    );
    Ok(())
}
