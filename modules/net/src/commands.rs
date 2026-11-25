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
        "whether each source answers, what the devices do, and what's worth a look",
    )
    .example(
        "net devices",
        "every device, what it does and where it's reached",
    )
    .example(
        "net devices Study",
        "one device in full, from each of its sources",
    )
    .example("net --json", "the whole record, as net.network/1")
    .example(
        "net uplink",
        "who filled this machine's uplink over the last 5 s",
    );
    [devices(), uplink(), daemon(), switch_reader()]
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
        .argument(
            "<name>",
            "a device's name, in any case, one of its addresses, or its id",
        )
        .example(
            "net devices",
            "each device as a line: its roles, address and model",
        )
        .example(
            "net devices Study",
            "the study's eero: its fields, ports, radios and links",
        )
        .example("net devices 10.0.0.40", "the device at that address")
        .example("net devices Modem --json", "the same as net.device/1")
        .exit(Exit::NotFound, "no device goes by that name or address")
        .exit(
            Exit::Unavailable,
            "there's no record of the home network yet",
        )
        .exit(Exit::Config, "XDG_RUNTIME_DIR isn't set")
        .subcommand(press())
        .subcommand(wake())
}

/// Press a key on a TV's remote, from this machine.
fn press() -> Help {
    Help::new(
        "press",
        "press a key on a TV's remote; the first time, the TV asks on its screen to allow it",
    )
    .argument(
        "<key>",
        "power, volume-up, volume-down, mute, home, source, back, ok, up, down, left, \
         right, play or pause",
    )
    .example(
        "net devices Living press volume-up",
        "the TV turns its volume up a step",
    )
    .example(
        "net devices Living press power --json",
        "the same for its power key, said as net.pressed/1",
    )
    .exit(Exit::Usage, "it isn't a TV, or it has no such key")
    .exit(Exit::NotFound, "no device goes by that name or address")
    .exit(
        Exit::Refused,
        "the TV refused or nobody pressed Allow, or what answered isn't the TV that allowed it",
    )
    .exit(
        Exit::Unavailable,
        "the TV is asleep or off, or there's no record of the home network yet",
    )
    .exit(Exit::TryAgain, "the TV broke off")
    .exit(
        Exit::Failed,
        "what the TV allowed can't be opened, or what answered doesn't speak as a TV",
    )
    .exit(Exit::Config, "XDG_RUNTIME_DIR isn't set")
}

/// Wake a TV from standby with Wake-on-LAN.
fn wake() -> Help {
    Help::new(
        "wake",
        "wake a TV from standby, waiting about 15 s for it to answer as on",
    )
    .example("net devices Living wake", "the TV comes on from standby")
    .example(
        "net devices Living wake --json",
        "the same, said as net.woken/1",
    )
    .exit(Exit::Usage, "it isn't a TV")
    .exit(Exit::NotFound, "no device goes by that name or address")
    .exit(
        Exit::Unavailable,
        "the TV didn't wake (on Wi-Fi it needs Power On with Mobile), or there's no record yet",
    )
    .exit(Exit::Failed, "the wake couldn't be sent")
    .exit(Exit::Config, "XDG_RUNTIME_DIR isn't set")
}

/// Who fills this machine's uplink: its own sockets, not the network's.
fn uplink() -> Help {
    Help::new(
        "uplink",
        "who filled this machine's uplink over the last 5 s, and what for",
    )
    .example(
        "net uplink",
        "the rate, your share and the agents', what it was for, who and where to",
    )
    .example("net uplink --json", "the same as net.uplink/1")
    .exit(Exit::Failed, "the kernel's socket list can't be read")
}

/// The switch's reader: what the switch's system service runs, holding
/// the password the daemon never sees.
fn switch_reader() -> Help {
    Help::new(
        "switch-reader",
        "read the switch's status pages and report them for the daemon",
    )
    .argument("<address>", "where the switch answers, as 10.0.0.30")
    .example(
        "net switch-reader 10.0.0.30",
        "what net-switch.service runs",
    )
    .exit(Exit::Failed, "the report can't be written")
}

/// The collector: what `net.service` runs.
fn daemon() -> Help {
    Help::new(
        "daemon",
        "read every source in turn, keep the record the views draw and serve it",
    )
    .example("net daemon", "what net.service runs")
    .exit(Exit::Failed, "the record can't be written")
    .exit(Exit::Config, "XDG_RUNTIME_DIR isn't set")
}
