//! Claude Code's status line: the folder, the model and how full the context is.
//!
//! Claude Code runs this after each turn with its session as JSON on standard input, and
//! shows the line it prints. `home.nix` names it in Claude Code's settings with a JSON file
//! of the theme's colours, by what each part of the line is: `folder`, `model`, `context`
//! and `separator`, each written `#rrggbb`.
//!
//! Usage: `claude-statusline COLOURS`
//!
//! `line.rs` makes the line; this reads what it's made from.

mod line;

use std::io::{self, Read};
use std::path::Path;
use std::process::ExitCode;
use std::{env, fs};

use line::{Colours, Session};

/// What this answers when it isn't given the colours.
const MISUSED: u8 = 2;

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let (Some(colours), None) = (arguments.next(), arguments.next()) else {
        eprintln!("Usage: claude-statusline COLOURS");
        return ExitCode::from(MISUSED);
    };
    match shown(Path::new(&colours)) {
        Ok(line) => {
            println!("{line}");
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("claude-statusline: {why}");
            ExitCode::FAILURE
        }
    }
}

/// Give the line for the session on standard input, in the colours in that file.
fn shown(colours: &Path) -> Result<String, String> {
    let written = fs::read_to_string(colours)
        .map_err(|error| format!("can't read {}: {error}", colours.display()))?;
    let colours = Colours::from_json(&written)?;
    let mut session = String::new();
    io::stdin()
        .read_to_string(&mut session)
        .map_err(|error| format!("can't read the session: {error}"))?;
    let session = Session::from_json(&session)?;
    // A transcript not written yet, or gone, shows no context rather than no line.
    let read = session
        .transcript()
        .and_then(|transcript| fs::read_to_string(transcript).ok())
        .and_then(|transcript| line::context_tokens(&transcript));
    Ok(line::status(&session, &colours, read))
}
