//! health: what the machine said about itself today, from its journals.
//!
//! `health follow` is the daemon. It follows every journal the account may
//! read, the system's, its own and the kernel's, from the day's start, for
//! warnings and graver, for the restarts systemd schedules, and for the
//! devices' events the kernel says only as information (a USB device
//! dropped or reset, a PCI Express link's corrected errors). It tallies
//! them by kind (programs, services, devices, security), by the thing that
//! said them and by what was said, its numbers taken out so a line said a
//! thousand times is one signature counted a thousand times, and keeps the
//! day in `~/.local/state/health/<date>.json`.
//!
//! `health` reads that file: one line a kind, the worst first, naming its
//! worst thing. `health services` lists a kind's worst things, and `health
//! chrome` shows what each thing whose name holds the words kept saying.
//! `--day 2026-11-12` reads another day. Health judges nothing; `cadence
//! day` reads the day and holds it against the days before.

mod day;
mod follow;
mod journal;
mod show;
mod sign;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::day::Day;

/// What `health --help` says.
const USAGE: &str = "\
health                 what today's journals said, one line a kind, the worst first
health KIND            a kind's worst things: programs, services, devices or security
health WORDS           what each thing whose name holds the words kept saying
health --day DATE ...  another day, as YYYY-MM-DD
health follow          the daemon, which keeps the day's file current
";

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let into = match take_value(&mut args, "--into") {
        Ok(into) => into.map_or_else(kept_in, PathBuf::from),
        Err(said) => return refused(&said),
    };
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    if args.first().is_some_and(|arg| arg == "follow") {
        return match follow::follow(&into) {
            Ok(()) => ExitCode::SUCCESS,
            Err(fault) => {
                eprintln!("health: {fault}");
                ExitCode::FAILURE
            }
        };
    }
    let date = match take_value(&mut args, "--day") {
        Ok(date) => date.unwrap_or_else(|| {
            follow::date_of(Timestamp::now().as_microsecond(), &TimeZone::system())
        }),
        Err(said) => return refused(&said),
    };
    let Ok(file) = fs::read(follow::file_of(&into, &date)) else {
        eprintln!("health: nothing kept for {date}; is it running? systemctl --user status health");
        return ExitCode::FAILURE;
    };
    let day: Day = match serde_json::from_slice(&file) {
        Ok(day) => day,
        Err(fault) => {
            eprintln!("health: {date}'s file can't be read: {fault}");
            return ExitCode::FAILURE;
        }
    };
    let words = args.join(" ");
    if words.is_empty() {
        print!("{}", show::day(&day));
    } else {
        print!("{}", show::asked(&day, &words));
    }
    ExitCode::SUCCESS
}

/// Take a flag and its value out of the arguments, if it's there.
fn take_value(args: &mut Vec<String>, flag: &str) -> Result<Option<String>, String> {
    let Some(place) = args.iter().position(|arg| arg == flag) else {
        return Ok(None);
    };
    if place + 1 >= args.len() {
        return Err(format!("{flag} needs a value"));
    }
    let value = args.remove(place + 1);
    args.remove(place);
    Ok(Some(value))
}

/// Where the days are kept: the account's state folder.
fn kept_in() -> PathBuf {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_default()
        .join("health")
}

/// Say why the arguments weren't taken, and how they're given.
fn refused(said: &str) -> ExitCode {
    eprint!("health: {said}\n{USAGE}");
    ExitCode::from(64)
}
