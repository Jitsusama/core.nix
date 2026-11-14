//! Claude Code's status line: the folder, the model and how full the context is.
//!
//! Claude Code runs this after each turn with its session as JSON on standard input, and
//! shows the line it prints. Its help, exits and ink are the terminal kit's
//! (docs/commands.md), so it draws in the theme's roles and marks, read at each run; Claude
//! Code reads it through a pipe, so `home.nix` asks for `--color always`.
//!
//! `line.rs` makes the line; this reads what it's made from.

mod line;

use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

use serde_json::json;
use terminal_kit::{Exit, Failure, Help};

use line::Session;

fn help() -> Help {
    Help::new(
        "claude-statusline",
        "Claude Code's status line: folder, model, context",
    )
    .example(
        "claude-statusline --color always < session.json",
        "the line for the session Claude Code gives",
    )
    .exit(Exit::Failed, "the session couldn't be read")
    .exit(Exit::BadInput, "the session isn't JSON")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        if let Some(word) = context.args.first() {
            return Err(Failure::new(Exit::Usage, format!("it takes no {word}"))
                .try_this("claude-statusline --help"));
        }
        let mut written = String::new();
        io::stdin().read_to_string(&mut written).map_err(|error| {
            Failure::new(Exit::Failed, "the session couldn't be read").because(error.to_string())
        })?;
        let session = Session::from_json(&written).map_err(|why| {
            Failure::new(Exit::BadInput, "the session couldn't be read").because(why)
        })?;
        // A transcript not written yet, or gone, shows no context rather than no line.
        let read = session
            .transcript()
            .and_then(|transcript| fs::read_to_string(transcript).ok())
            .and_then(|transcript| line::context_tokens(&transcript));
        if context.format().is_json() {
            context.answer(&json!({
                "schema": "claude-statusline.line/1",
                "folder": session.folder(),
                "model": session.model(),
                "context_tokens": read,
            }))
        } else {
            context.say(&format!("{}\n", line::status(&session, &context.out, read)))
        }
        .map_err(|error| {
            Failure::new(Exit::Failed, "the line couldn't be said").because(error.to_string())
        })
    })
}
