//! `chrome-session [PROFILE]` prints what Chrome has open now, read from
//! the newest session file of PROFILE, Chrome's Default profile when none
//! is given, a tab a line: nothing where an id would be, since no command
//! can act on a tab by the file's, its window, whether it's in front
//! there, when it was last looked at in milliseconds since 1970, its
//! address and its title, a tab between each, as the programs daemon
//! holds them. A page with a program of its own, or off the web, keeps
//! its title alone. It's how a person, or the VM test reading a real
//! Chromium's file (tests/chrome-extension.nix), reads what the daemon
//! reads.
//!
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit. The VM test parses those lines through a
//! pipe, so there they stay as they are; into a terminal they're a table
//! fitting its width, and with `--json` they're the `tabs` of a
//! `chrome-session.tabs/1` object.

use std::env;
use std::io::{self, Write as _};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use chrome_session::{
    host,
    session::{self, Open},
};
use serde_json::{Value, json};
use terminal_kit::layout::{self, Align};
use terminal_kit::time::span;
use terminal_kit::{Context, Exit, Failure, Help, Room};

/// The table a terminal is shown: when each tab was looked at, its
/// window, whether it's in front there, its site and its title.
const COLUMNS: [(&str, Align); 5] = [
    ("looked", Align::Left),
    ("window", Align::Right),
    ("front", Align::Left),
    ("site", Align::Left),
    ("title", Align::Left),
];

/// What goes first where the table is too wide: the site, which the title
/// mostly names, then whether it's in front, then its window.
const GIVES_WAY: [usize; 3] = [3, 2, 1];

fn help() -> Help {
    Help::new("chrome-session", "what Chrome has open now, a tab a line")
        .command(
            "[PROFILE]",
            "reads the newest session of PROFILE, or Chrome's Default",
        )
        .example(
            "chrome-session",
            "the tabs Chrome's Default profile has open",
        )
        .exit(Exit::Failed, "the tabs couldn't be written")
        .exit(Exit::BadInput, "the session file isn't one it reads")
        .exit(Exit::Config, "no PROFILE given, and no HOME to find one in")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let profile = match context.args.as_slice() {
            [] => default_profile()?,
            [profile] => PathBuf::from(profile),
            _ => {
                return Err(
                    Failure::new(Exit::Usage, "chrome-session takes one profile at most")
                        .try_this("chrome-session --help"),
                );
            }
        };
        let open = session::read(&profile).map_err(|error| {
            Failure::new(Exit::BadInput, "Chrome's session couldn't be read").because(error)
        })?;
        said(context, &open).map_err(|error| {
            Failure::new(Exit::Failed, "the tabs couldn't be written").because(error.to_string())
        })
    })
}

/// Chrome's Default profile, under HOME.
fn default_profile() -> Result<PathBuf, Failure> {
    let home = env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .ok_or_else(|| {
            Failure::new(Exit::Config, "there's no HOME to find Chrome's profile in")
                .try_this("chrome-session ~/.config/google-chrome/Default")
        })?;
    Ok(PathBuf::from(home).join(".config/google-chrome/Default"))
}

/// Says the tabs: a line each, as the daemon holds them, or the same as a
/// `chrome-session.tabs/1` object under `--json`.
fn said(context: &Context, open: &[Open]) -> io::Result<()> {
    if context.format().is_json() {
        let tabs: Vec<Value> = open
            .iter()
            .map(|tab| {
                json!({
                    "window": tab.window,
                    "front": tab.front,
                    "last": tab.last,
                    "address": tab.address,
                    "title": tab.title,
                })
            })
            .collect();
        return context.answer(&json!({ "schema": "chrome-session.tabs/1", "tabs": tabs }));
    }
    let mut out = io::stdout().lock();
    if context.room == Room::Unbounded {
        return open
            .iter()
            .try_for_each(|tab| writeln!(out, "{}", tab.line()));
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis());
    let rows: Vec<Vec<String>> = open
        .iter()
        .map(|tab| {
            vec![
                looked(tab.last, now),
                tab.window.to_string(),
                if tab.front { "front" } else { "" }.to_owned(),
                host(&tab.address).unwrap_or_default().to_owned(),
                tab.title.clone(),
            ]
        })
        .collect();
    // A title is prose, so the table is set at a reading measure however
    // wide the terminal.
    let room = context.room.for_prose();
    let lines = layout::table_in(&COLUMNS, &rows, &context.out, room, &GIVES_WAY);
    out.write_all(layout::joined(vec![lines]).as_bytes())
}

/// How long ago a tab was looked at, `last` milliseconds after 1970, or
/// that Chrome never said.
fn looked(last: i64, now: u128) -> String {
    let Ok(last) = u128::try_from(last) else {
        return "never".to_owned();
    };
    if last == 0 {
        return "never".to_owned();
    }
    let seconds = now.saturating_sub(last) / 1000;
    format!("{} ago", span(u64::try_from(seconds).unwrap_or(u64::MAX)))
}
