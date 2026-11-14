//! The daemon asked in a way its unit never would: with no socket, or
//! with no niri to hear, it ends at once in the terminal kit's words,
//! saying nothing on stdout and leaving no socket behind.

use std::path::Path;
use std::process::{Command, Output, Stdio};

/// What the daemon says and ends with, run with `args` and `niri` as its
/// `NIRI_SOCKET`, or none.
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
    command.output().expect("windows ran")
}

#[test]
fn a_run_its_unit_wouldnt_make_ends_at_once_in_the_kits_words() {
    let folder = std::env::temp_dir().join(format!("windows-asked-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("a folder");
    let socket = folder.join("windows.sock");
    let niri = folder.join("niri.sock");
    for (args, niri, code, names) in [
        (vec![], Some(niri.as_path()), 64, "windows --help"),
        (vec![socket.as_path()], None, 78, "NIRI_SOCKET"),
    ] {
        let told = ran(&args, niri);
        let said = String::from_utf8_lossy(&told.stderr);
        assert_eq!(told.status.code(), Some(code), "{args:?} ended so: {said}");
        assert!(told.stdout.is_empty(), "{args:?} said on stdout: {told:?}");
        assert!(said.contains(names), "{args:?} didn't name {names}: {said}");
        assert!(!socket.exists(), "{args:?} left a socket behind");
    }
    let _ = std::fs::remove_dir_all(&folder);
}
