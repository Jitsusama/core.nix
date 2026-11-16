//! The broker's command line, in the terminal kit's shape: what
//! `jobs-broker --help` says, each command's help, and the refusal for
//! words it doesn't take. Why each command exists is main.rs's note.

use terminal_kit::{Exit, Failure, Help};

/// What `jobs-broker --help` says, with a help of its own for each
/// command.
pub fn help() -> Help {
    Help::new(
        "jobs-broker",
        "makes a fenced command's own scope in agents' work, and says how units stand",
    )
    .example(
        "jobs-broker run -- nix build .#thing",
        "runs the build in a scope of its own, in its quest's slice",
    )
    .example(
        "jobs-broker status pi-agent-proxy.socket",
        "says how the unit stands, as systemctl show does",
    )
    .subcommand(
        Help::new(
            "run",
            "asks for a scope of its own, then becomes the command in it",
        )
        .example(
            "jobs-broker run --quest 0tud09 -- cargo test",
            "runs the tests in the quest's slice",
        )
        .option(
            "--quest QUEST",
            "the quest whose slice it runs in; the session's",
        )
        .option("--socket PATH", "the broker's socket; the runtime folder's")
        .argument("-- COMMAND...", "the command, and every word after it")
        .passes("the command's own once it runs, or 127 when it can't be run")
        .exit(Exit::Failed, "no scope was made, so the command never ran"),
    )
    .subcommand(
        Help::new("status", "says each unit's standing, a property a line")
            .example(
                "jobs-broker status google.socket slack-unreads.socket",
                "says how both stand",
            )
            .argument("UNIT...", "the units, by name")
            .exit(
                Exit::Failed,
                "the broker couldn't be asked, or a unit couldn't be read",
            ),
    )
    .subcommand(
        Help::new("serve", "answers on the socket, as its unit runs it")
            .example(
                "jobs-broker serve --socket \"$XDG_RUNTIME_DIR/daemons/jobs-broker.sock\"",
                "listens where its unit has it listen",
            )
            .option("--socket PATH", "where it listens; the runtime folder's")
            .exit(Exit::Failed, "it couldn't listen"),
    )
}

/// The flags `run` and `serve` take before their words, each with a
/// value.
pub const FLAGS: [&str; 2] = ["--quest", "--socket"];

/// The refusal for words `command` doesn't take: asked wrongly, 64,
/// saying where its help is.
pub fn wrongly(command: &str, why: impl Into<String>) -> Failure {
    Failure::new(Exit::Usage, why).try_this(format!("{command} --help"))
}
