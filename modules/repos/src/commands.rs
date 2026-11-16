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
    // The word after repos is always a name (docs/commands.md), so no
    // listed subcommand stands there. The timers' refresh and count, and
    // the daemon's serve, are unlisted, which the kit matches before a
    // name, and taught by their examples. A repository named any of them
    // is still reached by its key.
    .subcommand(refresh().unlisted())
    .subcommand(count().unlisted())
    .subcommand(serve().unlisted())
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
    .example("repos count", "how far each head is from its branch, now")
    .example(
        "repos serve",
        "the record held ready for the views, as the repos service does",
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

/// The refresh: what `repos-refresh.service` runs on its timer, and what
/// Joel runs for GitHub's part at once.
fn refresh() -> Help {
    Help::new(
        "refresh",
        "ask GitHub after each repository's open changes, as a timer does",
    )
    .example(
        "repos refresh",
        "GitHub's part now, rather than at the timer's next",
    )
    .exit(
        Exit::Unavailable,
        "neither the GitHub broker nor GitHub answered",
    )
    .exit(Exit::Config, "there's nowhere to keep GitHub's part")
}

/// The daemon: what `repos.service` runs, holding the record ready on
/// its socket for the views, read again as git writes.
fn serve() -> Help {
    Help::new(
        "serve",
        "hold the record ready on its socket, split by kind, read again as git writes",
    )
    .example(
        "repos serve",
        "serves at $XDG_RUNTIME_DIR/daemons/repos.sock, or the socket systemd hands it",
    )
    .exit(Exit::Config, "there's nowhere to look, or to serve")
    .exit(
        Exit::Failed,
        "it couldn't listen, or stopped hearing the disk",
    )
}

/// The counting: what `repos-count.service` runs on its timer, and what
/// Joel runs to see how far each head is at once.
fn count() -> Help {
    Help::new(
        "count",
        "count how far each head is behind or ahead of its branch, as a timer does",
    )
    .example("repos count", "how far each head is from its branch, now")
    .exit(Exit::Config, "there's nowhere to keep the counts")
}
