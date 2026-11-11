//! The desktop's list of files opened lately (recent.rs), for a program that
//! doesn't keep it itself and for the launcher that reads it.
//!
//! `recent-files add PROGRAM FILE...` says PROGRAM opened each FILE now,
//! PROGRAM being the name of its desktop entry or command, which also runs
//! it again (`PROGRAM %u`). A FILE may be a `file://` link; whatever else a
//! program was given (its options, a web address, a folder) is passed over,
//! so a program can hand on everything it was started with, and nothing
//! at all is said when none is a file. `recent-files list` prints every local file on
//! the list that's still there, the one opened last first, a line each: the
//! program, the file and when that program last opened it in seconds since
//! 1970, a tab between each, once for each program that opened it, so the
//! launcher can place it among Recent's other sources by when (as
//! recents.nix's `dated` says). `recent-files by PROGRAM...` prints every
//! local file still there that one of PROGRAMs opened, once, the one they
//! opened last first, a line each: the file, when the last of them opened
//! it in microseconds since 1970, and its kind as the list says it, a tab
//! between each, so a program's own lister reads its files lately
//! (inkscape-drawings, which keeps the drawings). A file whose path holds a
//! tab or a line's end can't be said on a line, and isn't.

use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use open_with::recent;

const USAGE: &str =
    "usage: recent-files add PROGRAM FILE... | recent-files list | recent-files by PROGRAM...";

/// Prints what `programs` had lately, as `by` says above.
fn had_by(list: &Path, programs: &[String]) -> Result<(), String> {
    let had = recent::by(list, programs).map_err(|error| error.to_string())?;
    let mut out = io::stdout().lock();
    had.iter()
        .filter(|each| each.path.exists())
        .map(|each| (each, each.path.display().to_string()))
        .filter(|(_, path)| !path.contains(['\t', '\n', '\r']))
        .try_for_each(|(each, path)| writeln!(out, "{path}\t{}\t{}", each.at, each.kind))
        .map_err(|error| error.to_string())
}

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let list = recent::place();
    let done = match arguments.split_first() {
        Some((verb, rest)) if verb == "add" && !rest.is_empty() => {
            let (program, given) = rest.split_first().expect("one or more");
            let paths = recent::files(given, &env::current_dir().unwrap_or_default());
            if paths.is_empty() {
                Ok(())
            } else {
                recent::add(&list, program, &format!("{program} %u"), &paths, None)
                    .map_err(|error| error.to_string())
            }
        }
        Some((verb, [])) if verb == "list" => recent::list(&list)
            .map_err(|error| error.to_string())
            .and_then(|opened| {
                let mut out = io::stdout().lock();
                opened
                    .iter()
                    .filter(|each| each.path.exists())
                    .try_for_each(|each| {
                        let seconds = each.at.div_euclid(1_000_000);
                        writeln!(out, "{}\t{}\t{seconds}", each.program, each.path.display())
                    })
                    .map_err(|error| error.to_string())
            }),
        Some((verb, programs)) if verb == "by" && !programs.is_empty() => had_by(&list, programs),
        _ => Err(USAGE.to_owned()),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("recent-files: {error}");
            ExitCode::FAILURE
        }
    }
}
