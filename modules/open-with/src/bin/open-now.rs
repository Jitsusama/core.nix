//! What a program has open now (open.rs), for its Recent in the launcher.
//!
//! `open-now zathura` prints each zathura with a document open, a line
//! each: its process id, a tab and the document's whole path.
//!
//! `open-now neovim` prints each file a Neovim has open in a kitty window, a
//! line each: `@` and niri's window id, a tab, the file's whole path, a tab,
//! and what `open-now neovim go` is given to bring that Neovim forward
//! inside kitty, which it does before the launcher focuses niri's window.
//!
//! With none open, either prints nothing.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use open_with::open;
use serde_json::{Value, json};
use zbus::Connection;

const USAGE: &str = "usage: open-now zathura | open-now neovim [go <pid>:<window>]";

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let done = match arguments.as_slice() {
        [program] if program == "zathura" => zathura().await,
        [program] if program == "neovim" => neovim(),
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
        .try_for_each(|(pid, file)| writeln!(out, "{pid}\t{file}"))
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

/// Each kitty niri has a window of is asked once; one that doesn't answer
/// is passed over, so the rest are still said.
fn neovim() -> Result<(), String> {
    let niri = env::var_os("NIRI_SOCKET").ok_or("NIRI_SOCKET isn't set")?;
    let windows = open::niri_windows(&PathBuf::from(niri)).map_err(|error| error.to_string())?;
    let mut pids: Vec<u32> = windows
        .iter()
        .filter(|window| window["app_id"] == "kitty")
        .filter_map(|window| window["pid"].as_u64()?.try_into().ok())
        .collect();
    pids.sort_unstable();
    pids.dedup();
    let kitties: Vec<(u32, Value)> = pids
        .into_iter()
        .filter_map(|pid| Some((pid, open::kitty(&socket_of(pid), "ls", &json!({})).ok()?)))
        .collect();
    let mut out = io::stdout().lock();
    open::neovims(&windows, &kitties)
        .iter()
        .try_for_each(|one| writeln!(out, "@{}\t{}\t{}", one.window, one.path, one.go))
        .map_err(|error| error.to_string())
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
