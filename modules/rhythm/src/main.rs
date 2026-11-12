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

fn main() -> ExitCode {
    let Some(paths) = keep::Paths::from_env() else {
        eprintln!("rhythm needs HOME and XDG_RUNTIME_DIR, as a session gives them");
        return ExitCode::FAILURE;
    };
    match std::env::args().nth(1).as_deref() {
        None => {
            println!("{}", keep::read(&paths));
            ExitCode::SUCCESS
        }
        Some("keep") => match keep::keep(&paths) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("<3>rhythm can't keep what it sees: {error}");
                ExitCode::FAILURE
            }
        },
        Some(other) => {
            eprintln!("rhythm says when you usually stop; `rhythm keep` keeps it, not {other}");
            ExitCode::FAILURE
        }
    }
}
