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
    .example(
        "net",
        "whether each source answers, and what the devices do",
    )
    .example(
        "net devices",
        "every device, what it does and where it's reached",
    )
    .example(
        "net devices Study",
        "one device in full, from each of its sources",
    )
    .example("net --json", "the whole record, as net.network/1");
    [devices(), daemon()]
        .into_iter()
        .fold(root, Help::subcommand)
        .exit(Exit::Failed, "the daemon stopped")
        .exit(
            Exit::Unavailable,
            "there's no record of the home network yet",
        )
        .exit(Exit::Config, "XDG_RUNTIME_DIR isn't set")
}

/// Every device, and one by its name: a collection answering to its
/// singular too, as eero's are.
fn devices() -> Help {
    Help::new("devices", "every device, or one in full")
        .alias("device")
        .argument("<name>", "a device's name, in any case, or its id")
        .example(
            "net devices",
            "each device as a line: its roles, address and model",
        )
        .example(
            "net devices Study",
            "the study's eero: its fields, ports, radios and links",
        )
        .example("net devices Modem --json", "the same as net.device/1")
        .exit(Exit::NotFound, "no device goes by that name")
        .exit(
            Exit::Unavailable,
            "there's no record of the home network yet",
        )
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
