//! The spend-watch command: the daemon, its reports and its errands
//! (src/cli.rs lists them).

use std::process::ExitCode;

fn main() -> ExitCode {
    let help = spend_watch::cli::help();
    terminal_kit::main(&help, env!("CARGO_PKG_VERSION"), |context| {
        spend_watch::cli::act(context, &help)
    })
}
