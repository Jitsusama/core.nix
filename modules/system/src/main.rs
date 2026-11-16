//! system: how the whole system stands, the machines of the realm, the
//! mycelium, the home network and the code, each from its owner's own
//! command (the crate's lib.rs says why).
//!
//! ```text
//! system          a line for each part and each machine, and where to go deeper
//! system --json   the whole record, as system.view/1
//! system serve    the record held ready on its socket (serve.rs)
//! ```
//!
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit.

mod ask;
mod serve;
mod views;

use std::io;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use system::parts;
use system::record::System;
use terminal_kit::{Context, Exit, Failure, Help};

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), run)
}

fn help() -> Help {
    Help::new(
        "system",
        "how the whole system stands: the machines, the mycelium, the network and the code",
    )
    .example(
        "system",
        "a line for each part and each machine, and where each goes deeper",
    )
    .example("system --json", "the whole record, as system.view/1")
    .example(
        "system serve --socket SOCKET",
        "the record held ready there for the views, read again every 15 s",
    )
    .exit(Exit::Usage, "the words are unknown")
}

fn run(context: &Context) -> Result<(), Failure> {
    if let [serve, rest @ ..] = context.args.as_slice()
        && serve == "serve"
    {
        return serve::run(rest);
    }
    if !context.args.is_empty() {
        return Err(
            Failure::new(Exit::Usage, "system doesn't know those words").try_this("system --help")
        );
    }
    let system = read();
    let written = if context.format().is_json() {
        context.answer(&system.json())
    } else {
        context.say(&views::system(&system, &context.out, context.room))
    };
    written.or_else(|error| match error.kind() {
        // A reader that stopped early, as `head` does, ends the answer.
        io::ErrorKind::BrokenPipe => Ok(()),
        _ => Err(
            Failure::new(Exit::Failed, "system couldn't write its answer")
                .because(error.to_string()),
        ),
    })
}

/// The system as each owner says it now.
pub(crate) fn read() -> System {
    let [machines, health, net, repos]: [_; 4] = ask::all(&[
        &["mycelium", "machines"],
        &["mycelium", "health"],
        &["net", "--json"],
        &["repos", "--json"],
    ])
    .try_into()
    .unwrap_or_else(|_| unreachable!("one answer is given for each command asked"));
    System {
        machine: machine(),
        read_at: terminal_kit::time::rfc3339(now()),
        parts: vec![
            parts::machines(&machines, &health),
            parts::mycelium(&health),
            parts::network(&net),
            parts::code(&repos),
        ],
    }
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        })
}

/// The machine's name as it calls itself, read where the kernel keeps it.
fn machine() -> String {
    ["/proc/sys/kernel/hostname", "/etc/hostname"]
        .iter()
        .find_map(|file| {
            std::fs::read_to_string(file)
                .ok()
                .map(|name| name.trim().to_owned())
                .filter(|name| !name.is_empty())
        })
        .or_else(|| {
            std::process::Command::new("hostname")
                .output()
                .ok()
                .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
                .filter(|name| !name.is_empty())
        })
        .unwrap_or_else(|| "this machine".to_owned())
}
