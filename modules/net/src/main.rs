//! net: the whole home network, read from this machine.
//!
//! A device is one box, and its roles are what it does: modem, router,
//! access point, switch or client (model.rs, docs/glossary.md). One eero
//! is one device with the router and access point roles. Each vendor is a
//! source behind those roles: eero's record, which the eero daemon keeps,
//! the cable modem's status pages, and the switch's web page. No source is
//! a command of its own here, and no vendor's command reads another
//! vendor's equipment.
//!
//! The daemon reads each source in turn and keeps one record,
//! `$XDG_RUNTIME_DIR/net/network.json` (`net.network/1`), and the views
//! read only that record, so a view is instant and says how old what it
//! shows is.
//!
//! ```text
//! net daemon                      keep the home network's record
//! ```
//!
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit.

mod commands;
pub mod model;

use std::process::ExitCode;

use terminal_kit::{Context, Exit, Failure};

fn main() -> ExitCode {
    terminal_kit::main(&commands::help(), env!("CARGO_PKG_VERSION"), run)
}

/// Runs what the words ask, which the kit has walked down net's tree
/// (commands.rs).
fn run(context: &Context) -> Result<(), Failure> {
    let words: Vec<&str> = context.args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["daemon"] => Err(Failure::new(
            Exit::Unavailable,
            "the collector isn't built yet",
        )),
        _ => Err(Failure::new(Exit::Usage, "net doesn't know those words").try_this("net --help")),
    }
}
