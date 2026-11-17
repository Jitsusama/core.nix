//! The sorter's command line, in the terminal kit's shape: what
//! `jobs-sorter --help` says, its two commands' help, and the refusal
//! for words it doesn't take. Each flag's why is in main.rs's own note.

use terminal_kit::{Exit, Failure, Help};

/// Every flag the sorter takes, each with a value, as its units give it.
const FLAGS: [(&str, &str); 25] = [
    (
        "--under SLICE[=CGROUP]",
        "a slice of terminals it watches, and where their busy work goes; app.slice",
    ),
    ("--home PREFIX", "a terminal's scope by its prefix; kitty-"),
    ("--keep NAME", "a group leader never moved whole, as pi"),
    (
        "--jobs CGROUP",
        "where busy work goes when --under names none; jobs.slice/jobs.service",
    ),
    ("--busy MS", "the CPU a second that counts as busy; 500"),
    (
        "--ticks COUNT",
        "the seconds running it's busy before it moves; 2",
    ),
    ("--every MS", "how often it looks; 1000"),
    (
        "--split CGROUP[=SLICE]",
        "a cgroup whose every tree of work goes in a scope of its own",
    ),
    (
        "--no-real-time CGROUP",
        "a cgroup whose work runs without real time, and goes first when memory runs out",
    ),
    (
        "--by-quest SLICE",
        "a slice of split work that goes in its quest's slice",
    ),
    ("--quests FILE", "each quest's weight"),
    ("--tiers FILE", "each quest's tier, with --parents"),
    (
        "--parents FILE",
        "where each tier's slice sits, with --tiers",
    ),
    (
        "--tier-order NAMES",
        "the tiers, first to last, split at commas",
    ),
    (
        "--background FOLDER",
        "where each pi nobody is at is marked, for its window to yield",
    ),
    ("--low-weight WEIGHT", "a yielding window's weight; 10"),
    (
        "--weigh PREFIX",
        "a session's scope by its prefix, weighed as a window",
    ),
    (
        "--sessions SLICE",
        "where a session found in jobs goes back to, with --weigh",
    ),
    (
        "--hosts PREFIX",
        "a pi host's unit by its prefix, ranked as its sessions",
    ),
    (
        "--revive CGROUP",
        "a cgroup that gives memory back when the kernel's reclaim gives up",
    ),
    (
        "--door CGROUP",
        "a cgroup whose new Nix work waits while memory is short",
    ),
    (
        "--calls PW-DUMP",
        "the pw-dump that hears a call, which the door keeps room for",
    ),
    (
        "--calls-elsewhere SOCKETS=MYCELIUM",
        "the mycelium nodes' sockets, and mycelium, that hear a call on another machine",
    ),
    (
        "--niri NIRI",
        "the niri whose focus ranks the windows Joel is at",
    ),
    (
        "--orchestrators FOLDER",
        "the mesh's presence, whose orchestrators' windows end last",
    ),
];

/// What `jobs-sorter --help` says: the daemon, its flags, and its two
/// commands.
pub fn help() -> Help {
    let sorter = Help::new(
        "jobs-sorter",
        "moves a terminal's busy work into the jobs slice, so what's beside it keeps its CPU",
    )
    .example(
        "jobs-sorter --under app.slice --keep pi",
        "sorts the apps' terminals, never moving pi whole",
    )
    .subcommand(
        Help::new(
            "probe",
            "wakes as an app would, so the kernel counts its wait for a processor",
        )
        .example(
            "jobs-sorter probe --every 20",
            "wakes fifty times a second, until it's stopped",
        )
        .option("--every MS", "how often it wakes; 100")
        // It wakes until it's stopped, so it never ends done.
        .never_done(),
    )
    .subcommand(
        Help::new(
            "swap-in",
            "reads back the swapped memory of a focused window's process",
        )
        .example(
            "jobs-sorter swap-in PID",
            "as root, becoming the process's own account",
        )
        .argument("PID", "the window's process")
        .option("--owned", "as the process's own account already")
        .exit(
            Exit::Failed,
            "the process is root's, whose memory isn't read back",
        ),
    )
    .exit(Exit::Failed, "it couldn't read or sort the cgroups");
    FLAGS
        .iter()
        .fold(sorter, |help, (flag, does)| help.option(flag, does))
}

/// The refusal for a word `command` doesn't take, saying where to look.
pub fn wrongly(command: &str, why: impl Into<String>) -> Failure {
    Failure::new(Exit::Usage, why).try_this(format!("{command} --help"))
}

/// The flags' names alone, for the nearest to a mistyped one.
pub fn flags() -> Vec<&'static str> {
    FLAGS
        .iter()
        .filter_map(|(flag, _)| flag.split(' ').next())
        .collect()
}
