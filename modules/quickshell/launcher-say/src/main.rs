//! What a key says to the launcher: `launcher-say toggle`, or
//! `launcher-say collection <name>`, run by niri on each of the launcher's
//! chords. It writes the words down the socket the shell keeps open
//! (Launcher.qml), ending the line with when it ran, so the shell can time
//! an opening from the chord, and ends. With no shell listening, as while it
//! restarts, `qs ipc call launcher` carries the words instead.
//!
//! It was a bash script running socat. Between chords their files fall out
//! of the page cache on a loaded machine, and reading them back took 0.8 to
//! 1.7 s before the shell heard a chord (bash 255 to 334 ms and socat 546 to
//! 1,403 ms, its eleven libraries with them, against 3 ms each when cached,
//! under 92% disk pressure on 2026-11-10). This is one file, linked
//! statically so no library is read, and `launcher-say-resident` holds its
//! pages in memory (home.nix), so a chord reads nothing from the disk.

use std::env;
use std::io::{self, Write};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// `qs`, by the path the package was built with, or found on PATH.
const QS: &str = match option_env!("LAUNCHER_SAY_QS") {
    Some(path) => path,
    None => "qs",
};

/// The line the shell reads: the words, space apart, then `@` and the wall
/// clock's seconds to the microsecond, as bash's `$EPOCHREALTIME` wrote it.
fn line(words: &[String], at: Duration) -> String {
    format!(
        "{} @{}.{:06}\n",
        words.join(" "),
        at.as_secs(),
        at.subsec_micros()
    )
}

/// Writes the line down the socket, which the shell closes once it's read.
fn say(socket: &Path, said: &str) -> io::Result<()> {
    UnixStream::connect(socket)?.write_all(said.as_bytes())
}

/// The launcher's socket, or none where there's no runtime folder to hold
/// one.
fn socket() -> Option<PathBuf> {
    env::var_os("XDG_RUNTIME_DIR").map(|runtime| PathBuf::from(runtime).join("launcher.sock"))
}

fn main() -> ExitCode {
    let words: Vec<String> = env::args().skip(1).collect();
    let at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    if let Some(socket) = socket()
        && say(&socket, &line(&words, at)).is_ok()
    {
        return ExitCode::SUCCESS;
    }
    // exec only returns when it failed.
    let failed = Command::new(QS)
        .args(["ipc", "call", "launcher"])
        .args(&words)
        .exec();
    eprintln!("launcher-say: neither the launcher's socket nor {QS} answered: {failed}");
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::os::unix::net::UnixListener;

    fn words(said: &[&str]) -> Vec<String> {
        said.iter().map(|word| (*word).to_owned()).collect()
    }

    #[test]
    fn a_line_ends_with_when_it_ran_to_the_microsecond() {
        let said = line(
            &words(&["collection", "on"]),
            Duration::new(1_731_268_512, 636_123_999),
        );
        assert_eq!(
            said, "collection on @1731268512.636123\n",
            "the line read {said:?}"
        );
    }

    #[test]
    fn a_time_under_a_tenth_of_a_second_keeps_its_leading_zeros() {
        let said = line(&words(&["toggle"]), Duration::new(7, 5_000));
        assert_eq!(said, "toggle @7.000005\n", "the line read {said:?}");
    }

    #[test]
    fn the_shell_hears_the_line_said() {
        let folder = env::temp_dir().join(format!("launcher-say-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder for the socket should be made");
        let path = folder.join("launcher.sock");
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).expect("the test's socket should bind");
        say(&path, "toggle @1.000000\n").expect("saying to a listening socket should work");
        let (mut heard, _) = listener.accept().expect("the say should have connected");
        let mut text = String::new();
        heard
            .read_to_string(&mut text)
            .expect("the line should be readable");
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(text, "toggle @1.000000\n", "the shell heard {text:?}");
    }

    #[test]
    fn no_shell_listening_is_an_error_so_qs_carries_it() {
        let path = env::temp_dir().join(format!("launcher-say-none-{}.sock", std::process::id()));
        assert!(
            say(&path, "toggle\n").is_err(),
            "saying to no socket should fail"
        );
    }
}
