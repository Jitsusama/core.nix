//! What changed-checks and keep-main say they are, take and end with, as
//! `--help` shows it (docs/commands.md), and the option names a mistyped
//! one is matched against.

use terminal_kit::{Exit, Help};

/// The options a run that compares takes, each with its value, and what
/// it does.
const OPTIONS: [(&str, &str); 8] = [
    (
        "--base REF",
        "compares with REF's merge base; origin's main unless named",
    ),
    (
        "--system SYSTEM",
        "compares that system's checks; the one with the most unless named",
    ),
    ("--build", "builds the VM tests that moved too"),
    ("--no-vm", "with --build, still leaves the VM tests out"),
    ("--name-only", "names what moved and builds nothing"),
    (
        "--keep-base",
        "only keeps the base's derivations, on a builder that stands",
    ),
    (
        "--here",
        "runs on this machine, rather than sending the run elsewhere",
    ),
    (
        "--first",
        "goes ahead of runs waiting for a builder; the net's alone",
    ),
];

/// The options' names, as typed: each without its value.
pub fn option_names() -> Vec<&'static str> {
    OPTIONS
        .iter()
        .filter_map(|(name, _)| name.split(' ').next())
        .collect()
}

/// changed-checks' help, with keep-main's under it.
pub fn help() -> Help {
    let help = Help::new(
        "changed-checks",
        "names the checks a change touches, by their derivations, and builds those",
    )
    .example(
        "changed-checks",
        "builds what the committed change moved, VM tests aside, on a builder",
    )
    .example(
        "changed-checks --name-only",
        "names what moved, building nothing",
    )
    .example(
        "changed-checks --base origin/main --system x86_64-linux",
        "compares Linux's checks with main's",
    );
    OPTIONS
        .iter()
        .fold(help, |help, (name, words)| help.option(name, words))
        .exit(
            Exit::Failed,
            "every build ran to its end, and a check it touched failed",
        )
        .exit(Exit::Usage, "asked wrongly, or the base shares no history")
        .exit(Exit::BadInput, "a check it touched no longer evaluates")
        .exit(
            Exit::Bug,
            "a build was cut short, or the run couldn't go on",
        )
        .exit(
            Exit::TryAgain,
            "nowhere took the run, so nothing ran; ask again later",
        )
        .passes("the far side's own code, when the run sent there fails")
        .subcommand(keep_main())
}

fn keep_main() -> Help {
    Help::new(
        "keep-main",
        "keeps each head of origin's main where runs are sent, until stopped",
    )
    .example(
        "changed-checks keep-main --follow ~/.nix-profile/bin/changed-checks",
        "ends between keeps once another build is installed there",
    )
    .option(
        "--follow PATH",
        "ends between keeps once PATH is another changed-checks",
    )
    .exit(Exit::Done, "another changed-checks was installed at PATH")
}
