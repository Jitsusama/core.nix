//! Says whether a call is on, for whatever mustn't cut one, such as an
//! update with nobody at it.
//!
//! ```text
//! a-call-is-on [RUNTIMES]
//! ```
//!
//! It exits 0 while a call is on, 1 while none is, and 2 when it can't
//! tell. On Linux it reads each account's PipeWire under RUNTIMES,
//! /run/user unless named (src/pipewire.rs); on a Mac it asks CoreAudio,
//! which answers for every account (src/coreaudio.rs).

#[cfg(target_os = "macos")]
mod coreaudio;
#[cfg(target_os = "linux")]
mod pipewire;

use std::process::ExitCode;

fn main() -> ExitCode {
    match on() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(why) => {
            eprintln!("a-call-is-on: {why}");
            ExitCode::from(2)
        }
    }
}

#[cfg(target_os = "linux")]
fn on() -> Result<bool, String> {
    let runtimes = std::env::args_os()
        .nth(1)
        .map_or_else(|| "/run/user".into(), std::path::PathBuf::from);
    pipewire::on(&runtimes)
}

#[cfg(target_os = "macos")]
fn on() -> Result<bool, String> {
    coreaudio::on()
}
