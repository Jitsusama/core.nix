//! timekeeping says how chrony keeps the clock, for the launcher's Time:
//! how far the clock is from true time and how fast it runs, what it's
//! kept by and when it was last set, the hardware clock's drift, and every
//! server chrony asks, each one's figures and why it's chosen or set aside
//! (VKCF3Z's design, Time's Clock and Servers).
//!
//! It asks chronyd over its command port on the loopback address, as
//! chronyc does, never by running chronyc. systemd starts it when the shell
//! first connects to its socket; it asks chronyd once a second while
//! someone is watching and otherwise waits, asking nothing.
//!
//! It also says what day it is in the Chinese, Japanese, Hebrew and
//! Islamic calendars, for the launcher's Date, reckoned with ICU4X when
//! asked (calendar.rs), and, asked from a place, when the sun sets there
//! to begin the Hebrew and Islamic days (the sun crate, modules/sun).
//!
//! Its unit alone starts it, in the terminal kit's shape, as
//! `timekeeping --help` says (`help()`).

mod calendar;
mod candm;
mod chrony;
mod service;

use std::net::{Ipv4Addr, SocketAddr};
use std::process::ExitCode;

use terminal_kit::{Exit, Failure, Help};
use zlink::Server;

/// What `timekeeping --help` says: how its unit runs it, and how it ends.
fn help() -> Help {
    Help::new(
        "timekeeping",
        "answers the launcher's Time and Date from chrony",
    )
    .example(
        "timekeeping",
        "as its unit runs it, on the socket systemd hands it",
    )
    .exit(Exit::Failed, "its socket couldn't be taken or answered on")
    .exit(Exit::Config, "systemd handed it no socket")
}

/// It runs as long as its socket's unit does, so it only ends by failing.
fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        if !context.args.is_empty() {
            return Err(Failure::new(Exit::Usage, "timekeeping takes no words")
                .because("systemd hands it its socket")
                .try_this("timekeeping --help"));
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| {
                Failure::new(Exit::Failed, "it couldn't start").because(error.to_string())
            })?;
        runtime.block_on(serve())
    })
}

async fn serve() -> Result<(), Failure> {
    let listener = service::listener()
        .map_err(|error| {
            Failure::new(Exit::Failed, "it can't take the socket").because(error.to_string())
        })?
        .ok_or_else(|| {
            Failure::new(Exit::Config, "systemd handed it no socket")
                .because("so nobody can ask anything")
                .try_this("systemctl --user start timekeeping.socket")
        })?;
    let timekeeping = service::Timekeeping {
        chrony: SocketAddr::from((Ipv4Addr::LOCALHOST, candm::PORT)),
    };
    Server::new(listener, timekeeping)
        .run()
        .await
        .map_err(|error| {
            Failure::new(Exit::Failed, "it stopped answering on the socket")
                .because(error.to_string())
        })
}
