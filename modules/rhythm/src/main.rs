//! rhythm learns when Joel stops for the night, from when he's at the
//! machine, so the evening's wind-down can follow him rather than a
//! clock (Joel, 2026-11-11: "i need for it to be adaptive").
//!
//! `rhythm keep`, a user service, looks each minute for swayidle-away's
//! mark of someone at the machine and keeps the minutes it's there, times
//! only (keep.rs), seeded on its first run from the times zsh's history
//! and pi's session logs hold (seed.rs). From them it finds when he
//! stopped each night and what's usual, by weekday where a weekday has its
//! own habit (nights.rs), and writes that, with today's sunrise and sunset
//! where whereabouts says he is, to `$XDG_STATE_HOME/rhythm/profile.json`
//! (read.rs says its shape) for the backlight and others to follow.
//!
//! `rhythm`, typed in a terminal, says it in plain words.
//!
//! In daylight it also suggests a few minutes outside after a long stretch
//! at the machine, at most three a day and one in poor weather, never
//! during a call or while Quiet holds notices (walk.rs says when).

mod clock;
mod keep;
mod nights;
mod read;
mod season;
mod seed;
mod walk;

use std::process::ExitCode;

use terminal_kit::{Exit, Failure, Help};

const NO_SESSION: &str = "there's no session's HOME or XDG_RUNTIME_DIR";

/// What `rhythm --help` says: its view, and `keep`, which its unit runs.
fn help() -> Help {
    Help::new("rhythm", "says when you usually stop for the night")
        .example("rhythm", "when you stop, by weekday, and the sun")
        .subcommand(
            Help::new("keep", "keeps the minutes you're at the machine")
                .example("rhythm keep", "as its unit runs it, each minute")
                .exit(Exit::Failed, "what it saw couldn't be kept")
                .exit(Exit::Config, NO_SESSION),
        )
        .exit(Exit::Config, NO_SESSION)
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let paths = keep::Paths::from_env().ok_or_else(|| {
            Failure::new(Exit::Config, NO_SESSION).because("rhythm reads a session's own folders")
        })?;
        if context.args.is_empty() {
            println!("{}", keep::read(&paths));
            return Ok(());
        }
        keep::keep(&paths).map_err(|error| {
            Failure::new(Exit::Failed, "rhythm can't keep what it sees").because(error.to_string())
        })
    })
}
