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
//! says busy or listening though its process is gone, and the most
//! defunct children one parent left unreaped. Each five minutes
//! it weighs whose work kept the processors busy, each quest's sessions and
//! commands, his apps' slice and each of the machine's services, by the
//! hour.
//! It keeps the day in `~/.local/state/health/<date>.json`, with the
//! figures cadence day holds it to beside the tally: the slowest boot and
//! shutdown, each kind's errors, the minutes lost, the most each watched
//! program held, the agents that died and the most defunct children
//! under one parent.
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
mod cli;
mod day;
mod defunct;
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
mod reloads;
mod replay;
mod reports;
mod sessions;
mod show;
mod sign;
mod source;
mod trials;
mod typing;
mod unified;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use terminal_kit::{Context, Exit, Failure};

use crate::cli::{Asked, View};

fn main() -> ExitCode {
    terminal_kit::main(&cli::help(), env!("CARGO_PKG_VERSION"), run)
}

/// Does what the words ask: follows the journals, or shows the day's
/// view they name, fitted to the terminal.
fn run(context: &Context) -> Result<(), Failure> {
    // Its views are said as text alone; a caller asking for JSON is told so
    // rather than handed text it would fail to read.
    if context.format().is_json() {
        return Err(
            Failure::new(Exit::Usage, "health says its views as text alone")
                .try_this("health --help --format json"),
        );
    }
    let Asked { into, day, view } = cli::read(&context.args)?;
    let into = into.unwrap_or_else(kept_in);
    if view == View::Follow {
        return follow::follow(&into).map_err(|fault| {
            Failure::new(Exit::Failed, "health stopped following the journals")
                .because(fault.to_string())
        });
    }
    let date = day
        .unwrap_or_else(|| follow::date_of(Timestamp::now().as_microsecond(), &TimeZone::system()));
    let day = cli::day_in(&into, &date)?;
    let shown = match view {
        View::Words(words) => show::asked(&day, &words, &context.out, context.room),
        View::Day | View::Follow => show::day(&day, &context.out, context.room),
    };
    print!("{shown}");
    Ok(())
}

/// Where the days are kept: the account's state folder.
fn kept_in() -> PathBuf {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_default()
        .join("health")
}
