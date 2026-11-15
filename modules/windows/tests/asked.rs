//! The daemon asked in a way its unit never would: with no socket, it
//! ends at once in the terminal kit's words, saying nothing on stdout and
//! leaving no socket behind. Started before niri, as a boot's first ask
//! can start it, it waits for niri rather than ending.

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::sync::Mutex;

use daemon_client::until;

/// The daemon as it runs, killed if a test ends before it does, so a
/// daemon that never ends fails its test rather than holding it.
struct Running(Mutex<Option<Child>>);

impl Drop for Running {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.lock().expect("the daemon").take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// What the daemon says and ends with, run with `args` and `niri` as its
/// `NIRI_SOCKET`, or none, once it ends by itself.
fn ran(args: &[&Path], niri: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_windows"));
    command
        .args(args)
        .env_remove("JOURNAL_STREAM")
        .env_remove("LISTEN_FDS")
        .stdin(Stdio::null());
    match niri {
        Some(niri) => command.env("NIRI_SOCKET", niri),
        None => command.env_remove("NIRI_SOCKET"),
    };
    let child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("windows ran");
    let running = Running(Mutex::new(Some(child)));
    until(&format!("{args:?} never ended"), || {
        let mut held = running.0.lock().expect("the daemon");
        held.as_mut()
            .is_some_and(|child| child.try_wait().ok().flatten().is_some())
    });
    let child = running
        .0
        .lock()
        .expect("the daemon")
        .take()
        .expect("the daemon");
    child.wait_with_output().expect("its output")
}

#[test]
fn a_run_with_no_socket_ends_at_once_in_the_kits_words() {
    let folder = std::env::temp_dir().join(format!("windows-asked-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("a folder");
    let niri = folder.join("niri.sock");

    let told = ran(&[], Some(&niri));

    let said = String::from_utf8_lossy(&told.stderr);
    assert_eq!(told.status.code(), Some(64), "it ended so: {said}");
    assert!(told.stdout.is_empty(), "it said on stdout: {told:?}");
    assert!(
        said.contains("windows --help"),
        "it didn't name its help: {said}"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn started_before_niri_it_waits_for_niri_rather_than_ending() {
    let folder = std::env::temp_dir().join(format!("windows-before-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("a folder");
    let socket = folder.join("windows.sock");
    // No PATH, so no user manager is asked and none names a niri: the
    // daemon must go on waiting, and never reach the niri of the machine
    // the test runs on.
    let child = Command::new(env!("CARGO_BIN_EXE_windows"))
        .arg(&socket)
        .env_remove("JOURNAL_STREAM")
        .env_remove("LISTEN_FDS")
        .env_remove("NIRI_SOCKET")
        .env("PATH", "")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("windows ran");
    let running = Running(Mutex::new(Some(child)));
    let said = running
        .0
        .lock()
        .expect("the daemon")
        .as_mut()
        .and_then(|child| child.stderr.take())
        .expect("its stderr");
    let mut first = String::new();
    BufReader::new(said)
        .read_line(&mut first)
        .expect("its first line");

    assert!(
        first.starts_with("<5>niri hasn't named its socket yet"),
        "it said: {first}"
    );
    let ended = running
        .0
        .lock()
        .expect("the daemon")
        .as_mut()
        .and_then(|child| child.try_wait().ok().flatten());
    assert_eq!(ended, None, "it ended rather than waiting for niri");
    assert!(!socket.exists(), "it listened before niri was there");
    drop(running);
    let _ = std::fs::remove_dir_all(&folder);
}
