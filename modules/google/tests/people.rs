//! google-people, the People source the people daemon runs: it looks for
//! google's daemon the moment its socket is made, not once a wait ends.

use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixListener;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// How long google's socket stays missing: past its looks at 1 s and 3 s,
/// so waiting alone it would look next at 7 s.
const MISSING: Duration = Duration::from_millis(3500);

/// How soon after the socket is made it must have looked.
const SOON: Duration = Duration::from_secs(1);

#[test]
fn people_are_watched_for_as_soon_as_googles_socket_is_made() {
    let folder = std::env::temp_dir().join(format!("google-people-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("a folder for google's socket");
    let socket = folder.join("google.sock");
    let mut people = Command::new(env!("CARGO_BIN_EXE_google-people"))
        .arg(&socket)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("google-people starts");
    std::thread::sleep(MISSING);
    let listener = UnixListener::bind(&socket).expect("google's socket is made");
    let made = Instant::now();
    let (heard, watch) = mpsc::channel();
    std::thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept() {
            let mut line = String::new();
            let _ = BufReader::new(stream).read_line(&mut line);
            let _ = heard.send(line);
        }
    });
    let watched = watch.recv_timeout(SOON * 10);
    let after = made.elapsed();
    let _ = people.kill();
    let _ = people.wait();
    let _ = std::fs::remove_dir_all(&folder);
    let watched = watched.expect("google-people never looked for google's daemon");
    assert!(
        watched.contains("\"watch\""),
        "it asks to watch who's named: {watched}"
    );
    assert!(
        after < SOON,
        "it looked {after:?} after google's socket was made"
    );
}
