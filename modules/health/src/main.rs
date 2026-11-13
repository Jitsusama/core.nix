//! health: what the machine said about itself today, from its journals.
//!
//! `health follow` is the daemon. It follows every journal the account may
//! read, the system's, its own and the kernel's, from the day's start, for
//! warnings and graver, for the restarts systemd schedules, and for the
//! devices' events the kernel says only as information (a USB device
//! dropped or reset, a PCI Express link's corrected errors). It tallies
//! them by kind (programs, services, devices, security), by the thing that
//! said them and by what was said, its numbers taken out so a line said a
//! thousand times is one signature counted a thousand times. It times each
//! boot from the power button to a ready launcher, Joel's password's wait
//! left out, and each shutdown from the ask to the journal's last line.
//! Each half minute it looks at the lid, and counts as a device's fault a
//! lid that reads shut while the laptop's own screen is on and its own keys
//! are typed on, since every program believes the lid and none says so.
//! And it keeps how using the machine felt: the day's most painful
//! interactions, as the pain-points detector beside it ranks them. It
//! reads the agents' sessions' logs on from where it stopped, for the
//! tools: a call that failed, was refused or was made twice running; and
//! for Joel's words, a line he typed that sounds like something hurt.
//! And each half minute it keeps what every pi session and the shell hold
//! in memory and swap, by how long each has run, so one that grows as it
//! ages shows, and counts the agents that died: a session whose presence
//! says busy or listening though its process is gone. Each five minutes
//! it weighs whose work kept the processors busy, each quest's sessions and
//! commands, his apps' slice and each of the machine's services, by the
//! hour.
//! It keeps the day in `~/.local/state/health/<date>.json`, with the
//! figures cadence day holds it to beside the tally: the slowest boot and
//! shutdown, each kind's errors, the minutes lost, the most each watched
//! program held and the agents that died.
//!
//! `health` reads that file: the day's three most painful interactions,
//! then one line a kind, the worst first, naming its worst thing.
//! `health services` lists a kind's worst things, and `health chrome` shows what each thing whose name holds the words kept saying.
//! `health boot` breaks each boot into its stages. `health launcher` times
//! each part of the launcher against its budget in docs/design.md, the
//! keys' p99 from every key the launcher's trials kept, and how many of
//! its searches Joel left with nothing picked,
//! `health launches` each program's openings and ends as the windows
//! daemon timed them, each daemon's ready and pi's start as it said it,
//! and each service a restart had gone, and
//! `health busy` names each hour's busiest three, in core-minutes, and
//! `health figures` says each figure's side: his, what Joel waited for or
//! saw, or ours, what the machine and its agents did or a model reckons.
//! `--day 2026-11-12` reads another day. Health judges nothing; `cadence
//! day` reads the day and holds it against the days before.

mod agents;
mod asks;
mod busy;
mod day;
mod disk;
mod feel;
mod figures;
mod follow;
mod journal;
mod kernel;
mod launcher;
mod launches;
mod lid;
mod memory;
mod reports;
mod sessions;
mod show;
mod sign;
mod source;
mod trials;
mod typing;
mod unified;

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
health KIND            a kind's worst things: words, programs, services, devices,
                       security or tools
health boot            each boot's stages, and each shutdown's seconds
health memory          what pi and the shell hold, by how long each copy has run
health died            the agents that died, and those gone since before today
health launcher        each part of the launcher, timed against its budget
health launches        each program's openings, ends and ready, the slowest first
health figures         each figure cadence holds the day to, his first, then ours
health busy            whose work kept the processors busy, an hour a line
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
