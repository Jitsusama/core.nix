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

use calls::{Recording, STARTS_AFTER};
use serde_json::Value;
use std::io::BufReader;
use std::process::{Command, ExitCode, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// How long the recording has to stop before the call counts as over.
const ENDS_AFTER: Duration = Duration::from_secs(5);

/// The start of each Spotify's name on the session bus: its own client's,
/// and spotify-player's.
const SPOTIFY: &str = "org.mpris.MediaPlayer2.spotify";

/// What a call's start or end asks of Spotify.
#[derive(Debug, PartialEq, Eq)]
enum Ask {
    Pause,
    Play,
}

/// Where a call stands, with when it last changed.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Call {
    Off,
    Starting(Instant),
    On,
    Ending(Instant),
}

impl Call {
    /// Moves on from whether anything is recording now, saying what to ask
    /// of Spotify when the call starts or ends.
    fn step(&mut self, recording: bool, now: Instant) -> Option<Ask> {
        let (next, ask) = match (*self, recording) {
            (Call::Off, true) => (Call::Starting(now), None),
            (Call::Starting(_) | Call::Off, false) => (Call::Off, None),
            (Call::Starting(since), true) if now - since >= STARTS_AFTER => {
                (Call::On, Some(Ask::Pause))
            }
            (Call::On, false) => (Call::Ending(now), None),
            (Call::Ending(_) | Call::On, true) => (Call::On, None),
            (Call::Ending(since), false) if now - since >= ENDS_AFTER => {
                (Call::Off, Some(Ask::Play))
            }
            (waiting, _) => (waiting, None),
        };
        *self = next;
        ask
    }

    /// How long until the call starts or ends if nothing changes, for the
    /// wait between pw-dump's arrays.
    fn due(&self, now: Instant) -> Option<Duration> {
        match *self {
            Call::Starting(since) => Some((since + STARTS_AFTER).saturating_duration_since(now)),
            Call::Ending(since) => Some((since + ENDS_AFTER).saturating_duration_since(now)),
            Call::Off | Call::On => None,
        }
    }
}

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
            Some(Ask::Pause) => paused = pause(),
            Some(Ask::Play) => play(&std::mem::take(&mut paused)),
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
    fn a_call_pauses_after_two_seconds_and_plays_five_after_it_ends() {
        let start = Instant::now();
        let at = |secs: f64| start + Duration::from_secs_f64(secs);
        let mut call = Call::Off;
        assert_eq!(call.step(true, at(0.0)), None);
        assert_eq!(call.due(at(0.5)), Some(Duration::from_secs_f64(1.5)));
        assert_eq!(call.step(true, at(1.9)), None);
        assert_eq!(call.step(true, at(2.0)), Some(Ask::Pause));
        assert_eq!(call.step(true, at(60.0)), None, "it pauses once");
        assert_eq!(call.step(false, at(61.0)), None);
        assert_eq!(call.step(false, at(65.9)), None);
        assert_eq!(call.step(false, at(66.0)), Some(Ask::Play));
        assert_eq!(call, Call::Off);
    }

    #[test]
    fn a_moment_of_recording_pauses_nothing() {
        let start = Instant::now();
        let mut call = Call::Off;
        call.step(true, start);
        call.step(false, start + Duration::from_secs(1));
        assert_eq!(call.step(false, start + Duration::from_secs(9)), None);
        assert_eq!(call, Call::Off);
    }

    #[test]
    fn a_call_app_reconnecting_keeps_the_call() {
        let start = Instant::now();
        let at = |secs: u64| start + Duration::from_secs(secs);
        let mut call = Call::Off;
        call.step(true, at(0));
        assert_eq!(call.step(true, at(2)), Some(Ask::Pause));
        call.step(false, at(10));
        assert_eq!(call.step(true, at(13)), None);
        assert_eq!(call.step(false, at(20)), None);
        assert_eq!(call.step(false, at(24)), None);
        assert_eq!(call.step(false, at(25)), Some(Ask::Play));
    }

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
