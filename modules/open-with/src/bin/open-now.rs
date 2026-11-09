//! What a program has open now (open.rs), for its Recent in the launcher.
//!
//! `open-now zathura` prints each zathura with a document open, a line
//! each: its process id, a tab, the document's whole path, two tabs, since
//! the launcher needs nothing to reach it inside its window, the page it's
//! on, counted from one, a tab and how many pages it has.
//!
//! `open-now neovim` prints each file a Neovim has open in a kitty window, a
//! line each: `@` and niri's window id, a tab, the file's whole path, a tab,
//! and what `open-now neovim go` is given to bring that Neovim forward
//! inside kitty, which it does before the launcher focuses niri's window.
//!
//! `open-now kitty` prints what each kitty niri has a window of holds, a line
//! each: its process id, a tab, and its remote control's `ls` as one line
//! of JSON, every OS window with its tabs and each tab's windows, each
//! process in front given its process group (`pgrp`), for
//! Programs › kitty in the launcher (../../../kitty/Kitty.js reads it). It's
//! one process asking every kitty over its socket, rather than the shell
//! starting a `kitten @ ls` for each of dozens.
//!
//! With none open, each prints nothing.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;

use open_with::open;
use serde_json::{Value, json};
use zbus::Connection;

const USAGE: &str =
    "usage: open-now zathura | open-now neovim [go <pid>:<window>] | open-now kitty";

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let done = match arguments.as_slice() {
        [program] if program == "zathura" => zathura().await,
        [program] if program == "neovim" => neovim(),
        [program] if program == "kitty" => kitty(),
        [program, go, id] if program == "neovim" && go == "go" => neovim_go(id),
        _ => Err(USAGE.to_owned()),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("open-now: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn zathura() -> Result<(), String> {
    let connection = Connection::session()
        .await
        .map_err(|error| error.to_string())?;
    let open = open::zathura(&connection)
        .await
        .map_err(|error| error.to_string())?;
    let mut out = io::stdout().lock();
    open.iter()
        .try_for_each(|one| writeln!(out, "{}", one.line()))
        .map_err(|error| error.to_string())
}

/// Where a kitty listens, from its process's command line.
fn socket_of(pid: u32) -> String {
    let cmdline = fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
    let arguments: Vec<String> = cmdline
        .split(|&byte| byte == 0)
        .map(|argument| String::from_utf8_lossy(argument).into_owned())
        .collect();
    open::kitty_socket(pid, &arguments)
}

/// Each kitty by its process, with its remote control's `ls`.
type Kitties = Vec<(u32, Value)>;

/// niri's windows, and what each kitty it has a window of holds. Each is
/// asked once, all at once, since a kitty takes about a sixth of a second
/// to list what runs in its windows: four took 0.66 s one at a time and
/// 0.23 s together (2026-11-07), and a kitty for each session would keep
/// the launcher waiting seconds. One that doesn't answer is passed over,
/// so the rest are still said.
fn kitties() -> Result<(Vec<Value>, Kitties), String> {
    let niri = env::var_os("NIRI_SOCKET").ok_or("NIRI_SOCKET isn't set")?;
    let windows = open::niri_windows(&PathBuf::from(niri)).map_err(|error| error.to_string())?;
    let kitties = thread::scope(|scope| {
        let asked: Vec<_> = open::kitty_pids(&windows)
            .into_iter()
            .map(|pid| {
                scope.spawn(move || {
                    Some((pid, open::kitty(&socket_of(pid), "ls", &json!({})).ok()?))
                })
            })
            .collect();
        asked
            .into_iter()
            .filter_map(|one| one.join().ok().flatten())
            .collect()
    });
    Ok((windows, kitties))
}

fn neovim() -> Result<(), String> {
    let (windows, kitties) = kitties()?;
    let mut out = io::stdout().lock();
    open::neovims(&windows, &kitties)
        .iter()
        .try_for_each(|one| writeln!(out, "@{}\t{}\t{}", one.window, one.path, one.go))
        .map_err(|error| error.to_string())
}

fn kitty() -> Result<(), String> {
    let (_, mut kitties) = kitties()?;
    let group = |pid: u64| open::group_of(&fs::read_to_string(format!("/proc/{pid}/stat")).ok()?);
    let mut out = io::stdout().lock();
    kitties.iter_mut().try_for_each(|(pid, listed)| {
        open::with_groups(listed, group);
        writeln!(out, "{pid}\t{listed}").map_err(|error| error.to_string())
    })
}

fn neovim_go(id: &str) -> Result<(), String> {
    let (pid, window) = id
        .split_once(':')
        .and_then(|(pid, window)| Some((pid.parse::<u32>().ok()?, window.parse::<u64>().ok()?)))
        .ok_or(USAGE)?;
    open::kitty(
        &socket_of(pid),
        "focus-window",
        &json!({ "match": format!("id:{window}") }),
    )
    .map(|_| ())
    .map_err(|error| error.to_string())
}
