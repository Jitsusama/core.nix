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
//! program, a tab and the file, once for each program that opened it.

use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use open_with::recent;

const USAGE: &str = "usage: recent-files add PROGRAM FILE... | recent-files list";

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
                    .try_for_each(|each| writeln!(out, "{}\t{}", each.program, each.path.display()))
                    .map_err(|error| error.to_string())
            }),
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
