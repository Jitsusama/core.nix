//! What a key says to the launcher: `launcher-say toggle`, or
//! `launcher-say collection <name>`, run by niri on each of the launcher's
//! chords. It writes the words down the socket the shell keeps open
//! (Launcher.qml), ending the line with when it ran, so the shell can time
//! an opening from the chord, and ends. With no shell listening, as while it
//! restarts, it waits for one, trying the socket again every few
//! milliseconds, so a chord pressed during a restart opens the launcher once
//! the new shell listens rather than being lost: a switch restarted the
//! shell and Joel's chords went unanswered for over a minute
//! (2026-11-12), since `qs ipc`, which carried them, fails just as the
//! socket does until the new shell has loaded. Only when no shell listens
//! within `WAITING` does `qs ipc call launcher` carry the words instead.
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
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// How long a chord waits for a shell to listen: a restart's new shell
/// listened 1 to 5 s after the old one stopped on Joel's machine
/// (2026-11-12), so this outlasts the slowest with room to spare.
const WAITING: Duration = Duration::from_secs(15);

/// How often it tries the socket again while it waits, so the new shell
/// hears the chord within this of listening.
const TRYING: Duration = Duration::from_millis(10);

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

/// Whether a failed say means no shell listens yet, as while one restarts:
/// the old shell's socket file gone, or left with nothing behind it.
fn not_listening(failed: &io::Error) -> bool {
    matches!(
        failed.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
    )
}

/// Says the line, trying again every `trying` while no shell listens, for
/// as long as `waiting`; it gives the last failure once that has passed, or
/// at once for a failure waiting wouldn't mend.
fn say_waiting(socket: &Path, said: &str, waiting: Duration, trying: Duration) -> io::Result<()> {
    let until = Instant::now() + waiting;
    loop {
        match say(socket, said) {
            Err(failed) if not_listening(&failed) && Instant::now() < until => {
                thread::sleep(trying);
            }
            told => return told,
        }
    }
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
        && say_waiting(&socket, &line(&words, at), WAITING, TRYING).is_ok()
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
    fn a_shell_listening_after_the_chord_still_hears_it() {
        let folder = env::temp_dir().join(format!("launcher-say-late-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder for the socket should be made");
        let path = folder.join("launcher.sock");
        let _ = std::fs::remove_file(&path);
        let listening = path.clone();
        let shell = thread::spawn(move || {
            // The new shell listens a while after the chord, as after a restart.
            thread::sleep(Duration::from_millis(300));
            let listener = UnixListener::bind(&listening).expect("the test's socket should bind");
            let (mut heard, _) = listener.accept().expect("the say should have connected");
            let mut text = String::new();
            heard
                .read_to_string(&mut text)
                .expect("the line should be readable");
            text
        });
        say_waiting(
            &path,
            "toggle @1.000000\n",
            Duration::from_secs(5),
            Duration::from_millis(10),
        )
        .expect("saying should wait for the shell to listen");
        let text = shell.join().expect("the shell should hear the line");
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(text, "toggle @1.000000\n", "the shell heard {text:?}");
    }

    #[test]
    fn a_socket_left_behind_by_a_stopped_shell_is_waited_out() {
        let folder = env::temp_dir().join(format!("launcher-say-left-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder for the socket should be made");
        let path = folder.join("launcher.sock");
        let _ = std::fs::remove_file(&path);
        // A socket file with no listener behind it, as a stopped shell leaves it.
        drop(UnixListener::bind(&path).expect("the test's socket should bind"));
        let failed = say(&path, "toggle\n").expect_err("no shell listens behind the file");
        let _ = std::fs::remove_dir_all(&folder);
        assert!(
            not_listening(&failed),
            "a socket with no listener should be waited on, but failed with {failed}"
        );
    }

    #[test]
    fn no_shell_listening_within_the_wait_is_an_error_so_qs_carries_it() {
        let path = env::temp_dir().join(format!("launcher-say-gone-{}.sock", std::process::id()));
        let started = Instant::now();
        let told = say_waiting(
            &path,
            "toggle\n",
            Duration::from_millis(100),
            Duration::from_millis(10),
        );
        assert!(told.is_err(), "saying to no shell should fail once the wait ends");
        assert!(
            started.elapsed() >= Duration::from_millis(100),
            "it gave up after {:?}, before the wait ended",
            started.elapsed()
        );
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
