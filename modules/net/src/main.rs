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
//! net                             each source, the devices, and what's worth a look
//! net devices [NAME]              every device, or one in full (views.rs)
//! net daemon                      keep the home network's record
//! net switch-reader ADDRESS       read the switch for the daemon (switch_reader.rs)
//! ```
//!
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit.

mod collect;
mod commands;
mod daemon;
mod history;
pub mod model;
mod room;
mod sources;
mod switch_reader;
mod time;
mod views;
mod xml;

use std::io;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use model::{Network, SCHEMA};
use serde_json::Value;
use terminal_kit::{Context, Exit, Failure};

fn main() -> ExitCode {
    terminal_kit::main(&commands::help(), env!("CARGO_PKG_VERSION"), run)
}

/// Runs what the words ask, which the kit has walked down net's tree
/// (commands.rs).
fn run(context: &Context) -> Result<(), Failure> {
    let words: Vec<&str> = context.args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["daemon"] => daemon::run().map_err(|why| Failure::new(Exit::Failed, why)),
        ["switch-reader", address] => {
            switch_reader::run(address).map_err(|why| Failure::new(Exit::Failed, why))
        }
        [] => {
            let network = record()?;
            show(context, &views::home_json(&network), || {
                views::home(&network, now(), &context.out, context.room)
            })
        }
        ["devices" | "device"] => {
            let network = record()?;
            show(context, &views::devices_json(&network), || {
                views::devices(&network, now(), &context.out, context.room)
            })
        }
        ["devices" | "device", name] => {
            let network = record()?;
            let device = views::find(&network, name)?;
            show(context, &views::device_json(&network, device), || {
                views::device(&network, device, now(), &context.out, context.room)
            })
        }
        _ => Err(Failure::new(Exit::Usage, "net doesn't know those words").try_this("net --help")),
    }
}

/// The answer as JSON when asked for, and otherwise the view laid out.
fn show(context: &Context, answer: &Value, view: impl FnOnce() -> String) -> Result<(), Failure> {
    let written = if context.format().is_json() {
        context.answer(answer)
    } else {
        context.say(&view())
    };
    written.or_else(|error| match error.kind() {
        // A reader that stopped early, as `head` does, ends the answer.
        io::ErrorKind::BrokenPipe => Ok(()),
        _ => {
            Err(Failure::new(Exit::Failed, "net couldn't write its answer")
                .because(error.to_string()))
        }
    })
}

/// The time now, in seconds since the epoch, as the record's times are.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// The record the daemon keeps, which every view reads and nothing else.
/// Its folder comes from the process, since the kit's environment holds
/// only the variables it draws by.
fn record() -> Result<Network, Failure> {
    let runtime = std::env::var("XDG_RUNTIME_DIR")
        .ok()
        .filter(|runtime| !runtime.is_empty())
        .ok_or_else(|| Failure::new(Exit::Config, "XDG_RUNTIME_DIR isn't set"))?;
    let path = PathBuf::from(runtime).join("net/network.json");
    let text = std::fs::read_to_string(&path).map_err(|error| {
        Failure::new(
            Exit::Unavailable,
            "there's no record of the home network yet",
        )
        .because(format!("{}: {error}", path.display()))
        .try_this("systemctl --user status net-daemon")
    })?;
    let unreadable = |why: String| {
        Failure::new(Exit::Unavailable, "the home network's record can't be read").because(why)
    };
    let network: Network = serde_json::from_str(&text)
        .map_err(|error| unreadable(format!("{}: {error}", path.display())))?;
    if network.schema != SCHEMA {
        return Err(unreadable(format!(
            "it's {}, and net reads {SCHEMA}",
            network.schema
        )));
    }
    Ok(network)
}
