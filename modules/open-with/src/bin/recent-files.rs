//! The desktop's list of files opened lately (recent.rs), for a program that
//! doesn't keep it itself and for the launcher that reads it. It takes the
//! shape every command of ours does (docs/commands.md), through the
//! terminal kit.
//!
//! `recent-files add PROGRAM [--] FILE...` says PROGRAM opened each FILE
//! now, PROGRAM being the name of its desktop entry or command, which also
//! runs it again (`PROGRAM %u`). A FILE may be a `file://` link; whatever
//! else a program was given (its options, a web address, a folder) is
//! passed over, so a program can hand on everything it was started with,
//! after `--` so none of it is taken for this command's own flags, and
//! nothing at all is said when none is a file. `recent-files list` prints
//! every local file on the list that's still there, the one opened last
//! first, a line each: the program, the file and when that program last
//! opened it in seconds since 1970, a tab between each, once for each
//! program that opened it, so the launcher can place it among Recent's
//! other sources by when (as recents.nix's `dated` says). `recent-files by
//! PROGRAM...` prints every local file still there that one of PROGRAMs
//! opened, once, the one they opened last first, a line each: the file,
//! when the last of them opened it in microseconds since 1970, and its kind
//! as the list says it, a tab between each, so a program's own lister reads
//! its files lately (inkscape-drawings, which keeps the drawings). A file
//! whose path holds a tab or a line's end can't be said on a line, and
//! isn't. Those lines are read by programs, so they stay as they are; with
//! `--json`, `list` and `by` answer as JSON instead, with RFC 3339 times,
//! and a path of any kind is said.

use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use open_with::recent;
use serde_json::json;
use terminal_kit::{Context, Exit, Failure, Help, time};

fn help() -> Help {
    Help::new("recent-files", "the desktop's list of files opened lately")
        .command("add", "says PROGRAM opened each FILE now")
        .command("list", "every file on it, by the program that opened it")
        .command("by", "the files one of PROGRAMs opened")
        .example("recent-files list", "a line each: program, file, when")
        .example("recent-files by inkscape", "the drawings Inkscape opened")
        .example("recent-files add vi -- a.md", "says vi opened a.md")
        .example("recent-files list --json", "the same, as JSON")
        .exit(
            Exit::Failed,
            "the list couldn't be written, or its lines said",
        )
        .exit(Exit::BadInput, "the list can't be read")
}

fn asked_wrongly(what: &str) -> Failure {
    Failure::new(Exit::Usage, what).try_this("recent-files --help")
}

fn unreadable(error: &recent::Failed) -> Failure {
    Failure::new(
        Exit::BadInput,
        "the list of files opened lately can't be read",
    )
    .because(error.to_string())
}

fn unsaid(error: &io::Error) -> Failure {
    Failure::new(Exit::Failed, "its lines couldn't be said").because(error.to_string())
}

/// Says PROGRAM opened each file among what it was given.
fn add(list: &Path, program: &str, given: &[String]) -> Result<(), Failure> {
    let given = given.strip_prefix(&["--".to_owned()]).unwrap_or(given);
    let paths = recent::files(given, &env::current_dir().unwrap_or_default());
    if paths.is_empty() {
        return Ok(());
    }
    recent::add(list, program, &format!("{program} %u"), &paths, None).map_err(|error| {
        Failure::new(
            Exit::Failed,
            "the list of files opened lately couldn't be written",
        )
        .because(error.to_string())
    })
}

/// Prints every file still there, as `list` says above.
fn list(context: &Context, list: &Path) -> Result<(), Failure> {
    let opened = recent::list(list).map_err(|error| unreadable(&error))?;
    let mut there = opened.iter().filter(|each| each.path.exists());
    if context.format().is_json() {
        let files: Vec<_> = there
            .map(|each| {
                json!({
                    "program": each.program,
                    "path": each.path.to_string_lossy(),
                    "opened_at": time::rfc3339(each.at.div_euclid(1_000_000)),
                })
            })
            .collect();
        return context
            .answer(&json!({"schema": "recent-files.list/1", "files": files}))
            .map_err(|error| unsaid(&error));
    }
    let mut out = io::stdout().lock();
    there
        .try_for_each(|each| {
            let seconds = each.at.div_euclid(1_000_000);
            writeln!(out, "{}\t{}\t{seconds}", each.program, each.path.display())
        })
        .map_err(|error| unsaid(&error))
}

/// Prints what `programs` had lately, as `by` says above.
fn had_by(context: &Context, list: &Path, programs: &[String]) -> Result<(), Failure> {
    let had = recent::by(list, programs).map_err(|error| unreadable(&error))?;
    let there = had.iter().filter(|each| each.path.exists());
    if context.format().is_json() {
        let files: Vec<_> = there
            .map(|each| {
                json!({
                    "path": each.path.to_string_lossy(),
                    "opened_at": time::rfc3339(each.at.div_euclid(1_000_000)),
                    "kind": each.kind,
                })
            })
            .collect();
        return context
            .answer(&json!({"schema": "recent-files.by/1", "files": files}))
            .map_err(|error| unsaid(&error));
    }
    let mut out = io::stdout().lock();
    there
        .map(|each| (each, each.path.display().to_string()))
        .filter(|(_, path)| !path.contains(['\t', '\n', '\r']))
        .try_for_each(|(each, path)| writeln!(out, "{path}\t{}\t{}", each.at, each.kind))
        .map_err(|error| unsaid(&error))
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let place = recent::place();
        match context.args.split_first() {
            Some((verb, rest)) if verb == "add" => match rest.split_first() {
                Some((program, given)) if !given.is_empty() => add(&place, program, given),
                _ => Err(asked_wrongly("add needs a PROGRAM and its files")),
            },
            Some((verb, [])) if verb == "list" => list(context, &place),
            Some((verb, _)) if verb == "list" => Err(asked_wrongly("list takes nothing more")),
            Some((verb, [])) if verb == "by" => Err(asked_wrongly("by needs a PROGRAM")),
            Some((verb, programs)) if verb == "by" => had_by(context, &place, programs),
            Some((verb, _)) => Err(terminal_kit::flags::unknown(
                "command",
                verb,
                &["add", "list", "by"],
            )),
            None => list(context, &place),
        }
    })
}
