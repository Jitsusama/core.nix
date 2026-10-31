//! update-to-main's command (src/lib.rs says what it's for).
//!
//! - `update-to-main serve --config FILE`: root's daemon, on the config's
//!   socket.
//! - `update-to-main ask [--socket FILE]`: asks for an update and prints
//!   each line as it comes, as JSON. It exits 0 when the machine runs main
//!   or already did, 1 when the update failed, 2 when it was refused and 3
//!   when the daemon couldn't be asked or stopped answering.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use update_to_main::config::Config;
use update_to_main::line::Line;
use update_to_main::{ask, serve};

/// Where the daemon listens unless the config says otherwise, so an asker
/// needn't be told.
const SOCKET: &str = if cfg!(target_os = "macos") {
    "/var/run/update-to-main.sock"
} else {
    "/run/update-to-main/ask.sock"
};

const USAGE: &str =
    "usage: update-to-main serve --config FILE\n       update-to-main ask [--socket FILE]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["serve", "--config", file] => {
            match Config::read(&PathBuf::from(file)).and_then(serve::serve) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("update-to-main: {e}");
                    ExitCode::from(3)
                }
            }
        }
        ["ask"] => asked(Path::new(SOCKET)),
        ["ask", "--socket", file] => asked(Path::new(file)),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(64)
        }
    }
}

fn asked(socket: &Path) -> ExitCode {
    let ending = ask::ask(socket, &mut |line| println!("{}", line.json()));
    match ending {
        Ok(Line::Done { .. } | Line::Unchanged { .. }) => ExitCode::SUCCESS,
        Ok(Line::Failed { .. }) => ExitCode::from(1),
        Ok(Line::Refused { .. }) => ExitCode::from(2),
        Ok(_) => ExitCode::from(3),
        Err(e) => {
            eprintln!("update-to-main: {}: {e}", socket.display());
            ExitCode::from(3)
        }
    }
}
