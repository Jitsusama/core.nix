//! Pauses Spotify while a call app records from a microphone, and plays it
//! again once the call ends, as Joel chose: a call app takes out only its
//! own sound, so music from the speakers would reach the other end.
//!
//! ```text
//! pause-for-calls
//! ```
//!
//! It reads `pw-dump --monitor`, which prints PipeWire's whole graph as a
//! JSON array at the start and then an array of what changed for each
//! change, and keeps which nodes are a program recording from a
//! microphone, as modules/calls counts them. A call is on once one has been
//! running for two seconds, so
//! a browser opening the microphone for a moment pauses nothing, and over
//! once none has for five, so a call app reconnecting doesn't restart the
//! music. At a call's start it pauses each Spotify that's playing, through
//! its MPRIS interface with `busctl`; at its end it plays again only those
//! it paused that are still paused, so music Joel started or stopped
//! himself in the meantime is left as he left it. It ends when PipeWire
//! goes, for systemd to start it again.

use calls::{Call, Change, Recording};
use serde_json::Value;
use std::io::BufReader;
use std::process::{Command, ExitCode, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

/// The start of each Spotify's name on the session bus: its own client's,
/// and spotify-player's.
const SPOTIFY: &str = "org.mpris.MediaPlayer2.spotify";

/// The Spotify names among the session bus's names, a line each with the
/// name first, as `busctl --user list` prints them.
fn spotifies(listed: &str) -> Vec<String> {
    listed
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| name.starts_with(SPOTIFY))
        .map(String::from)
        .collect()
}

/// Whether a player's `PlaybackStatus`, as `busctl get-property` prints it,
/// says it's playing or paused.
fn says(status: &str, state: &str) -> bool {
    status.trim() == format!("s \"{state}\"")
}

fn busctl(args: &[&str]) -> String {
    Command::new("busctl")
        .arg("--user")
        .args(args)
        .stderr(Stdio::null())
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        .unwrap_or_default()
}

const PLAYER: [&str; 2] = ["/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2.Player"];

fn status(name: &str) -> String {
    busctl(&["get-property", name, PLAYER[0], PLAYER[1], "PlaybackStatus"])
}

fn tell(name: &str, method: &str) {
    busctl(&["call", name, PLAYER[0], PLAYER[1], method]);
}

/// Pauses each Spotify that's playing, and gives the names of those paused.
fn pause() -> Vec<String> {
    let paused: Vec<String> = spotifies(&busctl(&["list", "--no-legend"]))
        .into_iter()
        .filter(|name| says(&status(name), "Playing"))
        .collect();
    for name in &paused {
        tell(name, "Pause");
    }
    paused
}

/// Plays again each Spotify paused at the call's start that's still paused.
fn play(paused: &[String]) {
    for name in paused.iter().filter(|name| says(&status(name), "Paused")) {
        tell(name, "Play");
    }
}

fn main() -> ExitCode {
    let Ok(mut dump) = Command::new("pw-dump")
        .arg("--monitor")
        .arg("--no-colors")
        .stdout(Stdio::piped())
        .spawn()
    else {
        eprintln!("pause-for-calls: pw-dump wouldn't start");
        return ExitCode::FAILURE;
    };
    let Some(out) = dump.stdout.take() else {
        return ExitCode::FAILURE;
    };

    // pw-dump's arrays are read on a thread of their own, so the wait for
    // a call to start or end can run out between them.
    let (arrays, heard) = mpsc::channel();
    thread::spawn(move || {
        for array in serde_json::Deserializer::from_reader(BufReader::new(out)).into_iter::<Value>()
        {
            let Ok(array) = array else { break };
            if arrays.send(array).is_err() {
                break;
            }
        }
    });

    let mut recording = Recording::default();
    let mut call = Call::Off;
    let mut paused = Vec::new();
    loop {
        let next = match call.due(Instant::now()) {
            Some(wait) => heard.recv_timeout(wait),
            None => heard
                .recv()
                .map_err(|_| mpsc::RecvTimeoutError::Disconnected),
        };
        match next {
            Ok(array) => recording.take(&array),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        match call.step(recording.any(), Instant::now()) {
            Some(Change::Started) => paused = pause(),
            Some(Change::Ended) => play(&std::mem::take(&mut paused)),
            None => {}
        }
    }
    let _ = dump.wait();
    eprintln!("pause-for-calls: pw-dump ended");
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_spotify_is_found_among_the_bus_names() {
        let listed = "\
:1.4 1201 niri jitsusama :1.4 session-2.scope 2 -
org.mpris.MediaPlayer2.spotify 3410 spotify jitsusama :1.80 app.scope 2 -
org.mpris.MediaPlayer2.spotify_player 3500 spotify_player jitsusama :1.81 app.scope 2 -
org.mpris.MediaPlayer2.chromium.instance3 3600 chrome jitsusama :1.90 app.scope 2 -
";
        assert_eq!(
            spotifies(listed),
            [
                "org.mpris.MediaPlayer2.spotify",
                "org.mpris.MediaPlayer2.spotify_player"
            ]
        );
    }

    #[test]
    fn a_status_says_playing_or_paused() {
        assert!(says("s \"Playing\"\n", "Playing"));
        assert!(!says("s \"Paused\"\n", "Playing"));
        assert!(says("s \"Paused\"\n", "Paused"));
        assert!(!says("", "Paused"), "no answer is neither");
    }
}
