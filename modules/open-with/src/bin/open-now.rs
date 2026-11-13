//! What a program has open now (open.rs), for its Recent in the launcher.
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit. The lines below are read by the launcher, so
//! they stay as they are; with `--json`, each answers as JSON instead.
//!
//! `open-now zathura` prints each zathura with a document open, a line
//! each: its process id, a tab, the document's whole path, two tabs, since
//! the launcher needs nothing to reach it inside its window, the page it's
//! on, counted from one, a tab and how many pages it has.
//!
//! `open-now neovim` prints what each Neovim in a kitty window holds, a line
//! each: `@` and niri's window id, a tab, what `open-now neovim go` is
//! given to bring that Neovim forward inside kitty, a tab, where its server
//! listens, a tab, and what it said it holds as one line of JSON (its
//! process, folder, tabs and files, as ../neovim-held.lua says them), for
//! Programs › Neovim in the launcher (../../../neovim/Neovim.js reads it).
//! Each Neovim is asked at once, as each kitty is, and one that doesn't
//! answer, or whose server is another process's, is passed over.
//!
//! `open-now neovim at <server>` prints what the one Neovim listening there
//! holds, as that JSON alone, for asking one by hand.
//!
//! `open-now neovim go <pid>:<window> [<server> <buffer>]` brings that
//! Neovim's kitty window forward inside kitty, which it does before the
//! launcher focuses niri's window, and then, given its server and a
//! buffer, that buffer inside Neovim, never starting another.
//!
//! `open-now kitty` prints what each kitty niri has a window of holds, a line
//! each: its process id, a tab, and its remote control's `ls` as one line
//! of JSON, every OS window with its tabs and each tab's windows, each
//! process in front given its process group (`pgrp`), for
//! Programs › kitty in the launcher (../../../kitty/Kitty.js reads it). It's
//! one process asking every kitty over its socket, rather than the shell
//! starting a `kitten @ ls` for each of dozens.
//!
//! With none open, each prints nothing, and answers an empty list as JSON.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;

use open_with::{neovim, open};
use serde_json::{Value, json};
use terminal_kit::{Context, Exit, Failure, Help};
use zbus::Connection;

fn help() -> Help {
    Help::new("open-now", "what a program has open now, for the launcher")
        .command("zathura", "each document a zathura has open")
        .command("neovim", "what each Neovim in a kitty window holds")
        .command("neovim at SERVER", "what the Neovim at SERVER holds")
        .command("neovim go PID:WINDOW", "brings it forward in kitty")
        .command("kitty", "what each kitty holds")
        .example("open-now zathura", "a line each: pid, path, page, pages")
        .example(
            "open-now neovim go 81:4 S 3",
            "kitty 81's window 4, then S's buffer 3",
        )
        .example("open-now neovim --json", "each Neovim's holdings, as JSON")
        .exit(Exit::Failed, "its lines couldn't be said")
        .exit(Exit::NotFound, "the buffer is gone from that Neovim")
        .exit(
            Exit::Unavailable,
            "niri, D-Bus, a kitty or a Neovim didn't answer",
        )
}

fn asked_wrongly(what: &str) -> Failure {
    Failure::new(Exit::Usage, what).try_this("open-now --help")
}

fn unsaid(error: &io::Error) -> Failure {
    Failure::new(Exit::Failed, "its lines couldn't be said").because(error.to_string())
}

fn unanswered(who: &str, error: &impl ToString) -> Failure {
    Failure::new(Exit::Unavailable, format!("{who} didn't answer")).because(error.to_string())
}

/// Says `lines` as text, or `answer` as JSON, as the context was asked.
fn say(context: &Context, answer: &Value, lines: &[String]) -> Result<(), Failure> {
    if context.format().is_json() {
        return context.answer(answer).map_err(|error| unsaid(&error));
    }
    let mut out = io::stdout().lock();
    lines
        .iter()
        .try_for_each(|line| writeln!(out, "{line}"))
        .map_err(|error| unsaid(&error))
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let words: Vec<&str> = context.args.iter().map(String::as_str).collect();
        match words.as_slice() {
            ["zathura"] => zathura(context),
            ["neovim"] => neovim(context),
            ["kitty"] => kitty(context),
            ["neovim", "at", server] => neovim_at(context, server),
            ["neovim", "go", id] => neovim_go(context, id, None),
            ["neovim", "go", id, server, buffer] => neovim_go(context, id, Some((server, buffer))),
            ["neovim", ..] => Err(asked_wrongly("neovim takes at SERVER or go PID:WINDOW")),
            [program, ..] => Err(terminal_kit::flags::unknown(
                "program",
                program,
                &["zathura", "neovim", "kitty"],
            )),
            [] => Err(asked_wrongly(
                "open-now needs a program: zathura, neovim or kitty",
            )),
        }
    })
}

/// zathura's figures are 0 where it didn't say one, and JSON says null.
fn figure(count: u32) -> Value {
    if count == 0 {
        Value::Null
    } else {
        json!(count)
    }
}

fn zathura(context: &Context) -> Result<(), Failure> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            Failure::new(Exit::Failed, "it couldn't start").because(error.to_string())
        })?;
    let open = runtime
        .block_on(async {
            let connection = Connection::session().await?;
            open::zathura(&connection).await
        })
        .map_err(|error| unanswered("D-Bus", &error))?;
    let answer = json!({
        "schema": "open-now.zathura/1",
        "open": open.iter().map(|one| json!({
            "pid": one.pid,
            "path": one.file,
            "page": figure(one.page),
            "pages": figure(one.pages),
        })).collect::<Vec<_>>(),
    });
    say(
        context,
        &answer,
        &open.iter().map(open::InZathura::line).collect::<Vec<_>>(),
    )
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
fn kitties() -> Result<(Vec<Value>, Kitties), Failure> {
    let niri = env::var_os("NIRI_SOCKET").ok_or_else(|| {
        Failure::new(Exit::Unavailable, "niri isn't there to ask")
            .because("NIRI_SOCKET isn't set")
            .try_this("open-now from inside a niri session")
    })?;
    let windows =
        open::niri_windows(&PathBuf::from(niri)).map_err(|error| unanswered("niri", &error))?;
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

/// What each Neovim in a kitty window holds, asked of each at once, since
/// one may be busy for a moment and the rest needn't wait on it.
fn neovim(context: &Context) -> Result<(), Failure> {
    let (windows, kitties) = kitties()?;
    let found = open::neovims(&windows, &kitties);
    let held: Vec<_> = thread::scope(|scope| {
        let asked: Vec<_> = found
            .iter()
            .map(|one| scope.spawn(move || neovim::held(Path::new(&one.server)).ok()))
            .collect();
        asked
            .into_iter()
            .map(|one| one.join().ok().flatten())
            .collect()
    });
    let answering: Vec<_> = found
        .iter()
        .zip(held)
        .filter_map(|(one, held)| Some((one, held?)))
        .filter(|(one, held)| one.answers(held))
        .collect();
    let answer = json!({
        "schema": "open-now.neovim/1",
        "open": answering.iter().map(|(one, held)| json!({
            "window": one.window,
            "go": one.go,
            "server": one.server,
            "held": held,
        })).collect::<Vec<_>>(),
    });
    let lines: Vec<_> = answering
        .iter()
        .map(|(one, held)| format!("@{}\t{}\t{}\t{held}", one.window, one.go, one.server))
        .collect();
    say(context, &answer, &lines)
}

fn kitty(context: &Context) -> Result<(), Failure> {
    let (_, mut kitties) = kitties()?;
    let group = |pid: u64| open::group_of(&fs::read_to_string(format!("/proc/{pid}/stat")).ok()?);
    for (_, listed) in &mut kitties {
        open::with_groups(listed, group);
    }
    let answer = json!({
        "schema": "open-now.kitty/1",
        "kitties": kitties.iter().map(|(pid, listed)| json!({"pid": pid, "ls": listed}))
            .collect::<Vec<_>>(),
    });
    let lines: Vec<_> = kitties
        .iter()
        .map(|(pid, listed)| format!("{pid}\t{listed}"))
        .collect();
    say(context, &answer, &lines)
}

/// What the one Neovim at `server` holds.
fn neovim_at(context: &Context, server: &str) -> Result<(), Failure> {
    let held =
        neovim::held(Path::new(server)).map_err(|error| unanswered("that Neovim", &error))?;
    let answer = json!({"schema": "open-now.held/1", "held": held});
    say(context, &answer, &[held.to_string()])
}

/// Brings a Neovim's kitty window forward inside kitty, then, given its
/// server and a buffer, the buffer inside it.
fn neovim_go(context: &Context, id: &str, inside: Option<(&str, &str)>) -> Result<(), Failure> {
    let (pid, window) = id
        .split_once(':')
        .and_then(|(pid, window)| Some((pid.parse::<u32>().ok()?, window.parse::<u64>().ok()?)))
        .ok_or_else(|| asked_wrongly(&format!("{id} isn't a PID:WINDOW")))?;
    let buffer = inside
        .map(|(server, buffer)| buffer.parse::<u64>().map(|buffer| (server, buffer)))
        .transpose()
        .map_err(|_| asked_wrongly("a BUFFER is a number"))?;
    open::kitty(
        &socket_of(pid),
        "focus-window",
        &json!({ "match": format!("id:{window}") }),
    )
    .map_err(|error| unanswered("kitty", &error))?;
    if let Some((server, buffer)) = buffer {
        match neovim::go(Path::new(server), buffer) {
            Ok(true) => {}
            Ok(false) => {
                return Err(Failure::new(
                    Exit::NotFound,
                    format!("buffer {buffer} is gone from that Neovim"),
                ));
            }
            Err(error) => return Err(unanswered("that Neovim", &error)),
        }
    }
    if context.format().is_json() {
        let answer = json!({
            "schema": "open-now.went/1",
            "window": window,
            "buffer": buffer.map(|(_, buffer)| buffer),
        });
        return context.answer(&answer).map_err(|error| unsaid(&error));
    }
    Ok(())
}
