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

use std::env;
use std::io::{self, Write as _};
use std::path::PathBuf;
use std::process::ExitCode;

use chrome_session::session;

fn main() -> ExitCode {
    let profile = env::args_os().nth(1).map_or_else(
        || {
            PathBuf::from(env::var_os("HOME").unwrap_or_default())
                .join(".config/google-chrome/Default")
        },
        PathBuf::from,
    );
    let read = session::read(&profile).and_then(|open| {
        let mut out = io::stdout().lock();
        open.iter()
            .try_for_each(|tab| writeln!(out, "{}", tab.line()))
            .map_err(|error| error.to_string())
    });
    match read {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("chrome-session: {error}");
            ExitCode::FAILURE
        }
    }
}
