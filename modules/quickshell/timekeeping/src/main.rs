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

mod calendar;
mod candm;
mod chrony;
mod service;

use std::net::{Ipv4Addr, SocketAddr};
use std::process::ExitCode;

use zlink::Server;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let listener = match service::listener() {
        Ok(Some(listener)) => listener,
        Ok(None) => {
            println!("<3>systemd passed no socket, so nobody can ask anything");
            return ExitCode::FAILURE;
        }
        Err(error) => {
            println!("<3>can't take the socket: {error}");
            return ExitCode::FAILURE;
        }
    };
    let timekeeping = service::Timekeeping {
        chrony: SocketAddr::from((Ipv4Addr::LOCALHOST, candm::PORT)),
    };
    match Server::new(listener, timekeeping).run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            println!("<3>stopped answering on the socket: {error}");
            ExitCode::FAILURE
        }
    }
}
