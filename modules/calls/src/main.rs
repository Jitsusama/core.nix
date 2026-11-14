//! Says whether a call is on, for whatever mustn't cut one, such as an
//! update with nobody at it.
//!
//! ```text
//! a-call-is-on [RUNTIMES]
//! ```
//!
//! It's a question in the terminal kit's shape (`help()`): it exits 0
//! while a call is on and 1 while none is, saying nothing, and 69 when it
//! can't tell. On Linux it reads each account's PipeWire under RUNTIMES,
//! /run/user unless named (src/pipewire.rs); on a Mac it asks Core Audio,
//! which answers for every account (src/coreaudio.rs).

#[cfg(target_os = "macos")]
mod coreaudio;
#[cfg(target_os = "linux")]
mod pipewire;

use std::process::ExitCode;

use terminal_kit::{Exit, Failure, Help};

/// What `a-call-is-on --help` says: how it's asked, and what it answers.
fn help() -> Help {
    Help::new("a-call-is-on", "says whether a call is on, by its exit")
        .example("a-call-is-on", "in every account here")
        .example("a-call-is-on /run/user", "under these runtime folders")
        .argument("[runtimes]", "the accounts' runtime folders, /run/user")
        .exit(Exit::Done, "a call is on")
        .exit(Exit::Failed, "no call is on")
        .exit(Exit::Unavailable, "the sound server couldn't be read")
}

fn main() -> ExitCode {
    terminal_kit::ask(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let runtimes = match context.args.as_slice() {
            [] => None,
            [runtimes] => Some(runtimes.as_str()),
            _ => {
                return Err(
                    Failure::new(Exit::Usage, "a-call-is-on takes one folder at most")
                        .try_this("a-call-is-on --help"),
                );
            }
        };
        on(runtimes).map_err(|why| {
            Failure::new(Exit::Unavailable, "it couldn't tell whether a call is on").because(why)
        })
    })
}

#[cfg(target_os = "linux")]
fn on(runtimes: Option<&str>) -> Result<bool, String> {
    pipewire::on(std::path::Path::new(runtimes.unwrap_or("/run/user")))
}

#[cfg(target_os = "macos")]
fn on(_runtimes: Option<&str>) -> Result<bool, String> {
    coreaudio::on()
}
