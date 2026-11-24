//! A candidate gate's words written as they're said, each line opened by
//! how long the gate had run, so a slow candidate's log shows where its
//! time went: evaluating, waiting for room on a builder, or building. On
//! 24 November one candidate took 26 minutes, and its log, unstamped,
//! couldn't say how much of that it spent waiting.
//!
//! The output and the errors are copied by a thread each, so within one
//! second the log's order across the two streams isn't always the order
//! they were said in; each stream's own order holds, and the words the
//! train's verdict reads all come on the errors.

use std::fs::File;
use std::io::{self, BufRead as _, BufReader, Read, Write as _};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

/// How long the words still coming are waited for once the gate exits.
/// A process it left behind may hold its output open for good, and the
/// train goes on rather than wait for one.
const LAST_WORDS: Duration = Duration::from_secs(30);

/// The longest piece of a line held before it's written: a longer line is
/// written in pieces of this, each stamped, so a build that prints without
/// a newline can't grow the train's memory with it.
const LONGEST_PIECE: u64 = 64 * 1024;

/// Run `command`, its output and its errors written to `log` a line at a
/// time, each opened by how long it had run: how it exited.
///
/// # Errors
///
/// When the log can't be made, the command can't start, or a line couldn't
/// be written, since the train reads its verdict from the log: before the
/// stamps, a full disk failed the gate's own writes and so the gate.
pub(super) fn run(command: Command, log: &Path) -> io::Result<ExitStatus> {
    run_within(command, log, LAST_WORDS)
}

/// The log, shared by the copiers; whether the train has stopped reading
/// it, after which nothing more is written; and the first write that
/// failed.
struct Log {
    file: Mutex<File>,
    closed: AtomicBool,
    lost: Mutex<Option<io::Error>>,
}

impl Log {
    /// Write `said` whole, unless the log is closed; a failed write is
    /// kept, the first only, for the gate's caller.
    fn write(&self, said: &[u8]) {
        let mut file = self.file.lock().unwrap_or_else(PoisonError::into_inner);
        if !self.closed.load(Ordering::Acquire) {
            self.kept(file.write_all(said));
        }
    }

    /// Keep `written`'s error if it's the first.
    fn kept(&self, written: io::Result<()>) {
        if let Err(error) = written {
            let mut lost = self.lost.lock().unwrap_or_else(PoisonError::into_inner);
            if lost.is_none() {
                *lost = Some(error);
            }
        }
    }

    /// Say at `began`'s time that the gate's last words weren't all heard
    /// within `wait`, then write nothing more, so the log holds just the
    /// words the verdict was read from.
    fn close(&self, began: Instant, wait: Duration) {
        let said = format!(
            "{}merge-net: stopped waiting for the gate's last words {} ms after it \
             exited, so what it says later isn't kept\n",
            stamp(began.elapsed().as_secs()),
            wait.as_millis()
        );
        let mut file = self.file.lock().unwrap_or_else(PoisonError::into_inner);
        self.kept(file.write_all(said.as_bytes()));
        self.closed.store(true, Ordering::Release);
    }
}

/// [`run`], waiting at most `wait` for the words still coming once the
/// command exits. Copiers that outlive the wait stay blocked on whatever
/// holds their stream, and end when it closes, writing nothing more.
fn run_within(mut command: Command, log: &Path, wait: Duration) -> io::Result<ExitStatus> {
    let shared = Arc::new(Log {
        file: Mutex::new(File::create(log)?),
        closed: AtomicBool::new(false),
        lost: Mutex::new(None),
    });
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
        let (shared, done) = (Arc::clone(&shared), done.clone());
        thread::spawn(move || {
            copy(from, &shared, began);
            let _ = done.send(());
        });
        copying += 1;
    }
    let status = child.wait();
    let until = Instant::now() + wait;
    let heard_all = (0..copying).all(|_| {
        ended
            .recv_timeout(until.saturating_duration_since(Instant::now()))
            .is_ok()
    });
    if !heard_all {
        shared.close(began, wait);
    }
    let lost = shared
        .lost
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take();
    match lost {
        Some(error) => Err(io::Error::new(
            error.kind(),
            format!("the gate's log lost words: {error}"),
        )),
        None => status,
    }
}

/// Copy `from`'s lines to `to`, each opened by its time since `began`, a
/// line longer than [`LONGEST_PIECE`] in pieces. Each is written whole
/// under the lock, so the output's and the errors' lines never interleave
/// within one. Bytes that aren't UTF-8 pass through as they are.
fn copy(from: impl Read, to: &Log, began: Instant) {
    let mut from = BufReader::new(from);
    let mut piece = Vec::new();
    loop {
        piece.clear();
        match from
            .by_ref()
            .take(LONGEST_PIECE)
            .read_until(b'\n', &mut piece)
        {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        if piece.last() != Some(&b'\n') {
            piece.push(b'\n');
        }
        let mut said = stamp(began.elapsed().as_secs()).into_bytes();
        said.extend_from_slice(&piece);
        to.write(&said);
    }
}

/// How a line said `seconds` into the gate opens: its minutes and seconds.
/// The minutes go unpadded, as the commands' rules ask of a width set by
/// hand, so a gate's lines shift a column only at its 10th minute.
fn stamp(seconds: u64) -> String {
    format!("[{}:{:02}] ", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests;
