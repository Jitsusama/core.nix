//! repos's commands as one tree of resources, as net's and kubectl's are:
//! the command is named for the collection, a name after it shows one, and
//! what belongs to one follows its name (docs/commands.md). The kit answers
//! each one's help for `repos <command> --help`, `-h` and `repos help`.

use terminal_kit::{Exit, Help};

pub fn help() -> Help {
    Help::new(
        "repos",
        "every repository on this machine, each once, with its checkouts and trees",
    )
    .argument(
        "[<name>]",
        "a repository's name in any case, owner/name, or its key",
    )
    .argument(
        "[changes]",
        "after a name: its open changes, with their checks and review",
    )
    .argument("[trees]", "after a name: every tree cut from it")
    .command(
        "refresh",
        "asks GitHub after each repository's open changes, as a timer does",
    )
    .example(
        "repos",
        "each repository as a line: its changes, trees and checkouts",
    )
    .example("repos core.nix", "one repository in full")
    .example(
        "repos core.nix changes",
        "its open changes, updated last first",
    )
    .example("repos core.nix trees", "every tree of it, newest first")
    .example("repos --json", "the whole record, as repos.repositories/1")
    .example(
        "repos refresh",
        "GitHub's part now, rather than at the timer's next",
    )
    .exit(
        Exit::Usage,
        "two repositories go by that name, or the words are unknown",
    )
    .exit(Exit::NotFound, "no repository goes by that name")
    .exit(
        Exit::Unavailable,
        "refresh: neither the GitHub broker nor GitHub answered",
    )
    .exit(
        Exit::Config,
        "there's nowhere to look, or to keep GitHub's part",
    )
}
