//! A candidate gate's words written as they're said, each line opened by
//! how long the gate had run, so a slow candidate's log shows where its
//! time went: evaluating, waiting for room on a builder, or building. On
//! 24 November one candidate took 26 minutes, and its log, unstamped,
//! couldn't say how much of that it spent waiting.

use std::fs::File;
use std::io::{self, BufRead as _, BufReader, Read, Write as _};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

/// How long the words still coming are waited for once the gate exits.
/// A process it left behind may hold its output open for good, and the
/// train goes on rather than wait for one.
const LAST_WORDS: Duration = Duration::from_secs(30);

/// Run `command`, its output and its errors written to `log` a line at a
/// time, each opened by how long it had run: how it exited.
pub(super) fn run(mut command: Command, log: &Path) -> io::Result<ExitStatus> {
    let file = Arc::new(Mutex::new(File::create(log)?));
    let began = Instant::now();
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let streams: [Option<Box<dyn Read + Send>>; 2] = [
        child
            .stdout
            .take()
            .map(|out| Box::new(out) as Box<dyn Read + Send>),
        child
            .stderr
            .take()
            .map(|errors| Box::new(errors) as Box<dyn Read + Send>),
    ];
    let (done, ended) = mpsc::channel();
    let mut copying = 0;
    for from in streams.into_iter().flatten() {
        let (file, done) = (Arc::clone(&file), done.clone());
        thread::spawn(move || {
            copy(from, &file, began);
            let _ = done.send(());
        });
        copying += 1;
    }
    let status = child.wait();
    let until = Instant::now() + LAST_WORDS;
    for _ in 0..copying {
        if ended
            .recv_timeout(until.saturating_duration_since(Instant::now()))
            .is_err()
        {
            break;
        }
    }
    status
}

/// Copy `from`'s lines to `to`, each opened by its time since `began`. A
/// line is written whole under the lock, so the output's and the errors'
/// lines never interleave within one.
fn copy(from: impl Read, to: &Mutex<File>, began: Instant) {
    for line in BufReader::new(from).split(b'\n').map_while(Result::ok) {
        let mut said = stamp(began.elapsed().as_secs()).into_bytes();
        said.extend_from_slice(&line);
        said.push(b'\n');
        let mut file = to.lock().unwrap_or_else(PoisonError::into_inner);
        let _ = file.write_all(&said);
    }
}

/// How a line said `seconds` into the gate opens: its minutes and seconds,
/// padded so that a gate's lines align for its first 16 hours.
fn stamp(seconds: u64) -> String {
    format!("[{:>3}:{:02}] ", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stamp_says_minutes_and_seconds_aligned() {
        assert_eq!(stamp(0), "[  0:00] ", "the first second's stamp");
        assert_eq!(
            stamp(1_573),
            "[ 26:13] ",
            "a stamp past the 26 minutes seen"
        );
        assert_eq!(
            stamp(59_999),
            "[999:59] ",
            "the last stamp that keeps its width"
        );
    }

    // Both streams reach the log, every line stamped and whole, and the
    // exit is the command's own, since the train reads a red from it.
    #[test]
    fn every_line_of_both_streams_is_stamped_and_the_exit_kept() {
        let log =
            std::env::temp_dir().join(format!("merge-net-stamped-{}.log", std::process::id()));
        let mut command = Command::new("sh");
        command.args([
            "-c",
            "echo said; echo erred >&2; printf 'no newline'; exit 3",
        ]);
        let status = run(command, &log).unwrap();
        assert_eq!(status.code(), Some(3), "the gate's own exit was lost");
        let words = std::fs::read_to_string(&log).unwrap();
        let _ = std::fs::remove_file(&log);
        let mut lines: Vec<&str> = words.lines().collect();
        lines.sort_unstable();
        assert_eq!(
            lines,
            ["[  0:00] erred", "[  0:00] no newline", "[  0:00] said"],
            "the log wasn't each line stamped once: {words}"
        );
    }
}
