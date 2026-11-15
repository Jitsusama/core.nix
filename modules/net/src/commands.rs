//! net's commands as one tree of resources, as eero's and kubectl's are:
//! each is a collection, a name after it shows one, and an act sits on
//! what it acts on (docs/commands.md). The kit answers each one's help for
//! `net <command> --help`, `-h` and `net help <command>`.

use terminal_kit::{Exit, Help};

pub fn help() -> Help {
    let root = Help::new(
        "net",
        "the home network: every device, what it does, and how they link",
    )
    .example("net daemon", "keep the home network's record for the views");
    [daemon()]
        .into_iter()
        .fold(root, Help::subcommand)
        .exit(Exit::Failed, "the daemon stopped")
        .exit(Exit::Config, "XDG_RUNTIME_DIR isn't set")
}

/// The collector: what `net-daemon.service` runs.
fn daemon() -> Help {
    Help::new(
        "daemon",
        "read every source in turn and keep the record the views draw",
    )
    .example("net daemon", "what net-daemon.service runs")
    .exit(Exit::Failed, "the record can't be written")
    .exit(Exit::Config, "XDG_RUNTIME_DIR isn't set")
}
