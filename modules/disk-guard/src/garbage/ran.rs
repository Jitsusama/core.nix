//! Running Nix's commands for the garbage and generations they clear:
//! what each printed, how it ended, and why it failed, in Nix's own last
//! words with any nix.conf line they quote left out.

use crate::commands::{self, Job, Ran};
use std::ffi::OsStr;
use std::path::Path;
use std::time::Duration;

/// The most of a command's output read: a listing of 12,000 paths is
/// 4 MiB, and a chunk's description about 3.
const OUTPUT_LIMIT: usize = 256 << 20;

/// What a command printed, or why it failed, in its own last words.
pub(crate) fn ran(argv: &[&OsStr], timeout: Duration, idle: bool) -> Result<String, String> {
    match output(argv, timeout, idle, &[])? {
        (Some(0), stdout) => Ok(stdout),
        (_, stderr) => Err(failed(argv, &stderr)),
    }
}

/// How a command ended, by its exit code or none when a signal ended it,
/// with what it printed if it succeeded and what it said on its error
/// stream if not.
pub(super) fn output(
    argv: &[&OsStr],
    timeout: Duration,
    idle: bool,
    input: &[u8],
) -> Result<(Option<i32>, String), String> {
    let name = name(argv);
    let job = Job {
        argv: argv.iter().map(|&arg| arg.to_owned()).collect(),
        cwd: None,
        env: Vec::new(),
        timeout,
        limit: OUTPUT_LIMIT,
        idle,
    };
    match commands::fed(&job, input).map_err(|error| format!("starting {name}: {error}"))? {
        Ran::TimedOut => Err(format!("{name} ran past {timeout:?}")),
        Ran::Stopped => Err(format!("{name} was stopped with the guard")),
        Ran::Finished(output) if output.truncated => Err(format!("{name} said more than it may")),
        Ran::Finished(output) if output.status == Some(0) => {
            let stdout = String::from_utf8(output.stdout);
            Ok((
                Some(0),
                stdout.map_err(|_| format!("{name} printed what isn't text"))?,
            ))
        }
        Ran::Finished(output) => Ok((
            output.status,
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )),
    }
}

/// Why a command failed: its name and the last line it said.
pub(super) fn failed(argv: &[&OsStr], said: &str) -> String {
    format!(
        "{} failed: {}",
        name(argv),
        last_words(said).unwrap_or_default()
    )
}

/// The last line Nix said, with any nix.conf line it quotes left out, or
/// none when it said nothing. Nix ends a refusal of how it was asked, as of
/// a nix.conf line, with "Try '... --help' for more information.", which
/// says nothing of why, so the line before it is told.
pub(crate) fn last_words(said: &str) -> Option<String> {
    let mut lines = said.lines().rev().filter(|line| !line.trim().is_empty());
    let mut last = lines.next()?;
    if last.starts_with("Try '") && last.ends_with("--help' for more information.") {
        last = lines.next().unwrap_or(last);
    }
    Some(unquoted(last))
}

/// What Nix said, with any nix.conf line it quotes told by its length
/// alone: Nix quotes a line it refuses whole, which may hold an access
/// token, and the guard's log is often readable by any account. It quotes
/// one only in its words "configuration line '...' in", followed by the
/// file.
fn unquoted(said: &str) -> String {
    const QUOTED: &str = "configuration line '";
    let Some(start) = said.find(QUOTED).map(|at| at + QUOTED.len()) else {
        return said.to_owned();
    };
    let end = said[start..]
        .rfind("' in ")
        .map_or(said.len(), |at| start + at);
    format!(
        "{}<{} bytes left out>{}",
        &said[..start],
        end - start,
        &said[end..]
    )
}

/// A command's program, by its file's name.
pub(super) fn name(argv: &[&OsStr]) -> String {
    let program = Path::new(argv[0]).file_name().unwrap_or_default();
    program.to_string_lossy().into_owned()
}
