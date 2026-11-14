//! eero's commands as one tree of resources, as kubectl's are: each is a
//! collection, as `eero devices`, a name after it shows one, as
//! `eero devices Pixel`, and an act sits on what it acts on, as
//! `eero internet sqm on`. Each has help of its own, which the terminal
//! kit answers for `eero devices --help`, `-h` and `eero help devices`.
//! The old spellings (`device`, `node`, `profile`, `radio`, `sqm`,
//! `speedtest` and `watch <view>`) still answer, so nothing that calls
//! them breaks: a singular is listed once beside its collection, and the
//! old acts not at all.

use terminal_kit::{Exit, Help};

/// What `--watch` does, on every view.
const WATCH: &str = "draw it again each time it changes";

/// A view: what it shows, its examples, and the flags and exits every
/// view has.
fn view(name: &str, about: &str, examples: &[(&str, &str)]) -> Help {
    examples
        .iter()
        .fold(Help::new(name, about), |help, (command, words)| {
            help.example(command, words)
        })
        .option("--watch", WATCH)
        .exit(Exit::Unavailable, "eero-daemon isn't running")
}

/// A collection whose word after it names one of it, answering to its
/// singular too.
fn collection(
    name: &str,
    singular: &str,
    about: &str,
    by: &str,
    examples: &[(&str, &str)],
) -> Help {
    view(name, about, examples)
        .alias(singular)
        .argument("<name>", by)
        .exit(Exit::NotFound, &format!("no {singular} goes by that name"))
}

/// What one node or profile holds, read as `eero nodes Study devices`:
/// a view of its own, under the name before it.
fn held(name: &str, about: &str, examples: &[(&str, &str)]) -> Help {
    view(name, about, examples).exit(Exit::NotFound, "nothing goes by that name")
}

/// What a node holds: its devices and its radios.
fn on_a_node() -> [Help; 2] {
    [
        held(
            "devices",
            "the devices on this node now",
            &[
                (
                    "eero nodes Study devices",
                    "each device on the study's node",
                ),
                (
                    "eero nodes Study devices --json",
                    "the same as eero.devices/1",
                ),
            ],
        )
        .alias("device"),
        held(
            "radios",
            "this node's radios, band by band",
            &[
                ("eero nodes Study radios", "each radio on the study's node"),
                ("eero nodes Study radios --json", "the same as eero.radio/1"),
            ],
        )
        .alias("radio"),
    ]
}

/// What a profile holds: its devices.
fn in_a_profile() -> Help {
    held(
        "devices",
        "the devices in this profile, on now or not",
        &[
            ("eero profiles Kids devices", "each device in Kids"),
            (
                "eero profiles Kids devices --json",
                "the same as eero.devices/1",
            ),
        ],
    )
    .alias("device")
}

fn sqm() -> Help {
    Help::new("sqm", "optimize for conferencing and gaming, or not")
        .example("eero internet sqm", "whether it's on now")
        .example("eero internet sqm on", "puts calls and games first")
        .example("eero internet sqm off", "treats all traffic alike")
        .argument(
            "on|off",
            "turn it on or off, on every network; bare, say which",
        )
        .exit(Exit::Failed, "eero refused the change")
        .exit(Exit::Unavailable, "eero's cloud isn't answering")
        .exit(Exit::Refused, "not signed in")
}

fn run() -> Help {
    Help::new("run", "start the eero's own speed test")
        .example("eero speeds run", "tests now; the result shows in a minute")
        .example(
            "eero speeds run --json",
            "the networks tested, as eero.speedtest/1",
        )
        .exit(Exit::Failed, "eero refused")
        .exit(Exit::Unavailable, "eero's cloud isn't answering")
        .exit(Exit::Refused, "not signed in")
}

/// The whole tree, from `eero` down.
pub fn help() -> Help {
    let root = Help::new(
        "eero",
        "the home network's eero, read from its cloud at home or away",
    )
    .example("eero", "the home network at a glance")
    .example(
        "eero nodes Study",
        "one node: backhaul, radios, ports, devices",
    )
    .example(
        "eero devices --watch",
        "every device on now, drawn as it changes",
    )
    .example(
        "eero devices --json",
        "the same for a program, as eero.devices/1",
    );
    resources()
        .into_iter()
        .chain(network())
        .chain(more())
        .chain(old())
        .fold(root, Help::subcommand)
        .exit(
            Exit::Failed,
            "eero refused, or the daemon or a watch stopped",
        )
        .exit(Exit::BadInput, "the record isn't one this eero reads")
        .exit(
            Exit::NotFound,
            "no node, device or profile goes by that name",
        )
        .exit(
            Exit::Unavailable,
            "eero-daemon isn't running, or eero's cloud is down",
        )
        .exit(
            Exit::Refused,
            "not signed in, or no terminal to type the code in",
        )
        .exit(Exit::Config, "XDG_RUNTIME_DIR isn't set")
}

/// The home network's resources, as its help lists them, those it has
/// many of first.
fn resources() -> Vec<Help> {
    let [devices, radios] = on_a_node();
    vec![
        view(
            "status",
            "the home network at a glance",
            &[
                ("eero status", "every headline figure, on a page"),
                ("eero status --json", "the same as eero.status/1"),
            ],
        ),
        collection(
            "nodes",
            "node",
            "every node, or one: backhaul, radios, ports and devices",
            "a node's location or id; bare, every node",
            &[
                (
                    "eero nodes",
                    "every node: state, backhaul, devices, firmware",
                ),
                ("eero nodes Study", "the node in the study, every figure"),
                ("eero nodes Study devices", "the devices on it now"),
            ],
        )
        .subcommand(devices)
        .subcommand(radios),
        view(
            "mesh",
            "the nodes as a tree from the gateway",
            &[
                ("eero mesh", "which node hangs from which, and how"),
                ("eero mesh --json", "the same as eero.mesh/1"),
            ],
        ),
        collection(
            "devices",
            "device",
            "every device on now under its node, or one",
            "a device's name or MAC; bare, every device",
            &[
                ("eero devices", "every device on now, under its node"),
                ("eero devices Pixel", "one device, every figure eero has"),
                ("eero devices --json", "every device as eero.devices/1"),
            ],
        ),
        collection(
            "profiles",
            "profile",
            "each profile, what it blocks and its traffic, or one",
            "a profile's name or id; bare, every profile",
            &[
                ("eero profiles", "each profile and what it blocks"),
                ("eero profiles Kids", "one profile, every figure eero has"),
                ("eero profiles Kids devices", "the devices in it"),
            ],
        )
        .subcommand(in_a_profile()),
    ]
}

/// What the home network holds as one: its way out, speeds, firmware,
/// radios, traffic and events.
fn network() -> Vec<Help> {
    vec![
        view(
            "internet",
            "the way out and every setting that shapes it",
            &[
                ("eero internet", "the way out, its speeds and settings"),
                (
                    "eero internet sqm on",
                    "optimize for conferencing and gaming",
                ),
            ],
        )
        .subcommand(sqm()),
        view(
            "speeds",
            "each speed test eero kept, or a new one",
            &[
                ("eero speeds", "each speed test eero kept"),
                ("eero speeds run", "start a new one"),
            ],
        )
        .subcommand(run()),
        view(
            "firmware",
            "what runs, what's next and when it updates",
            &[
                ("eero firmware", "each node's version and the next"),
                ("eero firmware --json", "the same as eero.firmware/1"),
            ],
        ),
        view(
            "radios",
            "every node's radios, band by band",
            &[
                ("eero radios", "each radio's band, channel and clients"),
                ("eero radios --json", "the same as eero.radio/1"),
            ],
        )
        .alias("radio"),
        view(
            "traffic",
            "bytes by hour, for the home network and each node",
            &[
                ("eero traffic", "the day's bytes, hour by hour"),
                ("eero traffic --json", "the same as eero.traffic/1"),
            ],
        ),
        view(
            "events",
            "what happened, newest first",
            &[
                ("eero events", "what happened, newest first"),
                ("eero events --watch", "the same, as it happens"),
            ],
        ),
    ]
}

/// What isn't the home network's own: signing in, the daemon and zsh's
/// completion.
fn more() -> Vec<Help> {
    vec![
        Help::new("sign-in", "sign in once, typing the code eero sends")
            .example("eero sign-in you@example.com", "eero sends a code to type")
            .example("eero sign-in +15555550100", "the same, by phone")
            .argument("<email or phone>", "what eero knows you by")
            .exit(Exit::Failed, "eero didn't sign you in")
            .exit(Exit::Refused, "no terminal to type the code in"),
        Help::new("daemon", "keep the home network for the shell")
            .example("systemctl --user status eero-daemon", "its unit runs it")
            .exit(Exit::Failed, "the daemon stopped")
            .exit(Exit::Config, "XDG_RUNTIME_DIR isn't set"),
        Help::new("completion", "zsh's completion, as zsh reads it")
            .example("eero completion zsh", "the script zsh reads")
            .argument("zsh", "the shell it's written for"),
        Help::new("names", "what completion offers of a kind")
            .example("eero names devices", "each device's name, as zsh reads it")
            .example("eero names next nodes Study", "what may follow those words")
            .argument("views|nodes|devices|profiles|next", "the kind")
            .argument("<words>...", "for next, the words typed so far")
            .exit(Exit::Unavailable, "eero-daemon isn't running"),
    ]
}

/// The old acts, answered so an old caller still works, and listed
/// nowhere, so the tree reads one way.
fn old() -> Vec<Help> {
    vec![
        sqm().unlisted(),
        Help {
            name: "speedtest".to_owned(),
            ..run().unlisted()
        },
        Help::new("watch", "a view, drawn again each time it changes")
            .argument("<view>...", "the view's words")
            .unlisted(),
    ]
}
